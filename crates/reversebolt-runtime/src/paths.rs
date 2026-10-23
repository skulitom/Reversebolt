//! Follow a previously observed role/path through a fresh paused snapshot.
use super::*;
use reversebolt_core::runtime::{MAX_PATH_STEPS, Member};
use reversebolt_core::runtime_patch::{Guard, Patch};
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

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
    writable_field: bool,
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
        let (offset, width, kind, reference, writable) = match &step.member {
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
                (
                    field.offset,
                    field.width,
                    field.kind,
                    field.reference,
                    field.flags & 0x70 == 0
                        && field.offset >= c.header
                        && field
                            .offset
                            .checked_add(field.width)
                            .is_some_and(|end| end <= class.size),
                )
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
                (offset, stride, kind, reference, true)
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
            result.writable_field = writable;
            result.status = Status::Read;
            return result;
        }
        address = usize::from_le_bytes(result.bytes);
    }
    result.status = Status::NotScalar;
    result
}

#[derive(Clone, Copy, Default)]
struct WriteResult {
    attempted: bool,
    transferred: usize,
    after: [u8; 8],
    after_ok: bool,
    confirmed: bool,
    reason: &'static str,
    guard: Option<Guard>,
}

// No allocation or formatting while the managed world is paused. Path resolution,
// expected-byte comparison and the single write share this same pause.
fn write_patch(patch: &Patch, value: &Evaluation, stopped: Instant) -> WriteResult {
    use windows_sys::Win32::System::{Diagnostics::Debug::WriteProcessMemory, Memory::*};
    let mut result = WriteResult {
        reason: "path_unavailable",
        ..Default::default()
    };
    if value.status != Status::Read {
        return result;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let guard = patch.check(
        value.kind,
        value.address as u64,
        &value.visited[..value.visited_count],
        &value.bytes[..value.width],
        now,
    );
    result.guard = Some(guard);
    if guard != Guard::Ready {
        result.reason = "guard_refused";
        return result;
    }
    if !value.writable_field {
        result.reason = "readonly_field";
        return result;
    }
    let mut region: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    let queried = unsafe {
        VirtualQuery(
            value.address as *const c_void,
            &mut region,
            std::mem::size_of_val(&region),
        )
    };
    let end = value.address.checked_add(value.width);
    let region_end = (region.BaseAddress as usize).checked_add(region.RegionSize);
    if queried != std::mem::size_of_val(&region)
        || region.State != MEM_COMMIT
        || !matches!(region.Protect & 0xff, PAGE_READWRITE | PAGE_WRITECOPY)
        || region.Protect & (PAGE_GUARD | PAGE_NOACCESS) != 0
        || !matches!((end, region_end), (Some(a), Some(b)) if value.address >= region.BaseAddress as usize && a <= b)
    {
        result.reason = "page_not_writable";
        return result;
    }
    if stopped.elapsed() >= PAUSE_BUDGET {
        result.reason = "pause_budget";
        return result;
    }
    result.attempted = true;
    let written = unsafe {
        WriteProcessMemory(
            GetCurrentProcess(),
            value.address as *mut c_void,
            patch.replacement.as_ptr().cast(),
            value.width,
            &mut result.transferred,
        )
    };
    result.after_ok = read_into(value.address, &mut result.after[..value.width]);
    result.confirmed = written != 0
        && result.transferred == value.width
        && result.after_ok
        && result.after[..value.width] == patch.replacement;
    result.reason = if result.confirmed {
        "bytes_confirmed"
    } else {
        "write_outcome_uncertain"
    };
    result
}

pub(super) fn refresh(
    api: &Api,
    c: &mut Catalog,
    paths: &[Locator],
    scope: &Scope,
    start: Instant,
    catalog_ms: u64,
    patch: Option<&Patch>,
) -> Result<Value> {
    let roots: Vec<_> = paths.iter().map(|p| root_slot(c, p)).collect();
    let mut results = vec![Evaluation::default(); paths.len()];
    let mut pauses = Vec::new();
    let mut metadata_error = None;
    let mut added_classes = 0;
    let mut write = WriteResult {
        reason: "path_unavailable",
        ..Default::default()
    };
    let mut frequency = 0i64;
    let frequency_ok = unsafe { QueryPerformanceFrequency(&mut frequency) } != 0 && frequency > 0;
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
        let mut begin_ticks = 0i64;
        let clock_begin_ok = unsafe { QueryPerformanceCounter(&mut begin_ticks) } != 0;
        unsafe { (api.il2cpp_stop_gc_world)() };
        let guard = Stopped(api.il2cpp_start_gc_world);
        PAUSED.store(true, Ordering::Relaxed);
        let stopped = Instant::now();
        for ((path, root), result) in paths.iter().zip(&roots).zip(&mut results) {
            *result = follow(api, c, path, *root, stopped);
        }
        if let Some(patch) = patch {
            write = write_patch(patch, &results[0], stopped);
        }
        let held_us = stopped.elapsed().as_micros() as u64;
        drop(guard);
        let mut end_ticks = 0i64;
        let clock_end_ok = unsafe { QueryPerformanceCounter(&mut end_ticks) } != 0;
        let total_us = before.elapsed().as_micros() as u64;
        // Both clock calls are outside the managed pause. Bounds include the
        // entire observation; they do not identify a gameplay event instant.
        let sampling = if frequency_ok
            && clock_begin_ok
            && clock_end_ok
            && begin_ticks >= 0
            && end_ticks >= begin_ticks
        {
            json!({"frequency_hz":frequency,"begin_ticks":begin_ticks,"end_ticks":end_ticks})
        } else {
            Value::Null
        };
        pauses.push(
            json!({"attempt":attempt,"pause_held_us":held_us,"stop_to_resume_us":total_us,
            "sampling_qpc":sampling,
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
    let mut report = json!({"schema":1,"operation":"refresh_runtime_paths","scope":scope,"pid":scope.pid,"created":scope.created,"session":scope.session,"checks":checks,"pauses":pauses,
        "catalog_ms":catalog_ms,"classes_added":added_classes,"metadata_error":metadata_error,
        "elapsed_ms":start.elapsed().as_millis() as u64,"edit_authority":false,
        "identity_contract":"values belong to current path occupants in the final paused snapshot; no persistent object identity or edit authority"});
    if let Some(patch) = patch {
        report["operation"] = json!("patch_runtime_path");
        report["patch_id"] = json!(patch.id());
        report["patch"] = json!({"state":if write.confirmed {"written"} else if write.attempted {"uncertain"} else {"refused"},
            "attempted":write.attempted,"transferred":write.transferred,"reason":write.reason,"guard":write.guard,
            "observed_after":if write.after_ok {Some(&write.after[..results[0].width])} else {None}});
        report["identity_contract"] = json!(
            "fresh path/type/address/byte guards in one managed pause; address reuse is not excluded; no persistent actor identity, verified gameplay effect or complete rollback"
        );
    }
    Ok(report)
}
