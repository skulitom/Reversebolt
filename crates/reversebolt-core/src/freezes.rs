//! A live hold is explicit and never resumes after restart. Each attempted write
//! retains its intent before execution; unknown outcomes keep the process fenced.
use crate::{
    Result,
    runtime::{Locator, Scope, scalar_value},
    runtime_patch::{Patch, encode},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub field: String,
    pub objects: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Freeze {
    pub id: String,
    pub actor: String,
    pub investigation: String,
    pub catalog: String,
    pub requested_revision: u64,
    pub candidate: String,
    pub value: String,
    pub kind: i32,
    pub path: Locator,
    pub identity: Option<Identity>,
    pub phase: String,
    pub reason: String,
    pub checks: u64,
    pub writes: u64,
    pub checked_ms: u128,
    pub intent: Option<Patch>,
    pub evidence: Option<String>,
}
impl Freeze {
    pub fn active(&self) -> bool {
        matches!(self.phase.as_str(), "active" | "writing")
    }
    pub fn holds(&self, scope: &Scope) -> bool {
        self.path.scope == *scope && matches!(self.phase.as_str(), "writing" | "uncertain")
    }
    pub fn stop(&mut self, reason: &str) {
        if self.phase == "active" {
            self.phase = "stopped".into();
            self.reason = reason.into();
        }
    }
    pub fn recover(&mut self) {
        if self.phase == "writing" {
            self.phase = "uncertain".into();
            self.reason =
                "A freeze write was interrupted; its outcome is unknown. It will not be replayed."
                    .into();
        } else {
            self.stop("Freeze ended when Reversebolt restarted.");
        }
    }
    pub fn check(&mut self, check: &Value, now: u128) -> Result<Option<Patch>> {
        if self.phase != "active"
            || check["status"] != "read"
            || check["path_id"] != self.candidate
            || check["type_code"] != self.kind
        {
            return Err("The frozen value is no longer available.".into());
        }
        let bytes: Vec<u8> = serde_json::from_value(check["bytes"].clone())
            .map_err(|_| "Invalid freeze reading.")?;
        if scalar_value(self.kind, &bytes)? != check["value"] {
            return Err("Freeze reading disagrees with its bytes.".into());
        }
        let address = |v: &Value| -> Result<String> {
            let s = v.as_str().ok_or("Missing freeze address.")?;
            if s.strip_prefix("0x")
                .and_then(|s| u64::from_str_radix(s, 16).ok())
                .is_none_or(|a| a == 0)
            {
                return Err("Invalid freeze address.".into());
            }
            Ok(s.to_lowercase())
        };
        let identity = Identity {
            field: address(&check["field_address"])?,
            objects: check["snapshot_object_addresses"]
                .as_array()
                .ok_or("Missing freeze references.")?
                .iter()
                .map(address)
                .collect::<Result<_>>()?,
        };
        if identity.objects.len() != self.path.steps.len()
            || self
                .identity
                .as_ref()
                .is_some_and(|old| old.field != identity.field || old.objects != identity.objects)
        {
            return Err("The value's object changed. Freeze stopped before writing.".into());
        }
        self.identity = Some(identity);
        self.checks = self
            .checks
            .checked_add(1)
            .ok_or("Freeze count limit reached.")?;
        self.checked_ms = now;
        let replacement = encode(self.kind, &self.value)?;
        if bytes == replacement {
            return Ok(None);
        }
        Patch::prepare(self.path.clone(), check, &self.value, now).map(Some)
    }
}
