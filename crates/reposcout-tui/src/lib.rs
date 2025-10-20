// Terminal UI implementation using ratatui
// The pretty face of RepoScout

pub mod app;
pub mod runner;
pub mod ui;

pub use app::{App, InputMode, PreviewMode};
pub use runner::run_tui;
