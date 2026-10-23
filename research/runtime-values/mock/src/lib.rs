//! Owned ABI fixture. Deliberately rejects object APIs on unboxed storage.
use std::{
    cell::UnsafeCell,
    ffi::{CString, c_void},
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
type Ptr = *mut c_void;
struct Shared<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Shared<T> {}
static STOPPED: AtomicBool = AtomicBool::new(false);
static LOCK: AtomicBool = AtomicBool::new(false);
static STARTS: AtomicU32 = AtomicU32::new(0);
static STOPS: AtomicU32 = AtomicU32::new(0);
static MODE: AtomicU32 = AtomicU32::new(0);
static INIT: AtomicBool = AtomicBool::new(false);
static ASSEMBLY: usize = 1;
#[cfg(feature = "large-metadata")]
static NOISE_FIELDS: AtomicU32 = AtomicU32::new(0);
#[cfg(feature = "large-metadata")]
static NOISE_ROOT: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "large-metadata")]
#[unsafe(no_mangle)]
pub extern "C" fn MockMetadata(fields: u32, root: bool) {
    metadata();
    NOISE_FIELDS.store(fields, Ordering::Relaxed);
    NOISE_ROOT.store(root, Ordering::Relaxed);
}
fn metadata() {
    assert!(!STOPPED.load(Ordering::Relaxed));
}
fn lock() {
    while LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Pair {
    x: f32,
    y: f32,
    target: usize,
}
#[repr(C)]
struct Wrapper {
    pair: Pair,
    mode: u8,
    pad: [u8; 7],
}
#[repr(C)]
struct Container {
    class: usize,
    monitor: usize,
    pose: Pair,
    readonly: Pair,
    points: usize,
    phase: u8,
    pad: [u8; 7],
    wide: u64,
    wrapped: Wrapper,
    enums: usize,
}
#[repr(C)]
struct Array<T, const N: usize> {
    class: usize,
    monitor: usize,
    bounds: usize,
    length: usize,
    values: [T; N],
}
#[repr(C)]
struct Target {
    class: usize,
    monitor: usize,
    score: i32,
    pad: u32,
}
#[repr(C)]
struct Roots {
    current: usize,
    snapshot: Pair,
    readonly: Pair,
}
const P: Pair = Pair {
    x: 3.25,
    y: -7.5,
    target: 0,
};
static ROOTS: Shared<Roots> = Shared(UnsafeCell::new(Roots {
    current: 0,
    snapshot: P,
    readonly: P,
}));
static OBJECT: Shared<Container> = Shared(UnsafeCell::new(Container {
    class: 2,
    monitor: 0,
    pose: P,
    readonly: P,
    points: 0,
    phase: 7,
    pad: [0; 7],
    wide: u64::MAX - 2,
    wrapped: Wrapper {
        pair: P,
        mode: 9,
        pad: [0; 7],
    },
    enums: 0,
}));
static POINTS: Shared<Array<Pair, 3>> = Shared(UnsafeCell::new(Array {
    class: 7,
    monitor: 0,
    bounds: 0,
    length: 3,
    values: [P; 3],
}));
static ENUMS: Shared<Array<u8, 3>> = Shared(UnsafeCell::new(Array {
    class: 8,
    monitor: 0,
    bounds: 0,
    length: 3,
    values: [1, 7, 255],
}));
static TARGET: Shared<Target> = Shared(UnsafeCell::new(Target {
    class: 9,
    monitor: 0,
    score: 19,
    pad: 0,
}));

#[unsafe(no_mangle)]
pub extern "C" fn MockTick() {
    let control =
        std::env::var_os("REVERSEBOLT_VALUE_FIXTURE_CONTROL").map(std::path::PathBuf::from);
    let mode = control
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    lock();
    unsafe {
        if !INIT.swap(true, Ordering::Relaxed) {
            (*OBJECT.0.get()).pose.target = TARGET.0.get() as usize;
            (*OBJECT.0.get()).readonly.target = TARGET.0.get() as usize;
            (*OBJECT.0.get()).points = POINTS.0.get() as usize;
            (*OBJECT.0.get()).enums = ENUMS.0.get() as usize;
            (*OBJECT.0.get()).wrapped.pair.target = TARGET.0.get() as usize;
            for (i, p) in (*POINTS.0.get()).values.iter_mut().enumerate() {
                p.x = 11.25 + i as f32;
                p.target = TARGET.0.get() as usize;
            }
        }
        (*ROOTS.0.get()).current = if mode == 1 {
            0
        } else {
            OBJECT.0.get() as usize
        };
        (*POINTS.0.get()).length = if mode == 2 { 0 } else { 3 };
    }
    MODE.store(mode, Ordering::Relaxed);
    LOCK.store(false, Ordering::Release);
    if let Some(p) = control {
        std::fs::write(p.with_extension("ack"), mode.to_string()).unwrap();
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
/// # Safety
/// `n` must point to a writable `usize` for the duration of this call.
pub unsafe extern "C" fn il2cpp_domain_get_assemblies(_: Ptr, n: *mut usize) -> *const Ptr {
    metadata();
    unsafe {
        *n = 1;
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
    c"OwnedValues".as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_class_count(_: Ptr) -> usize {
    metadata();
    if cfg!(feature = "large-metadata") {
        13
    } else {
        12
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_image_get_class(_: Ptr, i: usize) -> Ptr {
    metadata();
    if cfg!(feature = "large-metadata") {
        (if i == 0 { 13 } else { i }) as Ptr
    } else {
        (i + 1) as Ptr
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_image(_: Ptr) -> Ptr {
    metadata();
    1usize as Ptr
}
fn name(c: usize) -> &'static std::ffi::CStr {
    if cfg!(feature = "large-metadata") && c == 13 {
        return c"DormantConstants";
    }
    [
        c"Roots",
        c"Container",
        c"Pair",
        c"Wrapper`1[Pair]",
        c"Mode",
        c"WideMode",
        c"Pair[]",
        c"Mode[]",
        c"Target",
        c"Single",
        c"Byte",
        c"UInt64",
    ][c - 1]
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_name(c: Ptr) -> *const u8 {
    metadata();
    name(c as usize).as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_namespace(_: Ptr) -> *const u8 {
    metadata();
    c"Owned".as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_parent(_: Ptr) -> Ptr {
    metadata();
    std::ptr::null_mut()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_is_generic(_: Ptr) -> bool {
    metadata();
    false
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_is_valuetype(c: Ptr) -> bool {
    metadata();
    matches!(c as usize,3..=6|10..=12)
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_is_enum(c: Ptr) -> bool {
    metadata();
    matches!(c as usize, 5 | 6)
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_enum_basetype(c: Ptr) -> Ptr {
    metadata();
    match c as usize {
        5 => 11usize as Ptr,
        6 => 12usize as Ptr,
        _ => std::ptr::null_mut(),
    }
}
fn value_size(c: usize) -> i32 {
    match c {
        3 => 16,
        4 => 24,
        5 | 11 => 1,
        6 | 12 => 8,
        10 => 4,
        _ => panic!("not value type"),
    }
}
#[unsafe(no_mangle)]
/// # Safety
/// `c` must identify a fixture value type; non-null `a` must be writable.
pub unsafe extern "C" fn il2cpp_class_value_size(c: Ptr, a: *mut u32) -> i32 {
    metadata();
    if !a.is_null() {
        unsafe {
            *a = 8;
        }
    }
    value_size(c as usize)
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_from_type(t: Ptr) -> Ptr {
    metadata();
    t
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_type(c: Ptr) -> Ptr {
    metadata();
    c
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_rank(c: Ptr) -> i32 {
    metadata();
    if matches!(c as usize, 7 | 8) { 1 } else { 0 }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_element_class(c: Ptr) -> Ptr {
    metadata();
    match c as usize {
        7 => 3usize as Ptr,
        8 => 5usize as Ptr,
        _ => std::ptr::null_mut(),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_instance_size(c: Ptr) -> i32 {
    metadata();
    match c as usize {
        1 | 13 => 16,
        2 => 104,
        7 | 8 => 32,
        9 => 24,
        n => 16 + value_size(n),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_data_size(c: Ptr) -> u32 {
    metadata();
    if c as usize == 1 || (cfg!(feature = "large-metadata") && c as usize == 13) {
        40
    } else {
        0
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_class_get_static_field_data(c: Ptr) -> Ptr {
    metadata();
    if c as usize == 1 || (cfg!(feature = "large-metadata") && c as usize == 13) {
        ROOTS.0.get().cast()
    } else {
        std::ptr::null_mut()
    }
}
struct F {
    name: &'static std::ffi::CStr,
    offset: usize,
    typ: usize,
    flags: i32,
}
const FIELDS: &[&[F]] = &[
    &[
        F {
            name: c"Current",
            offset: 0,
            typ: 2,
            flags: 16,
        },
        F {
            name: c"Snapshot",
            offset: 8,
            typ: 3,
            flags: 16,
        },
        F {
            name: c"ReadonlySnapshot",
            offset: 24,
            typ: 3,
            flags: 48,
        },
    ],
    &[
        F {
            name: c"Pose",
            offset: 16,
            typ: 3,
            flags: 0,
        },
        F {
            name: c"ReadonlyPose",
            offset: 32,
            typ: 3,
            flags: 32,
        },
        F {
            name: c"Points",
            offset: 48,
            typ: 7,
            flags: 0,
        },
        F {
            name: c"Phase",
            offset: 56,
            typ: 5,
            flags: 0,
        },
        F {
            name: c"WidePhase",
            offset: 64,
            typ: 6,
            flags: 0,
        },
        F {
            name: c"Wrapped",
            offset: 72,
            typ: 4,
            flags: 0,
        },
        F {
            name: c"Modes",
            offset: 96,
            typ: 8,
            flags: 0,
        },
        F {
            name: c"Outside",
            offset: 100,
            typ: 3,
            flags: 0,
        },
    ],
    &[
        F {
            name: c"X",
            offset: 16,
            typ: 10,
            flags: 0,
        },
        F {
            name: c"Y",
            offset: 20,
            typ: 10,
            flags: 0,
        },
        F {
            name: c"Target",
            offset: 24,
            typ: 9,
            flags: 0,
        },
    ],
    &[
        F {
            name: c"Pair",
            offset: 16,
            typ: 3,
            flags: 0,
        },
        F {
            name: c"Mode",
            offset: 32,
            typ: 5,
            flags: 0,
        },
    ],
    &[F {
        name: c"value__",
        offset: 16,
        typ: 11,
        flags: 0,
    }],
    &[F {
        name: c"value__",
        offset: 16,
        typ: 12,
        flags: 0,
    }],
    &[],
    &[],
    &[F {
        name: c"Score",
        offset: 16,
        typ: 11,
        flags: 0,
    }],
    &[],
    &[],
    &[],
];
fn field(p: Ptr) -> &'static F {
    let n = p as usize;
    #[cfg(feature = "large-metadata")]
    if n >= 10_000_000 {
        if n == 10_000_000 && NOISE_ROOT.load(Ordering::Relaxed) {
            return &F {
                name: c"Root",
                offset: 0,
                typ: 2,
                flags: 16,
            };
        }
        return &F {
            name: c"Constant",
            offset: 0,
            typ: 11,
            flags: 0x50,
        };
    }
    &FIELDS[n / 100 - 1][n % 100]
}
#[unsafe(no_mangle)]
/// # Safety
/// `c` must identify a fixture class; `i` must point to its initialized iterator.
pub unsafe extern "C" fn il2cpp_class_get_fields(c: Ptr, i: *mut Ptr) -> Ptr {
    metadata();
    let n = unsafe { *i as usize };
    #[cfg(feature = "large-metadata")]
    if c as usize == 13 {
        if n >= NOISE_FIELDS.load(Ordering::Relaxed) as usize {
            return std::ptr::null_mut();
        }
        unsafe {
            *i = (n + 1) as Ptr;
        }
        return (10_000_000 + n) as Ptr;
    }
    if n == FIELDS[c as usize - 1].len() {
        std::ptr::null_mut()
    } else {
        unsafe {
            *i = (n + 1) as Ptr;
        }
        (c as usize * 100 + n) as Ptr
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_name(f: Ptr) -> *const u8 {
    metadata();
    field(f).name.as_ptr().cast()
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_flags(f: Ptr) -> i32 {
    metadata();
    field(f).flags
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_offset(f: Ptr) -> usize {
    metadata();
    if f as usize == 300 && MODE.load(Ordering::Relaxed) == 3 {
        20
    } else {
        field(f).offset
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_field_get_type(f: Ptr) -> Ptr {
    metadata();
    field(f).typ as Ptr
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_type_get_type(t: Ptr) -> i32 {
    metadata();
    match t as usize {
        3 | 5 | 6 => 0x11,
        4 => 0x15,
        7 | 8 => 0x1d,
        10 => 12,
        11 => 5,
        12 => 11,
        _ => 0x12,
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
pub extern "C" fn il2cpp_array_element_size(c: Ptr) -> i32 {
    metadata();
    match c as usize {
        7 => 16,
        8 => 1,
        _ => panic!("not array"),
    }
}
#[unsafe(no_mangle)]
/// # Safety
/// `o` must be a live fixture object, with fixture mutation excluded by its lock.
pub unsafe extern "C" fn il2cpp_object_get_class(o: Ptr) -> Ptr {
    assert!(
        [
            OBJECT.0.get() as usize,
            POINTS.0.get() as usize,
            ENUMS.0.get() as usize,
            TARGET.0.get() as usize
        ]
        .contains(&(o as usize)),
        "object API called on unboxed data"
    );
    unsafe { *(o as *const usize) as Ptr }
}
#[unsafe(no_mangle)]
/// # Safety
/// `a` must point to a live fixture array while fixture mutation is excluded.
pub unsafe extern "C" fn il2cpp_array_length(a: Ptr) -> u32 {
    unsafe { *((a as *const usize).add(3)) as u32 }
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_string_length(_: Ptr) -> i32 {
    panic!("no strings")
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_string_chars(_: Ptr) -> *const u16 {
    panic!("no strings")
}
#[unsafe(no_mangle)]
pub extern "C" fn il2cpp_type_get_assembly_qualified_name(t: Ptr) -> *mut u8 {
    metadata();
    CString::new(format!(
        "Owned.{}, OwnedValues",
        name(t as usize).to_str().unwrap()
    ))
    .unwrap()
    .into_raw()
    .cast()
}
#[unsafe(no_mangle)]
/// # Safety
/// Non-null `p` must be an unfreed string returned by this fixture's name API.
pub unsafe extern "C" fn il2cpp_free(p: Ptr) {
    metadata();
    if !p.is_null() {
        drop(unsafe { CString::from_raw(p.cast()) });
    }
}
