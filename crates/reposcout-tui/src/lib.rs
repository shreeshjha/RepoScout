// Terminal UI implementation using ratatui
// The pretty face of RepoScout

pub mod app;
pub mod runner;
pub mod ui;
pub mod sparkline;

pub use app::{App, InputMode, PreviewMode, SearchMode, PlatformStatus};
pub use runner::run_tui;
