//! Owned ABI fixture; NOT an IL2CPP implementation or independent-game evidence.
#![allow(clippy::missing_safety_doc)]
use std::{
    ffi::c_void,
    sync::atomic::{AtomicU32, Ordering},
};
type Ptr = *mut c_void;
#[repr(C)]
struct State {
    charge: u32,
    hidden_bias: f32,
    enabled: u8,
    pad: [u8; 3],
    ticks: AtomicU32,
}
static STATE: State = State {
    charge: 37,
    hidden_bias: 1.75,
    enabled: 1,
    pad: [0; 3],
    ticks: AtomicU32::new(0),
};
static ASSEMBLY: usize = 1;
#[unsafe(no_mangle)]
pub extern "C" fn MockTick() {
    STATE.ticks.fetch_add(1, Ordering::Relaxed);
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
    c"OwnedAbiFixture".as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_class_count(_: Ptr) -> usize {
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_class(_: Ptr, _: usize) -> Ptr {
    1usize as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_name(_: Ptr) -> *const u8 {
    c"Dynamics".as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_namespace(_: Ptr) -> *const u8 {
    c"Owned".as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_is_generic(_: Ptr) -> bool {
    false
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_data_size(_: Ptr) -> u32 {
    std::mem::size_of::<State>() as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_static_field_data(_: Ptr) -> Ptr {
    (&STATE as *const State).cast_mut().cast()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn il2cpp_class_get_fields(_: Ptr, iter: *mut Ptr) -> Ptr {
    unsafe {
        let n = *iter as usize;
        if n == 8 {
            return std::ptr::null_mut();
        }
        *iter = (n + 1) as Ptr;
        (n + 1) as Ptr
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_name(f: Ptr) -> *const u8 {
    let names = [
        c"charge",
        c"hiddenBias",
        c"enabled",
        c"ticks",
        c"threadLocal",
        c"constant",
        c"instanceOnly",
        c"reference",
    ];
    names[f as usize - 1].as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_flags(f: Ptr) -> i32 {
    match f as usize {
        6 => 0x50,
        7 => 0,
        _ => 0x10,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_offset(f: Ptr) -> usize {
    match f as usize {
        1 => 0,
        2 => 4,
        3 => 8,
        4 => 12,
        5 => usize::MAX,
        _ => 0,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_type(f: Ptr) -> Ptr {
    f
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_type_get_type(t: Ptr) -> i32 {
    match t as usize {
        2 => 12,
        3 => 2,
        8 => 0x12,
        _ => 9,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_type_is_byref(_: Ptr) -> bool {
    false
}
