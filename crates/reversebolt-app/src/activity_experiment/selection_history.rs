//! Small immutable proposal markers outlive bounded job-history eviction.
use super::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reservation {
    binding: String,
    key: u8,
    experiment: String,
}
fn directory(base: &Path, binding: &str) -> PathBuf {
    base.join("activity-selection")
        .join(visual::digest(binding.as_bytes()))
}
fn checked(base: &Path, path: &Path, binding: &str, key: u8) -> Result<()> {
    let reserved: Reservation = serde_json::from_slice(&read(path)?).map_err(|e| e.to_string())?;
    let plan = load(base, &reserved.experiment)?;
    if reserved.binding != binding
        || reserved.key != key
        || plan.binding() != binding
        || plan.key != key
    {
        return Err("Input proposal history differs from its saved plan".into());
    }
    Ok(())
}
pub(crate) fn keys(base: &Path, binding: &str) -> Result<Vec<u8>> {
    let directory = directory(base, binding);
    let mut keys = vec![];
    for key in 1..=254u8 {
        let path = directory.join(format!("{key:03}.json"));
        if path.try_exists().map_err(|e| e.to_string())? {
            checked(base, &path, binding, key)?;
            keys.push(key);
        }
    }
    Ok(keys)
}
pub(crate) fn reserve(base: &Path, plan: &Plan, reference: &str) -> Result<()> {
    let binding = plan.binding();
    let directory = directory(base, &binding);
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let path = directory.join(format!("{:03}.json", plan.key));
    if path.try_exists().map_err(|e| e.to_string())? {
        return checked(base, &path, &binding, plan.key);
    }
    // Reserve before job admission. Even an interrupted/failed admission is not
    // silently retried by automatic selection; an explicit key remains possible.
    save(
        &path,
        &Reservation {
            binding,
            key: plan.key,
            experiment: reference.into(),
        },
    )
}
