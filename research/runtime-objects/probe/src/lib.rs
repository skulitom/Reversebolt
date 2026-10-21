//! Bounded snapshots of runtime-derived reference graphs. Research only.
#[path = "../../../runtime-probe/probe/src/win.rs"]
pub mod win;
use serde_json::{Value, json};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::{
    collections::HashMap,
    ffi::c_void,
    ptr::null_mut,
    time::{Duration, Instant},
};
use win::Result;
static PAUSED: AtomicBool = AtomicBool::new(false);
static PAUSED_ALLOCS: AtomicU64 = AtomicU64::new(0);
static PAUSED_FREES: AtomicU64 = AtomicU64::new(0);
struct TrackedAllocator;
// Counts this helper's Rust heap operations, not native runtime allocations.
unsafe impl GlobalAlloc for TrackedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if PAUSED.load(Ordering::Relaxed) {
            PAUSED_ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if PAUSED.load(Ordering::Relaxed) {
            PAUSED_FREES.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if PAUSED.load(Ordering::Relaxed) {
            PAUSED_ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
#[global_allocator]
static ALLOCATOR: TrackedAllocator = TrackedAllocator;
use windows_sys::Win32::System::{
    Diagnostics::Debug::ReadProcessMemory,
    LibraryLoader::GetProcAddress,
    Threading::{GetCurrentProcess, GetCurrentProcessId},
};
type Ptr = *mut c_void;
macro_rules! api {
    ($($name:ident: $ty:ty),+ $(,)?) => {
        struct Api { $($name: $ty),+ }
        impl Api { unsafe fn load(module: usize) -> Result<Self> {
            Ok(Self { $($name: unsafe {
                let pointer = GetProcAddress(module as _,concat!(stringify!($name),"\0").as_ptr()).ok_or(stringify!($name))?;
                std::mem::transmute::<unsafe extern "system" fn()->isize,$ty>(pointer)
            }),+ })
        }}
    }
}
api! {
    il2cpp_domain_get: unsafe extern "C" fn()->Ptr,
    il2cpp_domain_get_assemblies: unsafe extern "C" fn(Ptr,*mut usize)->*const Ptr,
    il2cpp_thread_attach: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_thread_detach: unsafe extern "C" fn(Ptr),
    il2cpp_assembly_get_image: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_image_get_name: unsafe extern "C" fn(Ptr)->*const u8,
    il2cpp_image_get_class_count: unsafe extern "C" fn(Ptr)->usize,
    il2cpp_image_get_class: unsafe extern "C" fn(Ptr,usize)->Ptr,
    il2cpp_class_get_name: unsafe extern "C" fn(Ptr)->*const u8,
    il2cpp_class_get_namespace: unsafe extern "C" fn(Ptr)->*const u8,
    il2cpp_class_get_parent: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_class_get_fields: unsafe extern "C" fn(Ptr,*mut Ptr)->Ptr,
    il2cpp_class_is_generic: unsafe extern "C" fn(Ptr)->bool,
    il2cpp_class_is_valuetype: unsafe extern "C" fn(Ptr)->bool,
    il2cpp_class_from_type: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_class_instance_size: unsafe extern "C" fn(Ptr)->i32,
    il2cpp_class_get_data_size: unsafe extern "C" fn(Ptr)->u32,
    il2cpp_class_get_static_field_data: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_field_get_name: unsafe extern "C" fn(Ptr)->*const u8,
    il2cpp_field_get_flags: unsafe extern "C" fn(Ptr)->i32,
    il2cpp_field_get_offset: unsafe extern "C" fn(Ptr)->usize,
    il2cpp_field_get_type: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_type_get_type: unsafe extern "C" fn(Ptr)->i32,
    il2cpp_type_is_byref: unsafe extern "C" fn(Ptr)->bool,
    il2cpp_object_get_class: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_object_header_size: unsafe extern "C" fn()->u32,
    il2cpp_stop_gc_world: unsafe extern "C" fn(),
    il2cpp_start_gc_world: unsafe extern "C" fn(),
}
struct Attached<'a>(&'a Api, Ptr);
impl Drop for Attached<'_> {
    fn drop(&mut self) {
        unsafe { (self.0.il2cpp_thread_detach)(self.1) }
    }
}
struct Stopped(unsafe extern "C" fn());
impl Drop for Stopped {
    fn drop(&mut self) {
        unsafe { (self.0)() }
        PAUSED.store(false, Ordering::Relaxed);
    }
}
#[derive(Clone)]
struct Field {
    name: String,
    offset: usize,
    kind: i32,
    width: usize,
    reference: bool,
    flags: i32,
}
struct Class {
    address: usize,
    name: String,
    namespace: String,
    image: usize,
    parent: usize,
    size: usize,
    fields: Vec<Field>,
    flat: Vec<(usize, usize)>,
}
struct Root {
    class: usize,
    field: usize,
    address: usize,
}
struct Catalog {
    images: Vec<String>,
    classes: Vec<Class>,
    lookup: HashMap<usize, usize>,
    roots: Vec<Root>,
    field_count: usize,
    omissions: Vec<String>,
    header: usize,
}
fn text(p: *const u8) -> Result<String> {
    win::text_pointer(p as usize)
}
fn width(kind: i32) -> usize {
    match kind {
        2 | 4 | 5 => 1,
        3 | 6 | 7 => 2,
        8 | 9 | 12 => 4,
        10 | 11 | 13 => 8,
        _ => 0,
    }
}
fn value(kind: i32, b: [u8; 8]) -> Value {
    match kind {
        2 => json!({"boolean":b[0]!=0,"raw":b[0]}),
        4 => json!(b[0] as i8),
        5 => json!(b[0]),
        3 | 7 => json!(u16::from_le_bytes([b[0], b[1]])),
        6 => json!(i16::from_le_bytes([b[0], b[1]])),
        8 => json!(i32::from_le_bytes(b[..4].try_into().unwrap())),
        9 => json!(u32::from_le_bytes(b[..4].try_into().unwrap())),
        10 => json!(i64::from_le_bytes(b).to_string()),
        11 => json!(u64::from_le_bytes(b).to_string()),
        12 => json!(f32::from_le_bytes(b[..4].try_into().unwrap()).to_string()),
        13 => json!(f64::from_le_bytes(b).to_string()),
        _ => Value::Null,
    }
}
fn catalog(api: &Api) -> Result<Catalog> {
    let start = Instant::now();
    let mut c = Catalog {
        images: Vec::new(),
        classes: Vec::new(),
        lookup: HashMap::new(),
        roots: Vec::new(),
        field_count: 0,
        omissions: Vec::new(),
        header: unsafe { (api.il2cpp_object_header_size)() } as usize,
    };
    if !(8..=128).contains(&c.header) {
        return Err("unsupported runtime header size".into());
    }
    unsafe {
        let mut count = 0;
        let list = (api.il2cpp_domain_get_assemblies)((api.il2cpp_domain_get)(), &mut count);
        if count == 0 || count > 256 {
            return Err("assembly budget".into());
        }
        let bytes = win::self_read(list as usize, count * 8)?;
        for (image_id, bytes) in bytes.as_chunks::<8>().0.iter().enumerate() {
            let image = (api.il2cpp_assembly_get_image)(usize::from_le_bytes(*bytes) as Ptr);
            if image.is_null() {
                return Err("null runtime image".into());
            }
            c.images.push(text((api.il2cpp_image_get_name)(image))?);
            let count = (api.il2cpp_image_get_class_count)(image);
            if count > 50_000 {
                return Err("image class budget".into());
            }
            for index in 0..count {
                if c.classes.len() >= 50_000
                    || c.field_count >= 200_000
                    || start.elapsed() > Duration::from_secs(20)
                {
                    return Err("catalog budget exceeded; no pause performed".into());
                }
                let class = (api.il2cpp_image_get_class)(image, index);
                if class.is_null() {
                    continue;
                }
                if (api.il2cpp_class_is_generic)(class) {
                    c.omissions.push("open_generic_class".into());
                    continue;
                }
                let class_id = c.classes.len();
                let mut entry = Class {
                    address: class as usize,
                    name: text((api.il2cpp_class_get_name)(class))?,
                    namespace: text((api.il2cpp_class_get_namespace)(class))?,
                    image: image_id,
                    parent: (api.il2cpp_class_get_parent)(class) as usize,
                    size: (api.il2cpp_class_instance_size)(class).max(0) as usize,
                    fields: Vec::new(),
                    flat: Vec::new(),
                };
                let mut iter = null_mut();
                loop {
                    if c.field_count >= 200_000 || start.elapsed() > Duration::from_secs(20) {
                        return Err("field budget exceeded; no pause performed".into());
                    }
                    let field = (api.il2cpp_class_get_fields)(class, &mut iter);
                    if field.is_null() {
                        break;
                    }
                    c.field_count += 1;
                    let typ = (api.il2cpp_field_get_type)(field);
                    if typ.is_null() {
                        continue;
                    }
                    let kind = (api.il2cpp_type_get_type)(typ);
                    let flags = (api.il2cpp_field_get_flags)(field);
                    let byref = (api.il2cpp_type_is_byref)(typ);
                    let reference = !byref
                        && match kind {
                            0x12 | 0x1c => true,
                            0x15 => {
                                let k = (api.il2cpp_class_from_type)(typ);
                                !k.is_null() && !(api.il2cpp_class_is_valuetype)(k)
                            }
                            _ => false,
                        };
                    let width = if reference {
                        8
                    } else if byref {
                        0
                    } else {
                        width(kind)
                    };
                    let f = Field {
                        name: text((api.il2cpp_field_get_name)(field))?,
                        offset: (api.il2cpp_field_get_offset)(field),
                        kind,
                        width,
                        reference,
                        flags,
                    };
                    if reference && flags & 0x10 != 0 && flags & 0x40 == 0 {
                        let storage = (api.il2cpp_class_get_static_field_data)(class) as usize;
                        let size = (api.il2cpp_class_get_data_size)(class) as usize;
                        if storage != 0
                            && f.offset.checked_add(8).is_some_and(|n| n <= size)
                            && let Some(address) = storage.checked_add(f.offset)
                        {
                            if c.roots.len() >= 10_000 {
                                return Err("root budget exceeded; no pause performed".into());
                            }
                            c.roots.push(Root {
                                class: class_id,
                                field: entry.fields.len(),
                                address,
                            });
                        }
                    }
                    entry.fields.push(f);
                }
                c.lookup.insert(entry.address, class_id);
                c.classes.push(entry);
            }
        }
    }
    // Flatten inheritance before stopping the runtime; lookup never creates metadata while paused.
    for index in 0..c.classes.len() {
        let mut current = index;
        let mut seen = Vec::new();
        let mut flat = Vec::new();
        loop {
            if seen.contains(&current) || seen.len() >= 64 {
                c.omissions.push("parent_chain_limit".into());
                break;
            }
            seen.push(current);
            for (field, f) in c.classes[current].fields.iter().enumerate() {
                if f.flags & 0x10 == 0
                    && f.width > 0
                    && f.offset >= c.header
                    && f.offset
                        .checked_add(f.width)
                        .is_some_and(|end| end <= c.classes[index].size)
                {
                    flat.push((current, field));
                }
            }
            let parent = c.classes[current].parent;
            if parent == 0 {
                break;
            }
            match c.lookup.get(&parent) {
                Some(&id) => current = id,
                None => {
                    c.omissions.push("uncatalogued_parent".into());
                    break;
                }
            }
        }
        c.classes[index].flat = flat;
    }
    Ok(c)
}
const MAX_NODES: usize = 1024;
const MAX_READS: usize = 16384;
const PAUSE_BUDGET: Duration = Duration::from_millis(10);
#[derive(Clone, Copy, Default)]
struct Node {
    address: usize,
    class: usize,
    depth: usize,
}
#[derive(Clone, Copy, Default)]
struct Read {
    node: usize,
    class: usize,
    field: usize,
    bytes: [u8; 8],
    ok: bool,
    child: usize,
}
#[derive(Clone, Copy, Default)]
struct RootRead {
    root: usize,
    address: usize,
    node: usize,
    ok: bool,
}
// No formatting, allocation, metadata enumeration, logging or handle creation in this region.
fn read_into(address: usize, buffer: &mut [u8]) -> bool {
    let mut count = 0;
    address != 0
        && unsafe {
            ReadProcessMemory(
                GetCurrentProcess(),
                address as *const c_void,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut count,
            ) != 0
        }
        && count == buffer.len()
}
fn node_for(address: usize, depth: usize, nodes: &mut Vec<Node>, api: &Api, c: &Catalog) -> usize {
    if address == 0 {
        return usize::MAX;
    }
    if let Some(i) = nodes.iter().position(|n| n.address == address) {
        return i;
    }
    if nodes.len() == MAX_NODES {
        return usize::MAX - 1;
    }
    // The pointer came from a reference-typed field while managed threads are stopped.
    // Check accessibility before the trivial runtime object-class accessor.
    let mut header = [0u8; 128];
    if !read_into(address, &mut header[..c.header]) {
        return usize::MAX - 2;
    }
    let klass = unsafe { (api.il2cpp_object_get_class)(address as Ptr) } as usize;
    let class = c.lookup.get(&klass).copied().unwrap_or(usize::MAX);
    nodes.push(Node {
        address,
        class,
        depth,
    });
    nodes.len() - 1
}
fn snapshot(api: &Api, c: &Catalog) -> Value {
    let mut nodes = Vec::with_capacity(MAX_NODES);
    let mut reads = Vec::with_capacity(MAX_READS);
    let mut roots = Vec::with_capacity(c.roots.len());
    // Touch reserved storage and warm kernel read/clock paths before runtime suspension.
    nodes.resize(MAX_NODES, Node::default());
    nodes.clear();
    reads.resize(MAX_READS, Read::default());
    reads.clear();
    roots.resize(c.roots.len(), RootRead::default());
    roots.clear();
    let mut warm = [0; 8];
    let _ = read_into(c as *const Catalog as usize, &mut warm);
    let before = Instant::now();
    let allocations_before = PAUSED_ALLOCS.load(Ordering::Relaxed);
    let frees_before = PAUSED_FREES.load(Ordering::Relaxed);
    unsafe { (api.il2cpp_stop_gc_world)() };
    let guard = Stopped(api.il2cpp_start_gc_world);
    PAUSED.store(true, Ordering::Relaxed);
    let stopped = Instant::now();
    let mut budget = false;
    for (index, root) in c.roots.iter().enumerate() {
        if stopped.elapsed() >= PAUSE_BUDGET {
            budget = true;
            break;
        }
        let mut bytes = [0; 8];
        let ok = read_into(root.address, &mut bytes);
        let address = if ok { usize::from_le_bytes(bytes) } else { 0 };
        let node = if ok {
            node_for(address, 0, &mut nodes, api, c)
        } else {
            usize::MAX - 2
        };
        roots.push(RootRead {
            root: index,
            address,
            node,
            ok,
        });
    }
    let mut cursor = 0;
    'objects: while cursor < nodes.len() {
        if stopped.elapsed() >= PAUSE_BUDGET {
            budget = true;
            break;
        }
        let n = nodes[cursor];
        if n.class != usize::MAX {
            for &(class, field) in &c.classes[n.class].flat {
                if reads.len() == MAX_READS || stopped.elapsed() >= PAUSE_BUDGET {
                    budget = true;
                    break 'objects;
                }
                let f = &c.classes[class].fields[field];
                let mut bytes = [0; 8];
                let ok = n
                    .address
                    .checked_add(f.offset)
                    .is_some_and(|a| read_into(a, &mut bytes[..f.width]));
                let child = if f.reference && ok {
                    if n.depth < 4 {
                        node_for(usize::from_le_bytes(bytes), n.depth + 1, &mut nodes, api, c)
                    } else if usize::from_le_bytes(bytes) != 0 {
                        usize::MAX - 3
                    } else {
                        usize::MAX
                    }
                } else {
                    usize::MAX
                };
                reads.push(Read {
                    node: cursor,
                    class,
                    field,
                    bytes,
                    ok,
                    child,
                });
            }
        }
        cursor += 1;
    }
    let held_us = stopped.elapsed().as_micros() as u64;
    drop(guard); // Always resume before allocation, serialization or error reporting.
    let total_us = before.elapsed().as_micros() as u64;
    let nodes_json:Vec<_>=nodes.iter().enumerate().map(|(id,n)|json!({"id":id,"address":format!("0x{:x}",n.address),"class":if n.class==usize::MAX{Value::Null}else{json!(n.class)},"depth":n.depth,"status":if n.class==usize::MAX{"uncatalogued_runtime_type"}else{"catalogued"}})).collect();
    let roots_json:Vec<_>=roots.iter().map(|r|{let root=&c.roots[r.root];json!({"root":r.root,"class":root.class,"field":root.field,"reference":format!("0x{:x}",r.address),"node":node_json(r.node),"read_ok":r.ok})}).collect();
    let reads_json:Vec<_>=reads.iter().map(|r|{let f=&c.classes[r.class].fields[r.field];json!({"node":r.node,"declaring_class":r.class,"field":r.field,"read_ok":r.ok,"bytes":&r.bytes[..f.width],"value":if !r.ok||f.reference{Value::Null}else{value(f.kind,r.bytes)},"reference":if f.reference{node_json(r.child)}else{Value::Null}})}).collect();
    json!({"pause_held_us":held_us,"stop_to_resume_us":total_us,"helper_allocations_while_paused":PAUSED_ALLOCS.load(Ordering::Relaxed)-allocations_before,"helper_deallocations_while_paused":PAUSED_FREES.load(Ordering::Relaxed)-frees_before,"time_or_read_budget_reached":budget,"node_capacity_reached":nodes.len()==MAX_NODES,"processed_nodes":cursor,"nodes":nodes_json,"roots":roots_json,"reads":reads_json})
}
fn node_json(index: usize) -> Value {
    match index {
        usize::MAX => json!({"status":"null_or_not_followed"}),
        n if n == usize::MAX - 1 => json!({"status":"node_budget"}),
        n if n == usize::MAX - 2 => json!({"status":"unreadable"}),
        n if n == usize::MAX - 3 => json!({"status":"depth_budget"}),
        n => json!({"id":n}),
    }
}
fn inspect() -> Result<Value> {
    let pid = unsafe { GetCurrentProcessId() };
    let matches: Vec<_> = win::modules(pid)?
        .into_iter()
        .filter_map(|m| unsafe { Api::load(m.base).ok().map(|a| (m, a)) })
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "expected one complete runtime API provider; got {}",
            matches.len()
        ));
    }
    let (module, api) = matches.into_iter().next().unwrap();
    let domain = unsafe { (api.il2cpp_domain_get)() };
    if domain.is_null() {
        return Err("domain unavailable".into());
    }
    let thread = unsafe { (api.il2cpp_thread_attach)(domain) };
    if thread.is_null() {
        return Err("attach failed".into());
    }
    let _attached = Attached(&api, thread);
    let start = Instant::now();
    let c = catalog(&api)?;
    let catalog_ms = start.elapsed().as_millis() as u64;
    let first = snapshot(&api, &c);
    std::thread::sleep(Duration::from_millis(250));
    let second = snapshot(&api, &c);
    let mut used = std::collections::BTreeSet::new();
    for shot in [&first, &second] {
        for r in shot["roots"].as_array().unwrap() {
            used.insert(r["class"].as_u64().unwrap() as usize);
        }
        for n in shot["nodes"].as_array().unwrap() {
            if let Some(id) = n["class"].as_u64() {
                used.insert(id as usize);
            }
        }
        for r in shot["reads"].as_array().unwrap() {
            used.insert(r["declaring_class"].as_u64().unwrap() as usize);
        }
    }
    let schema:Vec<_>=used.into_iter().map(|id|{let k=&c.classes[id];json!({"id":id,"name":k.name,"namespace":k.namespace,"image":c.images[k.image],"instance_size":k.size,"fields":k.fields.iter().enumerate().map(|(i,f)|json!({"id":i,"name":f.name,"offset":f.offset,"type_code":f.kind,"reference":f.reference,"flags":f.flags})).collect::<Vec<_>>()})}).collect();
    Ok(
        json!({"schema":2,"pid":pid,"created":win::creation(unsafe{GetCurrentProcess()})?,"session":win::session(pid)?,"runtime":module.path,"catalog_ms":catalog_ms,"catalog_classes":c.classes.len(),"catalog_fields":c.field_count,"catalog_roots":c.roots.len(),"catalog_omissions":c.omissions,"classes":schema,"snapshots":[first,second],"edit_authority":false,"identity_contract":"object IDs belong only to one paused snapshot; addresses may be reused; no retained object handles","instrumentation":"runtime thread attachment; metadata preparation; bounded managed-world pause with preallocated reads; no gameplay field writes"}),
    )
}
/// # Safety
/// argument is a NUL-terminated UTF-16 absolute output path in the current process.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn ReverseboltInspect(argument: Ptr) -> u32 {
    let result = std::panic::catch_unwind(|| -> Result<()> {
        let mut path = Vec::new();
        for index in 0..32768 {
            let b = win::self_read(
                (argument as usize)
                    .checked_add(index * 2)
                    .ok_or("path overflow")?,
                2,
            )?;
            let ch = u16::from_le_bytes([b[0], b[1]]);
            if ch == 0 {
                break;
            }
            path.push(ch);
        }
        if path.is_empty() || path.len() == 32768 {
            return Err("path length".into());
        }
        let path = String::from_utf16(&path).map_err(|_| "path encoding")?;
        if !std::path::Path::new(&path).is_absolute() {
            return Err("absolute path required".into());
        }
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let report = inspect().unwrap_or_else(|e| json!({"error":e}));
        serde_json::to_writer(file, &report).map_err(|e| e.to_string())?;
        Ok(())
    });
    match result {
        Ok(Ok(())) => 0,
        Ok(Err(_)) => 2,
        Err(_) => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static RESTARTS: AtomicU64 = AtomicU64::new(0);
    unsafe extern "C" fn restart() {
        RESTARTS.fetch_add(1, Ordering::Relaxed);
    }
    #[test]
    fn pause_guard_resumes_on_early_error() {
        fn fail() -> Result<()> {
            let _guard = Stopped(restart);
            Err("test".into())
        }
        let before = RESTARTS.load(Ordering::Relaxed);
        assert!(fail().is_err());
        assert_eq!(RESTARTS.load(Ordering::Relaxed), before + 1);
    }
}
