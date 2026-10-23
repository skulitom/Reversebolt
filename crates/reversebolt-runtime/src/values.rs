//! Optional metadata APIs for unboxed value storage. Never called while paused.
use super::*;

pub(super) struct Api {
    is_enum: unsafe extern "C" fn(Ptr) -> bool,
    enum_basetype: unsafe extern "C" fn(Ptr) -> Ptr,
    value_size: unsafe extern "C" fn(Ptr, *mut u32) -> i32,
}

impl Api {
    pub(super) unsafe fn load(module: usize) -> Option<Self> {
        unsafe {
            Some(Self {
                is_enum: std::mem::transmute::<
                    unsafe extern "system" fn() -> isize,
                    unsafe extern "C" fn(Ptr) -> bool,
                >(GetProcAddress(
                    module as _,
                    c"il2cpp_class_is_enum".as_ptr().cast(),
                )?),
                enum_basetype: std::mem::transmute::<
                    unsafe extern "system" fn() -> isize,
                    unsafe extern "C" fn(Ptr) -> Ptr,
                >(GetProcAddress(
                    module as _,
                    c"il2cpp_class_enum_basetype".as_ptr().cast(),
                )?),
                value_size: std::mem::transmute::<
                    unsafe extern "system" fn() -> isize,
                    unsafe extern "C" fn(Ptr, *mut u32) -> i32,
                >(GetProcAddress(
                    module as _,
                    c"il2cpp_class_value_size".as_ptr().cast(),
                )?),
            })
        }
    }
}

pub(super) struct Layout {
    pub kind: i32,
    pub width: usize,
    pub reference: bool,
    pub inline: usize,
    pub declared_type: Option<String>,
}

pub(super) fn size(api: &super::Api, class: Ptr) -> Option<usize> {
    let extension = api.values.as_ref()?;
    if class.is_null() || !unsafe { (api.il2cpp_class_is_valuetype)(class) } {
        return None;
    }
    let size = unsafe { (extension.value_size)(class, null_mut()) };
    (1..=1_048_576).contains(&size).then_some(size as usize)
}

pub(super) fn layout(api: &super::Api, typ: Ptr) -> Result<Layout> {
    let kind = unsafe { (api.il2cpp_type_get_type)(typ) };
    let byref = unsafe { (api.il2cpp_type_is_byref)(typ) };
    let reference = !byref && reference_kind(api, typ, kind);
    let mut result = Layout {
        kind,
        width: if byref {
            0
        } else if reference {
            8
        } else {
            width(kind)
        },
        reference,
        inline: 0,
        declared_type: None,
    };
    if !byref && !reference && matches!(kind, 0x11 | 0x15) {
        let class = unsafe { (api.il2cpp_class_from_type)(typ) };
        if let (Some(extension), Some(size)) = (&api.values, size(api, class)) {
            result.declared_type = Some(qualified_name(api, typ)?);
            if unsafe { (extension.is_enum)(class) } {
                let base = unsafe { (extension.enum_basetype)(class) };
                if !base.is_null() {
                    let underlying = unsafe { (api.il2cpp_type_get_type)(base) };
                    if matches!(underlying, 4..=11) && width(underlying) == size {
                        result.kind = underlying;
                        result.width = size;
                    }
                }
            } else {
                result.width = size;
                result.inline = class as usize;
            }
        }
    }
    Ok(result)
}
