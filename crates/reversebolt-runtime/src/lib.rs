//! Runtime-derived graph discovery. Not yet wired to automatic desktop attachment.
mod paths;
pub mod win;
use reversebolt_core::runtime::{Locator, Scope, TypeKey};
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
    il2cpp_class_get_image: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_class_get_type: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_class_get_rank: unsafe extern "C" fn(Ptr)->i32,
    il2cpp_class_get_element_class: unsafe extern "C" fn(Ptr)->Ptr,
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
    il2cpp_type_get_assembly_qualified_name: unsafe extern "C" fn(Ptr)->*mut u8,
    il2cpp_free: unsafe extern "C" fn(Ptr),
    il2cpp_type_is_byref: unsafe extern "C" fn(Ptr)->bool,
    il2cpp_object_get_class: unsafe extern "C" fn(Ptr)->Ptr,
    il2cpp_object_header_size: unsafe extern "C" fn()->u32,
    il2cpp_array_object_header_size: unsafe extern "C" fn()->u32,
    il2cpp_array_length: unsafe extern "C" fn(Ptr)->u32,
    il2cpp_array_element_size: unsafe extern "C" fn(Ptr)->i32,
    il2cpp_string_length: unsafe extern "C" fn(Ptr)->i32,
    il2cpp_string_chars: unsafe extern "C" fn(Ptr)->*const u16,
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
    key: TypeKey,
    address: usize,
    name: String,
    namespace: String,
    image: usize,
    parent: usize,
    size: usize,
    fields: Vec<Field>,
    flat: Vec<(usize, usize)>,
    shape: Shape,
}
#[derive(Clone, Copy)]
enum Shape {
    Object,
    String,
    Array {
        stride: usize,
        kind: i32,
        reference: bool,
    },
    UnsupportedArray,
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
    array_header: usize,
}
fn text(p: *const u8) -> Result<String> {
    win::text_pointer(p as usize)
}
fn qualified_name(api: &Api, typ: Ptr) -> Result<String> {
    let pointer = unsafe { (api.il2cpp_type_get_assembly_qualified_name)(typ) };
    if pointer.is_null() {
        return Err("runtime type name unavailable".into());
    }
    let result = text(pointer);
    unsafe { (api.il2cpp_free)(pointer.cast()) };
    result
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
fn reference_kind(api: &Api, typ: Ptr, kind: i32) -> bool {
    unsafe {
        match kind {
            0x0e | 0x12 | 0x14 | 0x1c | 0x1d => true,
            0x15 => {
                let k = (api.il2cpp_class_from_type)(typ);
                !k.is_null() && !(api.il2cpp_class_is_valuetype)(k)
            }
            _ => false,
        }
    }
}
fn load_class(api: &Api, c: &mut Catalog, class: Ptr, roots: bool, start: Instant) -> Result<()> {
    if class.is_null() || c.lookup.contains_key(&(class as usize)) {
        return Ok(());
    }
    if c.classes.len() >= 50_000
        || c.field_count >= 200_000
        || start.elapsed() > Duration::from_secs(20)
    {
        return Err("metadata budget".into());
    }
    unsafe {
        if (api.il2cpp_class_is_generic)(class) {
            c.omissions.push("open_generic_class".into());
            return Ok(());
        }
        let image = (api.il2cpp_class_get_image)(class);
        if image.is_null() {
            return Err("null class image".into());
        }
        let image_name = text((api.il2cpp_image_get_name)(image))?;
        let image_id = if let Some(id) = c.images.iter().position(|n| *n == image_name) {
            id
        } else {
            c.images.push(image_name.clone());
            c.images.len() - 1
        };
        let kind = (api.il2cpp_type_get_type)((api.il2cpp_class_get_type)(class));
        let rank = (api.il2cpp_class_get_rank)(class);
        let shape = if kind == 0x0e {
            Shape::String
        } else if rank > 0 {
            let element = (api.il2cpp_class_get_element_class)(class);
            if element.is_null() {
                Shape::UnsupportedArray
            } else {
                let typ = (api.il2cpp_class_get_type)(element);
                let kind = (api.il2cpp_type_get_type)(typ);
                let reference = !(api.il2cpp_class_is_valuetype)(element);
                let stride = (api.il2cpp_array_element_size)(class).max(0) as usize;
                if (reference && stride == 8)
                    || (!reference && width(kind) > 0 && stride == width(kind))
                {
                    Shape::Array {
                        stride,
                        kind,
                        reference,
                    }
                } else {
                    Shape::UnsupportedArray
                }
            }
        } else {
            Shape::Object
        };
        let class_id = c.classes.len();
        let mut entry = Class {
            key: TypeKey {
                image: image_name,
                type_name: qualified_name(api, (api.il2cpp_class_get_type)(class))?,
                layout_sha256: String::new(),
            },
            address: class as usize,
            name: text((api.il2cpp_class_get_name)(class))?,
            namespace: text((api.il2cpp_class_get_namespace)(class))?,
            image: image_id,
            parent: (api.il2cpp_class_get_parent)(class) as usize,
            size: (api.il2cpp_class_instance_size)(class).max(0) as usize,
            fields: Vec::new(),
            flat: Vec::new(),
            shape,
        };
        let mut iter = null_mut();
        loop {
            if c.field_count >= 200_000 || start.elapsed() > Duration::from_secs(20) {
                return Err("field metadata budget".into());
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
            let reference = !byref && reference_kind(api, typ, kind);
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
            if roots && reference && flags & 0x10 != 0 && flags & 0x40 == 0 {
                let storage = (api.il2cpp_class_get_static_field_data)(class) as usize;
                let size = (api.il2cpp_class_get_data_size)(class) as usize;
                if storage != 0
                    && f.offset.checked_add(8).is_some_and(|end| end <= size)
                    && let Some(address) = storage.checked_add(f.offset)
                {
                    if c.roots.len() >= 10_000 {
                        return Err("static root budget".into());
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
        use sha2::{Digest, Sha256};
        let shape = match entry.shape {
            Shape::Object => json!(["object"]),
            Shape::String => json!(["string"]),
            Shape::Array {
                stride,
                kind,
                reference,
            } => json!(["array", stride, kind, reference]),
            Shape::UnsupportedArray => json!(["unsupported_array"]),
        };
        let fields: Vec<_> = entry
            .fields
            .iter()
            .map(|f| json!([f.name, f.offset, f.kind, f.width, f.reference, f.flags]))
            .collect();
        entry.key.layout_sha256 = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&json!([entry.size, shape, fields]))
                    .map_err(|e| e.to_string())?
            )
        );
        entry.key.validate()?;
        c.lookup.insert(entry.address, class_id);
        c.classes.push(entry);
    }
    Ok(())
}
fn flatten(c: &mut Catalog) {
    c.omissions
        .retain(|r| r != "uncatalogued_parent" && r != "parent_chain_limit");
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
}
fn catalog(api: &Api, start: Instant) -> Result<Catalog> {
    let mut c = Catalog {
        images: Vec::new(),
        classes: Vec::new(),
        lookup: HashMap::new(),
        roots: Vec::new(),
        field_count: 0,
        omissions: Vec::new(),
        header: unsafe { (api.il2cpp_object_header_size)() } as usize,
        array_header: unsafe { (api.il2cpp_array_object_header_size)() } as usize,
    };
    if !(8..=128).contains(&c.header) || !(c.header..=256).contains(&c.array_header) {
        return Err("unsupported runtime header sizes".into());
    }
    unsafe {
        let mut count = 0;
        let list = (api.il2cpp_domain_get_assemblies)((api.il2cpp_domain_get)(), &mut count);
        if count == 0 || count > 256 {
            return Err("assembly budget".into());
        }
        let bytes = win::self_read(list as usize, count * 8)?;
        for bytes in bytes.as_chunks::<8>().0 {
            let image = (api.il2cpp_assembly_get_image)(usize::from_le_bytes(*bytes) as Ptr);
            if image.is_null() {
                return Err("null image".into());
            }
            let count = (api.il2cpp_image_get_class_count)(image);
            if count > 50_000 {
                return Err("image class budget".into());
            }
            for index in 0..count {
                load_class(
                    api,
                    &mut c,
                    (api.il2cpp_image_get_class)(image, index),
                    true,
                    start,
                )?;
            }
        }
    }
    flatten(&mut c);
    Ok(c)
}
fn prepare_encountered(
    api: &Api,
    c: &mut Catalog,
    classes: &[usize],
    start: Instant,
) -> Result<usize> {
    let before = c.classes.len();
    // Class metadata has runtime/domain lifetime. These are class pointers captured
    // from valid objects, never object pointers dereferenced after resumption.
    let mut pending = classes.to_vec();
    let mut seen = std::collections::HashSet::new();
    while let Some(address) = pending.pop() {
        if address == 0 || !seen.insert(address) || seen.len() > 2048 {
            continue;
        }
        load_class(api, c, address as Ptr, false, start)?;
        if let Some(&id) = c.lookup.get(&address) {
            let parent = c.classes[id].parent;
            if parent != 0 && !c.lookup.contains_key(&parent) {
                pending.push(parent);
            }
        }
    }
    if c.classes.len() != before {
        flatten(c)
    }
    Ok(c.classes.len() - before)
}
const MAX_NODES: usize = 1024;
const MAX_READS: usize = 16384;
const ROOT_BATCH: usize = 64;
const ARRAY_LIMIT: usize = 64;
const TEXT_LIMIT: usize = 128;
const PAUSE_BUDGET: Duration = Duration::from_millis(5);
#[derive(Clone, Copy, Default)]
struct Node {
    address: usize,
    class: usize,
    runtime_class: usize,
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
    element: bool,
    kind: i32,
    width: usize,
    reference: bool,
}
#[derive(Clone, Copy, Default)]
struct RootRead {
    root: usize,
    address: usize,
    node: usize,
    ok: bool,
}
#[derive(Clone, Copy)]
struct TextRead {
    node: usize,
    length: i32,
    count: usize,
    bytes: [u8; TEXT_LIMIT * 2],
    ok: bool,
}
impl Default for TextRead {
    fn default() -> Self {
        Self {
            node: 0,
            length: 0,
            count: 0,
            bytes: [0; TEXT_LIMIT * 2],
            ok: false,
        }
    }
}
#[derive(Clone, Copy, Default)]
struct ArrayRead {
    node: usize,
    length: usize,
    attempted: usize,
    supported: bool,
}
struct Capture {
    value: Value,
    unknown: Vec<usize>,
}
// No allocations, formatting, metadata enumeration, logging or GC handles while paused.
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
    let mut header = [0; 128];
    if !read_into(address, &mut header[..c.header]) {
        return usize::MAX - 2;
    }
    let runtime_class = unsafe { (api.il2cpp_object_get_class)(address as Ptr) } as usize;
    let class = c.lookup.get(&runtime_class).copied().unwrap_or(usize::MAX);
    nodes.push(Node {
        address,
        class,
        runtime_class,
        depth,
    });
    nodes.len() - 1
}
fn edge(address: usize, depth: usize, nodes: &mut Vec<Node>, api: &Api, c: &Catalog) -> usize {
    if address != 0 && depth >= 4 {
        usize::MAX - 3
    } else {
        node_for(address, depth + 1, nodes, api, c)
    }
}
fn snapshot(api: &Api, c: &Catalog, root_start: usize, root_end: usize) -> Capture {
    let mut nodes = vec![Node::default(); MAX_NODES];
    nodes.clear();
    let mut reads = vec![Read::default(); MAX_READS];
    reads.clear();
    let mut roots = vec![RootRead::default(); root_end - root_start];
    roots.clear();
    let mut texts = vec![TextRead::default(); MAX_NODES];
    texts.clear();
    let mut arrays = vec![ArrayRead::default(); MAX_NODES];
    arrays.clear();
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
    for (index, root) in c.roots.iter().enumerate().take(root_end).skip(root_start) {
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
            match c.classes[n.class].shape {
                Shape::String => {
                    let length = unsafe { (api.il2cpp_string_length)(n.address as Ptr) };
                    let count = length.max(0).min(TEXT_LIMIT as i32) as usize;
                    let mut text = TextRead {
                        node: cursor,
                        length,
                        count,
                        ..TextRead::default()
                    };
                    let address = unsafe { (api.il2cpp_string_chars)(n.address as Ptr) } as usize;
                    text.ok = length >= 0
                        && (count == 0 || read_into(address, &mut text.bytes[..count * 2]));
                    texts.push(text);
                }
                Shape::UnsupportedArray => {
                    arrays.push(ArrayRead {
                        node: cursor,
                        length: unsafe { (api.il2cpp_array_length)(n.address as Ptr) } as usize,
                        attempted: 0,
                        supported: false,
                    });
                }
                Shape::Array {
                    stride,
                    kind,
                    reference,
                } => {
                    let length = unsafe { (api.il2cpp_array_length)(n.address as Ptr) } as usize;
                    let mut attempted = 0;
                    for index in 0..length.min(ARRAY_LIMIT) {
                        if reads.len() == MAX_READS || stopped.elapsed() >= PAUSE_BUDGET {
                            budget = true;
                            break;
                        }
                        let mut bytes = [0; 8];
                        let ok = index
                            .checked_mul(stride)
                            .and_then(|v| v.checked_add(c.array_header))
                            .and_then(|v| n.address.checked_add(v))
                            .is_some_and(|a| read_into(a, &mut bytes[..stride]));
                        let child = if reference && ok {
                            edge(usize::from_le_bytes(bytes), n.depth, &mut nodes, api, c)
                        } else {
                            usize::MAX
                        };
                        reads.push(Read {
                            node: cursor,
                            class: n.class,
                            field: index,
                            bytes,
                            ok,
                            child,
                            element: true,
                            kind,
                            width: stride,
                            reference,
                        });
                        attempted += 1;
                    }
                    arrays.push(ArrayRead {
                        node: cursor,
                        length,
                        attempted,
                        supported: true,
                    });
                    if budget {
                        break 'objects;
                    }
                }
                Shape::Object => {
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
                            edge(usize::from_le_bytes(bytes), n.depth, &mut nodes, api, c)
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
                            element: false,
                            kind: f.kind,
                            width: f.width,
                            reference: f.reference,
                        });
                    }
                }
            }
        }
        cursor += 1;
    }
    let held_us = stopped.elapsed().as_micros() as u64;
    drop(guard);
    let total_us = before.elapsed().as_micros() as u64;
    let mut unknown: Vec<_> = nodes
        .iter()
        .filter(|n| n.class == usize::MAX)
        .map(|n| n.runtime_class)
        .collect();
    unknown.sort_unstable();
    unknown.dedup();
    let nodes_json:Vec<_>=nodes.iter().enumerate().map(|(id,n)|json!({"id":id,"address":format!("0x{:x}",n.address),"class":if n.class==usize::MAX{Value::Null}else{json!(n.class)},"depth":n.depth,"status":if n.class==usize::MAX{"uncatalogued_runtime_type"}else{"catalogued"}})).collect();
    let roots_json:Vec<_>=roots.iter().map(|r|{let root=&c.roots[r.root];json!({"root":r.root,"class":root.class,"field":root.field,"reference":format!("0x{:x}",r.address),"node":node_json(r.node),"read_ok":r.ok})}).collect();
    let reads_json:Vec<_>=reads.iter().map(|r|json!({"node":r.node,"declaring_class":r.class,"field":if r.element{Value::Null}else{json!(r.field)},"element_index":if r.element{json!(r.field)}else{Value::Null},"type_code":r.kind,"read_ok":r.ok,"bytes":&r.bytes[..r.width],"value":if !r.ok||r.reference{Value::Null}else{value(r.kind,r.bytes)},"reference":if r.reference{node_json(r.child)}else{Value::Null}})).collect();
    let strings_json:Vec<_>=texts.iter().map(|t|{let units:Vec<_>=t.bytes[..t.count*2].as_chunks::<2>().0.iter().map(|b|u16::from_le_bytes(*b)).collect();json!({"node":t.node,"length":t.length,"read_ok":t.ok,"truncated":t.length>TEXT_LIMIT as i32,"text":if t.ok{json!(String::from_utf16_lossy(&units))}else{Value::Null}})}).collect();
    let arrays_json:Vec<_>=arrays.iter().map(|a|json!({"node":a.node,"length":a.length,"elements_read":a.attempted,"supported":a.supported,"truncated":a.attempted<a.length})).collect();
    let value = json!({"root_start":root_start,"root_end":root_end,"pause_held_us":held_us,"stop_to_resume_us":total_us,"helper_allocations_while_paused":PAUSED_ALLOCS.load(Ordering::Relaxed)-allocations_before,"helper_deallocations_while_paused":PAUSED_FREES.load(Ordering::Relaxed)-frees_before,"time_or_read_budget_reached":budget,"node_capacity_reached":nodes.len()==MAX_NODES,"processed_nodes":cursor,"nodes":nodes_json,"roots":roots_json,"reads":reads_json,"strings":strings_json,"arrays":arrays_json});
    Capture { value, unknown }
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
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectionRequest {
    pub output: String,
    #[serde(default)]
    pub root_start: usize,
    #[serde(default)]
    pub expected_root_catalog_sha256: Option<String>,
    #[serde(default)]
    pub paths: Vec<Locator>,
}
fn inspect(request: &InspectionRequest) -> Result<Value> {
    use sha2::{Digest, Sha256};
    let pid = unsafe { GetCurrentProcessId() };
    let scope = Scope {
        pid,
        created: win::creation(unsafe { GetCurrentProcess() })?,
        session: win::session(pid)?,
    };
    if request.paths.len() > reversebolt_core::runtime::MAX_REFRESH_PATHS {
        return Err("too many runtime paths".into());
    }
    if !request.paths.is_empty()
        && (request.root_start != 0 || request.expected_root_catalog_sha256.is_some())
    {
        return Err("path refresh and root continuation cannot be combined".into());
    }
    for path in &request.paths {
        path.validate(&scope)?;
    }
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
    let mut c = catalog(&api, start)?;
    let catalog_ms = start.elapsed().as_millis() as u64;
    if !request.paths.is_empty() {
        return paths::refresh(&api, &mut c, &request.paths, &scope, start, catalog_ms);
    }
    let descriptor: Vec<_> = c
        .roots
        .iter()
        .map(|r| {
            let k = &c.classes[r.class];
            let f = &k.fields[r.field];
            json!([
                c.images[k.image],
                k.namespace,
                k.name,
                f.name,
                f.kind,
                f.offset
            ])
        })
        .collect();
    let root_fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&descriptor).map_err(|e| e.to_string())?)
    );
    if request.root_start > 0 && request.expected_root_catalog_sha256.is_none() {
        return Err("continuation requires the previous root catalog fingerprint".into());
    }
    if request
        .expected_root_catalog_sha256
        .as_ref()
        .is_some_and(|h| *h != root_fingerprint)
    {
        return Err("root catalog changed; restart discovery with a fresh catalog".into());
    }
    if request.root_start > c.roots.len() {
        return Err("root start outside catalog".into());
    }
    let mut next_root = request.root_start;
    let mut snapshots = Vec::new();
    let mut warmups = Vec::new();
    let mut output_bytes = 0;
    let mut stop_reason = "root_scan_complete";
    while next_root < c.roots.len() {
        if start.elapsed() > Duration::from_secs(18) {
            stop_reason = "time_budget";
            break;
        }
        let end = (next_root + ROOT_BATCH).min(c.roots.len());
        let mut shot = snapshot(&api, &c, next_root, end);
        let mut warming_error = None;
        for attempt in 0..3 {
            if shot.unknown.is_empty() {
                break;
            }
            let unknown = shot.unknown.len();
            let warm_start = Instant::now();
            match prepare_encountered(&api, &mut c, &shot.unknown, start) {
                Ok(added) => {
                    warmups.push(json!({"root_start":next_root,"attempt":attempt,"unknown_types":unknown,"added_classes":added,"elapsed_ms":warm_start.elapsed().as_millis() as u64,"discarded_snapshot":{"nodes":shot.value["nodes"].as_array().unwrap().len(),"pause_held_us":shot.value["pause_held_us"],"stop_to_resume_us":shot.value["stop_to_resume_us"],"helper_allocations_while_paused":shot.value["helper_allocations_while_paused"],"helper_deallocations_while_paused":shot.value["helper_deallocations_while_paused"]}}));
                    if added == 0 {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                    shot = snapshot(&api, &c, next_root, end);
                }
                Err(e) => {
                    warming_error = Some(e);
                    break;
                }
            }
        }
        if let Some(e) = warming_error {
            shot.value["warming_error"] = json!(e);
            stop_reason = "metadata_budget_or_error";
        }
        let length = serde_json::to_vec(&shot.value)
            .map_err(|e| e.to_string())?
            .len();
        if output_bytes + length > 24 * 1024 * 1024 {
            stop_reason = "output_budget";
            break;
        }
        output_bytes += length;
        let scanned = shot.value["roots"].as_array().unwrap().len();
        next_root += scanned;
        shot.value["snapshot_id"] = json!(snapshots.len());
        snapshots.push(shot.value);
        if scanned == 0 {
            stop_reason = "no_root_progress";
            break;
        }
        if stop_reason != "root_scan_complete" {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut used = std::collections::BTreeSet::new();
    for shot in &snapshots {
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
    let schema:Vec<_>=used.into_iter().map(|id|{let k=&c.classes[id];let shape=match k.shape{Shape::Object=>"object",Shape::String=>"string",Shape::Array{..}=>"array",Shape::UnsupportedArray=>"unsupported_array"};json!({"id":id,"key":k.key,"name":k.name,"namespace":k.namespace,"image":c.images[k.image],"instance_size":k.size,"shape":shape,"fields":k.fields.iter().enumerate().map(|(i,f)|json!({"id":i,"name":f.name,"offset":f.offset,"type_code":f.kind,"width":f.width,"reference":f.reference,"flags":f.flags})).collect::<Vec<_>>()})}).collect();
    Ok(
        json!({"schema":4,"pid":pid,"created":win::creation(unsafe{GetCurrentProcess()})?,"session":win::session(pid)?,"runtime":module.path,"catalog_ms":catalog_ms,"catalog_classes":c.classes.len(),"catalog_fields":c.field_count,"catalog_roots":c.roots.len(),"root_catalog_sha256":root_fingerprint,"root_start":request.root_start,"next_root":next_root,"root_scan_complete":next_root==c.roots.len(),"graph_complete":false,"stop_reason":stop_reason,"catalog_omissions":c.omissions,"classes":schema,"snapshots":snapshots,"warmups":warmups,"elapsed_ms":start.elapsed().as_millis() as u64,"edit_authority":false,"identity_contract":"object IDs belong only to one paused snapshot; addresses may be reused; fresh snapshots follow type preparation; no retained object handles","instrumentation":"runtime thread attachment; metadata preparation between bounded managed-world pauses; preallocated snapshot reads; no gameplay field writes"}),
    )
}
/// # Safety
/// argument is NUL-terminated UTF-16 JSON for an InspectionRequest in this process.
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
        let encoded = String::from_utf16(&path).map_err(|_| "request encoding")?;
        let request: InspectionRequest =
            serde_json::from_str(&encoded).map_err(|e| e.to_string())?;
        if !std::path::Path::new(&request.output).is_absolute() {
            return Err("absolute path required".into());
        }
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&request.output)
            .map_err(|e| e.to_string())?;
        let report = inspect(&request).unwrap_or_else(|e| json!({"error":e}));
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
