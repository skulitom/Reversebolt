pub mod action_patterns;
pub mod changes;
pub mod coverage;
pub mod discovery;
pub mod effects;
pub mod engine;
pub mod inputs;
pub mod intervention;
pub mod investigation;
pub mod live;
pub mod prediction;
pub mod probes;
pub mod references;
pub mod rule_checks;
pub mod temporal;
pub mod visual;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Command {
    pub action: String,
    #[serde(default)]
    pub args: Value,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "desktop".into()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub url: String,
    pub token: String,
    pub pid: u32,
}

pub fn data_dir() -> PathBuf {
    std::env::var_os("REVERSEBOLT_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()))
                .join("Reversebolt")
        })
}
