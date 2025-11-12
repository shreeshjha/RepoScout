// Terminal UI implementation using ratatui
// The pretty face of RepoScout

pub mod app;
pub mod runner;
pub mod ui;
pub mod sparkline;
pub mod code_ui;
pub mod portfolio_ui;

pub use app::{App, CodePreviewMode, InputMode, PreviewMode, SearchMode, PlatformStatus};
pub use runner::run_tui;
