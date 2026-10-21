//! Follow a previously observed role/path through a fresh paused snapshot.
use super::*;
use reversebolt_core::runtime::{MAX_PATH_STEPS, Member};

#[derive(Clone, Copy, serde::Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Status {
    #[default]
    Budget,
    Read,
    RootUnavailable,
    RootAmbiguous,
    NullReference,
    Unreadable,
    UnknownType,
    TypeChanged,
    FieldUnavailable,
    ArrayBounds,
    UnsupportedArray,
    NotReference,
    NotScalar,
    MetadataUnavailable,
}

#[derive(Clone, Copy, Default)]
struct Evaluation {
    status: Status,
    step: usize,
    unknown_class: usize,
    bytes: [u8; 8],
    width: usize,
    kind: i32,
    address: usize,
    visited: [usize; MAX_PATH_STEPS],
    visited_count: usize,
}

#[derive(Clone, Copy)]
struct RootSlot {
    address: usize,
    status: Status,
}

fn root_slot(c: &Catalog, path: &Locator) -> RootSlot {
    let mut found = None;
    for root in &c.roots {
        if c.classes[root.class].key == path.root.declaring && root.field == path.root.field_id {
            if found.is_some() {
                return RootSlot {
                    address: 0,
                    status: Status::RootAmbiguous,
                };
            }
            found = Some(root.address);
        }
    }
    RootSlot {
        address: found.unwrap_or(0),
        status: if found.is_some() {
            Status::Read
        } else {
            Status::RootUnavailable
        },
    }
}

// No allocation, metadata lookup API, serialization or formatting in this function.
fn follow(api: &Api, c: &Catalog, path: &Locator, root: RootSlot, stopped: Instant) -> Evaluation {
    let mut result = Evaluation {
        status: root.status,
        ..Evaluation::default()
    };
    if root.status != Status::Read {
        return result;
    }
    let mut pointer = [0; 8];
    if !read_into(root.address, &mut pointer) {
        result.status = Status::Unreadable;
        return result;
    }
    let mut address = usize::from_le_bytes(pointer);
    for (index, step) in path.steps.iter().enumerate() {
        result.step = index;
        if stopped.elapsed() >= PAUSE_BUDGET {
            result.status = Status::Budget;
            return result;
        }
        if address == 0 {
            result.status = Status::NullReference;
            return result;
        }
        let mut header = [0; 128];
        if !read_into(address, &mut header[..c.header]) {
            result.status = Status::Unreadable;
            return result;
        }
        result.visited[index] = address;
        result.visited_count = index + 1;
        let runtime_class = unsafe { (api.il2cpp_object_get_class)(address as Ptr) } as usize;
        let Some(&class_id) = c.lookup.get(&runtime_class) else {
            result.unknown_class = runtime_class;
            result.status = Status::UnknownType;
            return result;
        };
        let class = &c.classes[class_id];
        if class.key != step.owner {
            result.status = Status::TypeChanged;
            return result;
        }
        let (offset, width, kind, reference) = match &step.member {
            Member::Field { field } => {
                let mut found = None;
                for &(declaring, field_id) in &class.flat {
                    if field_id == field.field_id && c.classes[declaring].key == field.declaring {
                        if found.is_some() {
                            result.status = Status::FieldUnavailable;
                            return result;
                        }
                        found = Some(&c.classes[declaring].fields[field_id]);
                    }
                }
                let Some(field) = found else {
                    result.status = Status::FieldUnavailable;
                    return result;
                };
                (field.offset, field.width, field.kind, field.reference)
            }
            Member::Element { index } => {
                let Shape::Array {
                    stride,
                    kind,
                    reference,
                } = class.shape
                else {
                    result.status = Status::UnsupportedArray;
                    return result;
                };
                if *index >= unsafe { (api.il2cpp_array_length)(address as Ptr) } as usize {
                    result.status = Status::ArrayBounds;
                    return result;
                }
                let Some(offset) = index
                    .checked_mul(stride)
                    .and_then(|v| v.checked_add(c.array_header))
                else {
                    result.status = Status::ArrayBounds;
                    return result;
                };
                (offset, stride, kind, reference)
            }
        };
        let terminal = index + 1 == path.steps.len();
        if terminal && reference {
            result.status = Status::NotScalar;
            return result;
        }
        if !terminal && !reference {
            result.status = Status::NotReference;
            return result;
        }
        let Some(field_address) = address.checked_add(offset) else {
            result.status = Status::Unreadable;
            return result;
        };
        if width == 0 || width > 8 || !read_into(field_address, &mut result.bytes[..width]) {
            result.status = Status::Unreadable;
            return result;
        }
        if terminal {
            result.width = width;
            result.kind = kind;
            result.address = field_address;
            result.status = Status::Read;
            return result;
        }
        address = usize::from_le_bytes(result.bytes);
    }
    result.status = Status::NotScalar;
    result
}

pub(super) fn refresh(
    api: &Api,
    c: &mut Catalog,
    paths: &[Locator],
    scope: &Scope,
    start: Instant,
    catalog_ms: u64,
) -> Result<Value> {
    let roots: Vec<_> = paths.iter().map(|p| root_slot(c, p)).collect();
    let mut results = vec![Evaluation::default(); paths.len()];
    let mut pauses = Vec::new();
    let mut metadata_error = None;
    let mut added_classes = 0;
    for attempt in 0..=MAX_PATH_STEPS {
        if start.elapsed() > Duration::from_secs(18) {
            for result in &mut results {
                result.status = Status::Budget;
            }
            break;
        }
        // Touch preallocated results and warm kernel/time calls before stopping.
        results.fill(Evaluation::default());
        let mut warm = [0; 8];
        let _ = read_into(c as *const Catalog as usize, &mut warm);
        let before = Instant::now();
        let allocations = PAUSED_ALLOCS.load(Ordering::Relaxed);
        let frees = PAUSED_FREES.load(Ordering::Relaxed);
        unsafe { (api.il2cpp_stop_gc_world)() };
        let guard = Stopped(api.il2cpp_start_gc_world);
        PAUSED.store(true, Ordering::Relaxed);
        let stopped = Instant::now();
        for ((path, root), result) in paths.iter().zip(&roots).zip(&mut results) {
            *result = follow(api, c, path, *root, stopped);
        }
        let held_us = stopped.elapsed().as_micros() as u64;
        drop(guard);
        let total_us = before.elapsed().as_micros() as u64;
        pauses.push(
            json!({"attempt":attempt,"pause_held_us":held_us,"stop_to_resume_us":total_us,
            "helper_allocations_while_paused":PAUSED_ALLOCS.load(Ordering::Relaxed)-allocations,
            "helper_deallocations_while_paused":PAUSED_FREES.load(Ordering::Relaxed)-frees}),
        );
        let mut unknown: Vec<_> = results
            .iter()
            .filter(|r| r.status == Status::UnknownType)
            .map(|r| r.unknown_class)
            .collect();
        unknown.sort_unstable();
        unknown.dedup();
        if unknown.is_empty() || attempt == MAX_PATH_STEPS {
            break;
        }
        match prepare_encountered(api, c, &unknown, start) {
            Ok(0) => break,
            Ok(count) => added_classes += count,
            Err(e) => {
                metadata_error = Some(e);
                for result in &mut results {
                    if result.status == Status::UnknownType {
                        result.status = Status::MetadataUnavailable;
                    }
                }
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let checks: Vec<_> = paths.iter().zip(&results).map(|(path, result)| {
        let value = if result.status == Status::Read {
            reversebolt_core::runtime::scalar_value(result.kind, &result.bytes[..result.width]).unwrap_or(Value::Null)
        } else { Value::Null };
        json!({"path_id":path.id(),"status":result.status,"step":result.step,"type_code":result.kind,
            "bytes":if result.status == Status::Read { &result.bytes[..result.width] } else { &[] },
            "value":value,"field_address":if result.status == Status::Read { Some(format!("0x{:x}",result.address)) } else { None },
            "snapshot_object_addresses":result.visited[..result.visited_count].iter().map(|a|format!("0x{a:x}")).collect::<Vec<_>>()})
    }).collect();
    Ok(
        json!({"schema":1,"operation":"refresh_runtime_paths","scope":scope,"pid":scope.pid,"created":scope.created,"session":scope.session,"checks":checks,"pauses":pauses,
        "catalog_ms":catalog_ms,"classes_added":added_classes,"metadata_error":metadata_error,
        "elapsed_ms":start.elapsed().as_millis() as u64,"edit_authority":false,
        "identity_contract":"values belong to current path occupants in the final paused snapshot; no persistent object identity or edit authority"}),
    )
}
