//! Cheap root eligibility first; full typed layouts are loaded only when needed.
use super::*;

pub(super) const MAX_CLASSES: usize = 50_000;
pub(super) const MAX_FIELDS: usize = 200_000;
// Header enumeration retains no names or layouts for non-root fields. Give it a
// separate finite work bound; the retained-layout and elapsed budgets stay intact.
const MAX_ROOT_HEADERS: usize = 2_000_000;

#[derive(Default, serde::Serialize)]
pub(super) struct RootInventory {
    classes_examined: usize,
    field_headers_examined: usize,
    open_generic_classes: usize,
    potential_root_owners: usize,
    pub(super) complete: bool,
}

pub(super) fn root_owner(
    api: &Api,
    class: Ptr,
    inventory: &mut RootInventory,
    start: Instant,
) -> Result<bool> {
    if class.is_null() {
        return Err("null declared class during root inventory".into());
    }
    if inventory.classes_examined >= MAX_CLASSES || start.elapsed() > Duration::from_secs(20) {
        return Err("root class inventory budget".into());
    }
    inventory.classes_examined += 1;
    unsafe {
        if (api.il2cpp_class_is_generic)(class) {
            inventory.open_generic_classes += 1;
            return Ok(false);
        }
        let mut iter = null_mut();
        loop {
            if start.elapsed() > Duration::from_secs(20) {
                return Err("root inventory time budget".into());
            }
            let field = (api.il2cpp_class_get_fields)(class, &mut iter);
            if field.is_null() {
                return Ok(false);
            }
            if inventory.field_headers_examined >= MAX_ROOT_HEADERS {
                return Err("root field header budget".into());
            }
            inventory.field_headers_examined += 1;
            let flags = (api.il2cpp_field_get_flags)(field);
            if flags & 0x10 == 0 || flags & 0x40 != 0 {
                continue;
            }
            let typ = (api.il2cpp_field_get_type)(field);
            if typ.is_null() {
                continue;
            }
            let layout = values::layout(api, typ)?;
            if layout.reference || layout.inline != 0 {
                inventory.potential_root_owners += 1;
                // The ordinary loader still validates the whole layout and the
                // actual static storage. Eligibility confers no read authority.
                return Ok(true);
            }
        }
    }
}

pub(super) fn report(c: &Catalog, start: Instant) -> Value {
    json!({"policy":"roots_then_encountered_layouts_v1","root_inventory":c.root_inventory,
        "loaded_classes":c.classes.len(),"loaded_fields":c.field_count,"static_roots":c.roots.len(),
        "elapsed_ms":start.elapsed().as_millis() as u64,
        "limits":{"declared_classes":MAX_CLASSES,"root_field_headers":MAX_ROOT_HEADERS,
            "loaded_classes":MAX_CLASSES,"loaded_fields":MAX_FIELDS,"elapsed_ms":20000},
        "limitation":"Root eligibility covers declared non-open-generic types. Complete layouts are loaded for eligible root owners and encountered types, parents and inline dependencies; other layouts are deferred. Root inventory completion is not full object-graph coverage."})
}
