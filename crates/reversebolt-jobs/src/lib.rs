//! A single-owner state machine, not a service, thread pool, UI or MCP server.
pub mod adapter;
pub mod fake;
pub mod journal;
mod machine;
pub mod model;
pub use machine::*;
pub use model::*;
