//! Owned reference-replacement/collection-lifetime simulation, not a real GC.
#![allow(clippy::missing_safety_doc)]
use std::{
    cell::UnsafeCell,
    ffi::c_void,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
type Ptr = *mut c_void;
struct Shared<T>(UnsafeCell<T>);
// Mutations and snapshot reads are serialized by LOCK. Metadata is immutable.
unsafe impl<T> Sync for Shared<T> {}
static LOCK: AtomicBool = AtomicBool::new(false);
static STOPS: AtomicU32 = AtomicU32::new(0);
static STARTS: AtomicU32 = AtomicU32::new(0);
#[repr(C)]
#[derive(Clone, Copy)]
struct Object {
    class: usize,
    monitor: usize,
    reserve: f32,
    identity: i32,
    counter: u32,
    hidden: f32,
    peer: usize,
}
const EMPTY: Object = Object {
    class: 2,
    monitor: 0,
    reserve: 4.25,
    identity: 0,
    counter: 0,
    hidden: 0.0,
    peer: 0,
};
#[repr(C)]
struct Roots {
    active: usize,
    companion: usize,
    generation: u32,
    pad: u32,
}
static ROOTS: Shared<Roots> = Shared(UnsafeCell::new(Roots {
    active: 0,
    companion: 0,
    generation: 0,
    pad: 0,
}));
static OBJECTS: Shared<[Object; 4]> = Shared(UnsafeCell::new([EMPTY; 4]));
static ASSEMBLY: usize = 1;
fn lock() {
    while LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::hint::spin_loop()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn MockTick() {
    lock();
    unsafe {
        let roots = &mut *ROOTS.0.get();
        roots.generation += 1;
        let id = roots.generation as i32 * 2;
        let slot = (roots.generation as usize % 2) * 2;
        let objects = &mut *OBJECTS.0.get();
        objects[slot] = Object {
            identity: id,
            counter: roots.generation * 17,
            hidden: id as f32 + 0.375,
            ..EMPTY
        };
        objects[slot + 1] = Object {
            identity: id + 1,
            counter: roots.generation * 19,
            hidden: id as f32 + 1.375,
            ..EMPTY
        };
        let a = (&objects[slot] as *const Object) as usize;
        let b = (&objects[slot + 1] as *const Object) as usize;
        objects[slot].peer = b;
        objects[slot + 1].peer = a;
        roots.active = a;
        roots.companion = b;
    }
    LOCK.store(false, Ordering::Release);
}
#[unsafe(no_mangle)]
pub extern "C" fn MockBalance() -> u64 {
    ((STOPS.load(Ordering::Relaxed) as u64) << 32) | STARTS.load(Ordering::Relaxed) as u64
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_stop_gc_world() {
    lock();
    STOPS.fetch_add(1, Ordering::Relaxed);
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_start_gc_world() {
    STARTS.fetch_add(1, Ordering::Relaxed);
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
    unsafe {
        *count = 1;
    }
    (&ASSEMBLY as *const usize).cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_assembly_get_image(_: Ptr) -> Ptr {
    1usize as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_name(_: Ptr) -> *const u8 {
    c"OwnedObjectFixture".as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_class_count(_: Ptr) -> usize {
    3
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_class(_: Ptr, index: usize) -> Ptr {
    (index + 1) as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_name(class: Ptr) -> *const u8 {
    [c"Roots", c"Actor", c"Ancestor"][class as usize - 1]
        .as_ptr()
        .cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_namespace(_: Ptr) -> *const u8 {
    c"Owned".as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_parent(class: Ptr) -> Ptr {
    if class as usize == 2 {
        3usize as Ptr
    } else {
        std::ptr::null_mut()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_is_generic(_: Ptr) -> bool {
    false
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_is_valuetype(_: Ptr) -> bool {
    false
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_from_type(_: Ptr) -> Ptr {
    2usize as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_instance_size(class: Ptr) -> i32 {
    match class as usize {
        2 => 40,
        3 => 24,
        _ => 16,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_data_size(class: Ptr) -> u32 {
    if class as usize == 1 { 24 } else { 0 }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_static_field_data(class: Ptr) -> Ptr {
    if class as usize == 1 {
        ROOTS.0.get().cast()
    } else {
        std::ptr::null_mut()
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_class_get_fields(class: Ptr, iter: *mut Ptr) -> Ptr {
    let (start, count) = match class as usize {
        1 => (1, 3),
        2 => (4, 4),
        3 => (8, 1),
        _ => unreachable!(),
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
    [
        c"Active",
        c"Companion",
        c"Generation",
        c"Identity",
        c"Counter",
        c"HiddenScale",
        c"Peer",
        c"Reserve",
    ][field as usize - 1]
        .as_ptr()
        .cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_flags(field: Ptr) -> i32 {
    if field as usize <= 3 { 0x10 } else { 0 }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_offset(field: Ptr) -> usize {
    [0, 8, 16, 20, 24, 28, 32, 16][field as usize - 1]
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_type(field: Ptr) -> Ptr {
    field
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_type_get_type(typ: Ptr) -> i32 {
    match typ as usize {
        1 | 2 | 7 => 0x12,
        6 | 8 => 12,
        4 => 8,
        _ => 9,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_type_is_byref(_: Ptr) -> bool {
    false
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_object_header_size() -> u32 {
    16
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_object_get_class(object: Ptr) -> Ptr {
    unsafe { (*(object as *const Object)).class as Ptr }
}
