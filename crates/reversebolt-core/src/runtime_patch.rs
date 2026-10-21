//! Explicit numeric trials against freshly resolved runtime paths. Address equality
//! is a best-effort guard, not persistent identity or a complete gameplay rollback.
use crate::{
    Result,
    runtime::{Locator, Scope, scalar_value},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const LIFETIME_MS: u128 = 60_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub path: Locator,
    pub type_code: i32,
    pub expected: Vec<u8>,
    pub replacement: Vec<u8>,
    pub field_address: u64,
    pub object_addresses: Vec<u64>,
    pub prepared_unix_ms: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Guard {
    Ready,
    Expired,
    RepresentationChanged,
    PathMoved,
    ValueChanged,
}

fn address(value: &Value) -> Result<u64> {
    value
        .as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u64::from_str_radix(s, 16).ok())
        .filter(|a| *a != 0)
        .ok_or_else(|| "Missing runtime address guard.".into())
}

/// Parse integers without a float conversion, including exact 64-bit limits.
pub fn encode(kind: i32, text: &str) -> Result<Vec<u8>> {
    if text.is_empty() || text.len() > 64 || text.trim() != text {
        return Err("Enter one finite number without surrounding whitespace.".into());
    }
    macro_rules! integer {
        ($ty:ty) => {
            text.parse::<$ty>()
                .map(|v| v.to_le_bytes().to_vec())
                .map_err(|_| "Value does not fit the runtime integer type.".into())
        };
    }
    match kind {
        4 => integer!(i8),
        5 => integer!(u8),
        6 => integer!(i16),
        7 => integer!(u16),
        8 => integer!(i32),
        9 => integer!(u32),
        10 => integer!(i64),
        11 => integer!(u64),
        12 => text
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| v.to_le_bytes().to_vec())
            .ok_or_else(|| "Enter a finite 32-bit floating-point value.".into()),
        13 => text
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| v.to_le_bytes().to_vec())
            .ok_or_else(|| "Enter a finite 64-bit floating-point value.".into()),
        _ => Err("Runtime trials currently support numeric scalars only.".into()),
    }
}

fn bounded_difference(kind: i32, a: &[u8], b: &[u8]) -> Result<()> {
    let before = scalar_value(kind, a)?;
    let after = scalar_value(kind, b)?;
    let value_text = |v: Value| {
        v.as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| v.to_string())
    };
    let within = match kind {
        4..=11 => {
            let before: i128 = value_text(before)
                .parse()
                .map_err(|_| "Invalid integer bytes.")?;
            let after: i128 = value_text(after)
                .parse()
                .map_err(|_| "Invalid integer bytes.")?;
            let delta = (after - before).abs();
            delta > 0 && delta <= 20
        }
        12 | 13 => {
            let before: f64 = value_text(before)
                .parse()
                .map_err(|_| "Invalid float bytes.")?;
            let after: f64 = value_text(after)
                .parse()
                .map_err(|_| "Invalid float bytes.")?;
            let delta = (after - before).abs();
            before.is_finite() && after.is_finite() && delta > 0.0 && delta <= 20.0
        }
        _ => false,
    };
    if !within {
        return Err("Trial must change a finite numeric value by at most 20.".into());
    }
    Ok(())
}

impl Patch {
    pub fn prepare(path: Locator, check: &Value, desired: &str, now: u128) -> Result<Self> {
        if check["path_id"] != path.id() || check["status"] != "read" {
            return Err("A successful fresh read of this path is required.".into());
        }
        let type_code = check["type_code"]
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .ok_or("Missing scalar type.")?;
        let expected: Vec<u8> =
            serde_json::from_value(check["bytes"].clone()).map_err(|_| "Invalid bytes.")?;
        if scalar_value(type_code, &expected)? != check["value"] {
            return Err("Runtime value contradicts its bytes.".into());
        }
        let patch = Self {
            path,
            type_code,
            expected,
            replacement: encode(type_code, desired)?,
            field_address: address(&check["field_address"])?,
            object_addresses: check["snapshot_object_addresses"]
                .as_array()
                .ok_or("Missing reference chain.")?
                .iter()
                .map(address)
                .collect::<Result<_>>()?,
            prepared_unix_ms: now,
        };
        patch.validate(&patch.path.scope, now)?;
        Ok(patch)
    }

    pub fn validate(&self, scope: &Scope, now: u128) -> Result<()> {
        self.path.validate(scope)?;
        if now < self.prepared_unix_ms || now - self.prepared_unix_ms > LIFETIME_MS {
            return Err("Runtime trial expired or its clock moved backwards.".into());
        }
        if self.object_addresses.len() != self.path.steps.len()
            || self.object_addresses.contains(&0)
            || self.field_address == 0
        {
            return Err("Invalid runtime reference-chain guard.".into());
        }
        bounded_difference(self.type_code, &self.expected, &self.replacement)
    }

    pub fn id(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("serializable patch"))
        )
    }

    /// Allocation-free decision for use inside a bounded managed-world pause.
    pub fn check(
        &self,
        kind: i32,
        address: u64,
        objects: &[usize],
        bytes: &[u8],
        now: u128,
    ) -> Guard {
        if now < self.prepared_unix_ms || now - self.prepared_unix_ms > LIFETIME_MS {
            return Guard::Expired;
        }
        if kind != self.type_code || bytes.len() != self.expected.len() {
            return Guard::RepresentationChanged;
        }
        if address != self.field_address
            || objects.len() != self.object_addresses.len()
            || objects
                .iter()
                .zip(&self.object_addresses)
                .any(|(a, b)| *a as u64 != *b)
        {
            return Guard::PathMoved;
        }
        if bytes != self.expected {
            return Guard::ValueChanged;
        }
        Guard::Ready
    }

    pub fn restoration(&self, report: &Value, now: u128) -> Result<Self> {
        let checks = report["checks"]
            .as_array()
            .ok_or("Missing patch observation.")?;
        if checks.len() != 1 {
            return Err("Restoration requires one exact patch observation.".into());
        }
        let check = &checks[0];
        let objects = check["snapshot_object_addresses"]
            .as_array()
            .ok_or("Missing observed reference chain.")?
            .iter()
            .map(address)
            .collect::<Result<Vec<_>>>()?;
        if report["operation"] != "patch_runtime_path"
            || report["patch_id"] != self.id()
            || report["scope"] != serde_json::to_value(&self.path.scope).unwrap()
            || report["patch"]["state"] != "written"
            || report["patch"]["attempted"] != true
            || report["patch"]["guard"] != "ready"
            || report["patch"]["transferred"].as_u64() != Some(self.replacement.len() as u64)
            || report["patch"]["observed_after"] != serde_json::json!(self.replacement)
            || check["status"] != "read"
            || check["path_id"] != self.path.id()
            || check["type_code"] != self.type_code
            || check["bytes"] != serde_json::json!(self.expected)
            || check["value"] != scalar_value(self.type_code, &self.expected)?
            || address(&check["field_address"])? != self.field_address
            || objects != self.object_addresses
        {
            return Err("Restoration needs this patch's confirmed write receipt.".into());
        }
        let mut inverse = self.clone();
        std::mem::swap(&mut inverse.expected, &mut inverse.replacement);
        inverse.prepared_unix_ms = now;
        inverse.validate(&self.path.scope, now)?;
        Ok(inverse)
    }
}
