//! IL2CPP feasibility probe. Never linked into the shipping Reversebolt application.
//! Only exported runtime APIs describe fields; no game layouts or name filters.
pub mod win;
use serde_json::{Value, json};
use std::{ffi::c_void, io::Write, ptr::null_mut, time::Instant};
use win::{Result, self_read, text_pointer};
use windows_sys::Win32::System::{
    LibraryLoader::GetProcAddress,
    Threading::{GetCurrentProcess, GetCurrentProcessId},
};
type Ptr = *mut c_void;

macro_rules! api {
    ($($name:ident: $ty:ty),+ $(,)?) => {
        struct Api { $($name: $ty),+ }
        impl Api {
            unsafe fn load(module: usize) -> Result<Self> {
                Ok(Self { $($name: unsafe {
                    let name = concat!(stringify!($name), "\0");
                    let pointer = GetProcAddress(module as _, name.as_ptr()).ok_or_else(|| format!("missing {}", stringify!($name)))?;
                    std::mem::transmute::<unsafe extern "system" fn() -> isize, $ty>(pointer)
                }),+ })
            }
        }
    };
}
// ABI declarations correspond to Unity's public il2cpp-api-functions.h.
api! {
    il2cpp_domain_get: unsafe extern "C" fn() -> Ptr,
    il2cpp_domain_get_assemblies: unsafe extern "C" fn(Ptr, *mut usize) -> *const Ptr,
    il2cpp_thread_attach: unsafe extern "C" fn(Ptr) -> Ptr,
    il2cpp_thread_detach: unsafe extern "C" fn(Ptr),
    il2cpp_assembly_get_image: unsafe extern "C" fn(Ptr) -> Ptr,
    il2cpp_image_get_name: unsafe extern "C" fn(Ptr) -> *const u8,
    il2cpp_image_get_class_count: unsafe extern "C" fn(Ptr) -> usize,
    il2cpp_image_get_class: unsafe extern "C" fn(Ptr, usize) -> Ptr,
    il2cpp_class_get_name: unsafe extern "C" fn(Ptr) -> *const u8,
    il2cpp_class_get_namespace: unsafe extern "C" fn(Ptr) -> *const u8,
    il2cpp_class_get_fields: unsafe extern "C" fn(Ptr, *mut Ptr) -> Ptr,
    il2cpp_class_is_generic: unsafe extern "C" fn(Ptr) -> bool,
    il2cpp_class_get_data_size: unsafe extern "C" fn(Ptr) -> u32,
    il2cpp_class_get_static_field_data: unsafe extern "C" fn(Ptr) -> Ptr,
    il2cpp_field_get_name: unsafe extern "C" fn(Ptr) -> *const u8,
    il2cpp_field_get_flags: unsafe extern "C" fn(Ptr) -> i32,
    il2cpp_field_get_offset: unsafe extern "C" fn(Ptr) -> usize,
    il2cpp_field_get_type: unsafe extern "C" fn(Ptr) -> Ptr,
    il2cpp_type_get_type: unsafe extern "C" fn(Ptr) -> i32,
    il2cpp_type_is_byref: unsafe extern "C" fn(Ptr) -> bool,
}
struct Attachment<'a> {
    api: &'a Api,
    thread: Ptr,
}
impl Drop for Attachment<'_> {
    fn drop(&mut self) {
        unsafe { (self.api.il2cpp_thread_detach)(self.thread) }
    }
}

fn width(kind: i32) -> Option<usize> {
    match kind {
        2 | 4 | 5 => Some(1),
        3 | 6 | 7 => Some(2),
        8 | 9 | 12 => Some(4),
        10 | 11 | 13 => Some(8),
        _ => None,
    }
}
fn decode(kind: i32, b: &[u8]) -> Value {
    match kind {
        2 => json!({"boolean": b[0] != 0, "raw": b[0]}),
        3 | 7 => json!(u16::from_le_bytes(b.try_into().unwrap())),
        4 => json!(b[0] as i8),
        5 => json!(b[0]),
        6 => json!(i16::from_le_bytes(b.try_into().unwrap())),
        8 => json!(i32::from_le_bytes(b.try_into().unwrap())),
        9 => json!(u32::from_le_bytes(b.try_into().unwrap())),
        10 => json!(i64::from_le_bytes(b.try_into().unwrap()).to_string()),
        11 => json!(u64::from_le_bytes(b.try_into().unwrap()).to_string()),
        12 => json!(f32::from_le_bytes(b.try_into().unwrap()).to_string()),
        13 => json!(f64::from_le_bytes(b.try_into().unwrap()).to_string()),
        _ => Value::Null,
    }
}
fn name(pointer: *const u8) -> Value {
    match text_pointer(pointer as usize) {
        Ok(s) => json!({"text":s}),
        Err(e) => json!({"error":e}),
    }
}
fn sample(address: usize, kind: i32, size: usize, start: Instant) -> Value {
    let before_us = start.elapsed().as_micros() as u64;
    match self_read(address, size) {
        Ok(bytes) => {
            json!({"before_us":before_us,"after_us":start.elapsed().as_micros() as u64,"value":decode(kind,&bytes),"bytes":bytes})
        }
        Err(e) => json!({"before_us":before_us,"error":e}),
    }
}
fn inspect() -> Result<Value> {
    let start = Instant::now();
    let pid = unsafe { GetCurrentProcessId() };
    let candidates: Vec<_> = win::modules(pid)?
        .into_iter()
        .filter_map(|m| unsafe { Api::load(m.base).ok().map(|api| (m, api)) })
        .collect();
    if candidates.len() != 1 {
        return Err(format!(
            "expected one complete IL2CPP API provider, found {}",
            candidates.len()
        ));
    }
    let (module, api) = candidates.into_iter().next().unwrap();
    unsafe {
        let domain = (api.il2cpp_domain_get)();
        if domain.is_null() {
            return Err("runtime domain not ready".into());
        }
        let thread = (api.il2cpp_thread_attach)(domain);
        if thread.is_null() {
            return Err("runtime thread attachment failed".into());
        }
        let _attachment = Attachment { api: &api, thread };
        let mut assembly_count = 0;
        let assembly_list = (api.il2cpp_domain_get_assemblies)(domain, &mut assembly_count);
        if assembly_count > 256 || assembly_count == 0 {
            return Err(format!("assembly count outside budget: {assembly_count}"));
        }
        let assembly_bytes = self_read(assembly_list as usize, assembly_count * 8)?;
        let mut images = Vec::new();
        let mut classes = Vec::new();
        let mut fields = Vec::new();
        let mut omissions = Vec::new();
        'assemblies: for (assembly_index, bytes) in
            assembly_bytes.as_chunks::<8>().0.iter().enumerate()
        {
            let assembly = usize::from_le_bytes(*bytes) as Ptr;
            let image = (api.il2cpp_assembly_get_image)(assembly);
            if image.is_null() {
                omissions.push("null assembly image");
                continue;
            }
            let class_count = (api.il2cpp_image_get_class_count)(image);
            images.push(json!({"id":assembly_index,"name":name((api.il2cpp_image_get_name)(image)),"reported_class_count":class_count}));
            if class_count > 50_000 {
                omissions.push("image class budget exceeded");
                continue;
            }
            for index in 0..class_count {
                if classes.len() >= 50_000
                    || fields.len() >= 200_000
                    || start.elapsed().as_secs() >= 25
                {
                    omissions.push("global budget reached");
                    break 'assemblies;
                }
                let class = (api.il2cpp_image_get_class)(image, index);
                if class.is_null() {
                    omissions.push("null class");
                    continue;
                }
                let class_id = classes.len();
                let generic = (api.il2cpp_class_is_generic)(class);
                classes.push(json!({"id":class_id,"image":assembly_index,"index":index,"name":name((api.il2cpp_class_get_name)(class)),"namespace":name((api.il2cpp_class_get_namespace)(class)),"open_generic":generic}));
                let mut iterator = null_mut();
                // Enumeration may lazily initialize runtime metadata. No managed constructors are requested.
                loop {
                    if fields.len() >= 200_000 || start.elapsed().as_secs() >= 25 {
                        omissions.push("field/time budget reached");
                        break 'assemblies;
                    }
                    let field = (api.il2cpp_class_get_fields)(class, &mut iterator);
                    if field.is_null() {
                        break;
                    }
                    let flags = (api.il2cpp_field_get_flags)(field);
                    let offset = (api.il2cpp_field_get_offset)(field);
                    let typ = (api.il2cpp_field_get_type)(field);
                    if typ.is_null() {
                        omissions.push("null field type");
                        continue;
                    }
                    let kind = (api.il2cpp_type_get_type)(typ);
                    let mut record = json!({"class":class_id,"name":name((api.il2cpp_field_get_name)(field)),"flags":flags,"offset":offset,"type_code":kind,"status":"metadata_only"});
                    let reason = if flags & 0x10 == 0 {
                        "instance_object_required"
                    } else if flags & 0x40 != 0 {
                        "literal"
                    } else if generic {
                        "open_generic"
                    } else if (api.il2cpp_type_is_byref)(typ) || width(kind).is_none() {
                        "unsupported_type"
                    } else {
                        let storage = (api.il2cpp_class_get_static_field_data)(class) as usize;
                        let size = (api.il2cpp_class_get_data_size)(class) as usize;
                        let count = width(kind).unwrap();
                        if storage == 0 {
                            "no_existing_static_storage"
                        } else if offset.checked_add(count).is_none_or(|end| end > size)
                            || storage.checked_add(offset).is_none()
                        {
                            "outside_static_storage_or_thread_static"
                        } else {
                            let address = storage + offset;
                            record["status"] = json!("live_static_read_attempt");
                            record["address"] = json!(format!("0x{address:x}"));
                            record["samples"] = json!([
                                sample(address, kind, count, start),
                                sample(address, kind, count, start)
                            ]);
                            "runtime_api_static_storage"
                        }
                    };
                    record["reason"] = json!(reason);
                    fields.push(record);
                }
            }
        }
        Ok(
            json!({"schema":1,"pid":pid,"created":win::creation(GetCurrentProcess())?,"session":win::session(pid)?,"runtime":{"path":module.path,"base":format!("0x{:x}",module.base)},"instrumentation":"temporary native helper; runtime metadata APIs and current-process memory reads; no field setters, property invocation, class initialization request, GC control, or gameplay inputs","snapshots_are_atomic":false,"enumeration_elapsed_ms":start.elapsed().as_millis() as u64,"images":images,"classes":classes,"fields":fields,"omissions":omissions,"edit_authority":false}),
        )
    }
}

/// Run once on an explicit loader-created thread; no work is done from DllMain.
/// # Safety
/// `argument` must point to a valid, NUL-terminated UTF-16 output path in this process.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn ReverseboltInspect(argument: Ptr) -> u32 {
    let result = std::panic::catch_unwind(|| -> Result<()> {
        let mut path = Vec::new();
        for i in 0..32768 {
            let b = self_read(
                (argument as usize)
                    .checked_add(i * 2)
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
            return Err("invalid output path".into());
        }
        let path = String::from_utf16(&path).map_err(|_| "invalid UTF16")?;
        if !std::path::Path::new(&path).is_absolute() {
            return Err("absolute output path required".into());
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let report = match inspect() {
            Ok(r) => r,
            Err(e) => json!({"schema":1,"error":e}),
        };
        serde_json::to_writer(&mut file, &report).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
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
    #[test]
    fn primitive_decoding_preserves_sign_and_large_integers() {
        assert_eq!(decode(8, &(-123i32).to_le_bytes()), json!(-123));
        assert_eq!(
            decode(11, &u64::MAX.to_le_bytes()),
            json!(u64::MAX.to_string())
        );
        assert_eq!(decode(12, &f32::NAN.to_le_bytes()), json!("NaN"));
        assert_eq!(width(0x12), None);
    }
}
