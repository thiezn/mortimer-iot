/// Axum daemon runtime and route wiring.
pub mod daemon;

/// Shared app state and daemon run entrypoint.
pub use daemon::{AppState, run};
