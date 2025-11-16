// Terminal UI implementation using ratatui
// The pretty face of RepoScout

pub mod app;
pub mod runner;
pub mod ui;
pub mod sparkline;
pub mod code_ui;
pub mod portfolio_ui;
pub mod theme_ui;
pub mod discovery_ui;

pub use app::{App, CodePreviewMode, InputMode, PreviewMode, SearchMode, PlatformStatus, DiscoveryCategory};
pub use runner::run_tui;
