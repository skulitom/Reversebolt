//! Owned dynamic-schema, collection and non-root-child fixture. No real GC claims.
#![allow(clippy::missing_safety_doc)]
use std::{
    cell::UnsafeCell,
    ffi::{CString, c_void},
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};
type Ptr = *mut c_void;
const ROOT_COUNT: usize = 130;
const TYPE_TAG: usize = 0x100000;
struct Shared<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Shared<T> {}
static LOCK: AtomicBool = AtomicBool::new(false);
static STOPPED: AtomicBool = AtomicBool::new(false);
static STOPS: AtomicU32 = AtomicU32::new(0);
static STARTS: AtomicU32 = AtomicU32::new(0);
static MODE: AtomicU32 = AtomicU32::new(0);
static GENERATION: AtomicU32 = AtomicU32::new(0);
fn lock() {
    while LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::hint::spin_loop()
    }
}
fn metadata() {
    assert!(
        !STOPPED.load(Ordering::Relaxed),
        "metadata API called while stopped"
    )
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Actor {
    class: usize,
    monitor: usize,
    reserve: f32,
    identity: i32,
    counter: u32,
    hidden: f32,
    peer: usize,
}
const ACTOR: Actor = Actor {
    class: 2,
    monitor: 0,
    reserve: 4.25,
    identity: 0,
    counter: 0,
    hidden: 0.0,
    peer: 0,
};
#[repr(C)]
struct Boxed {
    class: usize,
    monitor: usize,
    coefficient: f32,
    pad: u32,
    label: usize,
    items: usize,
    long_label: usize,
    numbers: usize,
}
#[repr(C)]
struct RefArray {
    class: usize,
    monitor: usize,
    bounds: usize,
    length: usize,
    items: [usize; 70],
}
#[repr(C)]
struct FloatArray {
    class: usize,
    monitor: usize,
    bounds: usize,
    length: usize,
    items: [f32; 3],
}
#[repr(C)]
struct Text<const N: usize> {
    class: usize,
    monitor: usize,
    length: i32,
    chars: [u16; N],
}
static ROOTS: Shared<[usize; ROOT_COUNT]> = Shared(UnsafeCell::new([0; ROOT_COUNT]));
static ACTORS: Shared<[Actor; 6]> = Shared(UnsafeCell::new([ACTOR; 6]));
static BOX: Shared<Boxed> = Shared(UnsafeCell::new(Boxed {
    class: 10,
    monitor: 0,
    coefficient: 3.5,
    pad: 0,
    label: 0,
    items: 0,
    long_label: 0,
    numbers: 0,
}));
static ARRAY: Shared<RefArray> = Shared(UnsafeCell::new(RefArray {
    class: 11,
    monitor: 0,
    bounds: 0,
    length: 70,
    items: [0; 70],
}));
static NUMBERS: FloatArray = FloatArray {
    class: 13,
    monitor: 0,
    bounds: 0,
    length: 3,
    items: [1.25, -2.5, 3.75],
};
static LABEL: Text<5> = Text {
    class: 12,
    monitor: 0,
    length: 5,
    chars: [83, 112, 101, 101, 100],
};
static LONG: Text<200> = Text {
    class: 12,
    monitor: 0,
    length: 200,
    chars: [88; 200],
};
static ASSEMBLY: usize = 1;
#[unsafe(no_mangle)]
pub extern "C" fn MockTick() {
    let control =
        std::env::var_os("REVERSEBOLT_PATH_FIXTURE_CONTROL").map(std::path::PathBuf::from);
    let mode = control
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);
    MODE.store(mode, Ordering::Relaxed);
    lock();
    unsafe {
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        let slot = (generation as usize % 2) * 3;
        let actors = &mut *ACTORS.0.get();
        for j in 0..3 {
            let id = generation as i32 * 10 + j as i32;
            actors[slot + j] = Actor {
                identity: id,
                counter: generation * 17 + j as u32,
                hidden: id as f32 + 0.375,
                ..ACTOR
            };
        }
        let head = &actors[slot] as *const Actor as usize;
        let child1 = &actors[slot + 1] as *const Actor as usize;
        let child2 = &actors[slot + 2] as *const Actor as usize;
        actors[slot].peer = BOX.0.get() as usize;
        actors[slot + 1].peer = head;
        actors[slot + 2].peer = head;
        (*ROOTS.0.get()).fill(if mode == 1 { 0 } else { head });
        (*ARRAY.0.get()).length = if mode == 4 { 1 } else { 70 };
        for i in 0..70 {
            (*ARRAY.0.get()).items[i] = if (i + usize::from(mode == 2)) % 2 == 0 {
                child1
            } else {
                child2
            };
        }
        let boxed = &mut *BOX.0.get();
        boxed.class = if mode == 3 { 14 } else { 10 };
        boxed.label = &LABEL as *const _ as usize;
        boxed.items = ARRAY.0.get() as usize;
        boxed.long_label = &LONG as *const _ as usize;
        boxed.numbers = &NUMBERS as *const _ as usize;
    }
    LOCK.store(false, Ordering::Release);
    if let Some(path) = control {
        let _ = std::fs::write(path.with_extension("ack"), mode.to_string());
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn MockBalance() -> u64 {
    ((STOPS.load(Ordering::Relaxed) as u64) << 32) | STARTS.load(Ordering::Relaxed) as u64
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_stop_gc_world() {
    lock();
    STOPPED.store(true, Ordering::Relaxed);
    STOPS.fetch_add(1, Ordering::Relaxed);
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_start_gc_world() {
    STARTS.fetch_add(1, Ordering::Relaxed);
    STOPPED.store(false, Ordering::Relaxed);
    LOCK.store(false, Ordering::Release);
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_domain_get() -> Ptr {
    1usize as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_thread_attach(_: Ptr) -> Ptr {
    1usize as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_thread_detach(_: Ptr) {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_domain_get_assemblies(_: Ptr, count: *mut usize) -> *const Ptr {
    metadata();
    unsafe {
        *count = 1;
    }
    (&ASSEMBLY as *const usize).cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_assembly_get_image(_: Ptr) -> Ptr {
    metadata();
    1usize as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_name(_: Ptr) -> *const u8 {
    metadata();
    c"OwnedPathFixture".as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_class_count(_: Ptr) -> usize {
    metadata();
    3
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_class(_: Ptr, index: usize) -> Ptr {
    metadata();
    (index + 1) as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_image(_: Ptr) -> Ptr {
    metadata();
    1usize as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_name(class: Ptr) -> *const u8 {
    metadata();
    match class as usize {
        1 => c"Roots",
        2 => c"Actor",
        3 => c"Ancestor",
        4 => c"Single",
        10 | 14 => c"Box`1",
        11 => c"Actor[]",
        12 => c"String",
        13 => c"Single[]",
        _ => panic!("unknown class"),
    }
    .as_ptr()
    .cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_namespace(_: Ptr) -> *const u8 {
    metadata();
    c"Owned".as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_parent(class: Ptr) -> Ptr {
    metadata();
    if class as usize == 2 {
        3usize as Ptr
    } else {
        std::ptr::null_mut()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_is_generic(_: Ptr) -> bool {
    metadata();
    false
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_is_valuetype(class: Ptr) -> bool {
    metadata();
    class as usize == 4
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_from_type(_: Ptr) -> Ptr {
    metadata();
    10usize as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_type(class: Ptr) -> Ptr {
    metadata();
    (TYPE_TAG + class as usize) as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_rank(class: Ptr) -> i32 {
    metadata();
    if matches!(class as usize, 11 | 13) {
        1
    } else {
        0
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_element_class(class: Ptr) -> Ptr {
    metadata();
    if class as usize == 11 {
        2usize as Ptr
    } else {
        4usize as Ptr
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_instance_size(class: Ptr) -> i32 {
    metadata();
    match class as usize {
        2 => 40,
        3 => 24,
        10 | 14 => 56,
        11 | 13 => 32,
        12 => 24,
        _ => 16,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_data_size(class: Ptr) -> u32 {
    metadata();
    if class as usize == 1 {
        (ROOT_COUNT * 8) as u32
    } else {
        0
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_static_field_data(class: Ptr) -> Ptr {
    metadata();
    if class as usize == 1 {
        ROOTS.0.get().cast()
    } else {
        std::ptr::null_mut()
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_class_get_fields(class: Ptr, iter: *mut Ptr) -> Ptr {
    metadata();
    let (start, count) = match class as usize {
        1 => (10000, ROOT_COUNT),
        2 => (1, 4),
        3 => (5, 1),
        10 | 14 => (6, if cfg!(feature = "direct-controls") { 6 } else { 5 }),
        _ => (0, 0),
    };
    unsafe {
        let n = *iter as usize;
        if n == count {
            return std::ptr::null_mut();
        }
        *iter = (n + 1) as Ptr;
        (start + n) as Ptr
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_name(field: Ptr) -> *const u8 {
    metadata();
    let id = field as usize;
    if id >= 10000 {
        static NAMES: OnceLock<Vec<CString>> = OnceLock::new();
        NAMES.get_or_init(|| {
            (0..ROOT_COUNT)
                .map(|i| CString::new(format!("Entry{i:03}")).unwrap())
                .collect()
        })[id - 10000]
            .as_ptr()
            .cast()
    } else {
        [
            c"Identity",
            c"Counter",
            c"HiddenScale",
            c"Peer",
            c"Reserve",
            c"Coefficient",
            c"Label",
            c"Items",
            c"LongLabel",
            c"Numbers",
            c"Enabled",
        ][id - 1]
            .as_ptr()
            .cast()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_flags(field: Ptr) -> i32 {
    metadata();
    if field as usize >= 10000 { 0x10 } else { 0 }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_offset(field: Ptr) -> usize {
    metadata();
    let id = field as usize;
    if id == 3 && MODE.load(Ordering::Relaxed) == 5 {
        return 24;
    }
    if id >= 10000 {
        (id - 10000) * 8
    } else {
        [20, 24, 28, 32, 16, 16, 24, 32, 40, 48, 20][id - 1]
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_type(field: Ptr) -> Ptr {
    metadata();
    field
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_type_get_type(typ: Ptr) -> i32 {
    metadata();
    let id = typ as usize;
    if id >= TYPE_TAG {
        match id - TYPE_TAG {
            4 => 12,
            10 | 14 => 0x15,
            11 | 13 => 0x1d,
            12 => 0x0e,
            _ => 0x12,
        }
    } else if id >= 10000 {
        0x12
    } else {
        match id {
            1 => 8,
            2 => 9,
            3 | 5 | 6 => 12,
            4 => 0x1c,
            7 | 9 => 0x0e,
            8 | 10 => 0x1d,
            11 => 2,
            _ => panic!("unknown field type"),
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_type_is_byref(_: Ptr) -> bool {
    metadata();
    false
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_object_header_size() -> u32 {
    metadata();
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_array_object_header_size() -> u32 {
    metadata();
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_array_element_size(class: Ptr) -> i32 {
    metadata();
    if class as usize == 11 { 8 } else { 4 }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_object_get_class(object: Ptr) -> Ptr {
    unsafe { (*(object as *const usize)) as Ptr }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_array_length(array: Ptr) -> u32 {
    #[repr(C)]
    struct Header {
        class: usize,
        monitor: usize,
        bounds: usize,
        length: usize,
    }
    unsafe { (*(array as *const Header)).length as u32 }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_string_length(string: Ptr) -> i32 {
    unsafe { (*(string as *const Text<0>)).length }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_string_chars(string: Ptr) -> *const u16 {
    unsafe { std::ptr::addr_of!((*(string as *const Text<0>)).chars).cast() }
}

#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_type_get_assembly_qualified_name(typ: Ptr) -> *mut u8 {
    metadata();
    let class = typ as usize - TYPE_TAG;
    let name = match class {
        1 => "Owned.Roots",
        2 => "Owned.Actor",
        3 => "Owned.Ancestor",
        4 => "System.Single",
        10 => "Owned.Box`1[[Owned.Actor, OwnedPathFixture]]",
        14 => "Owned.Box`1[[Owned.Ancestor, OwnedPathFixture]]",
        11 => "Owned.Actor[]",
        12 => "System.String",
        13 => "System.Single[]",
        _ => panic!("unknown class"),
    };
    CString::new(format!("{name}, OwnedPathFixture"))
        .unwrap()
        .into_raw()
        .cast()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_free(pointer: Ptr) {
    metadata();
    if !pointer.is_null() {
        drop(unsafe { CString::from_raw(pointer.cast()) });
    }
}
