// Terminal UI implementation using ratatui
// The pretty face of RepoScout

pub mod app;
pub mod code_ui;
pub mod discovery_ui;
pub mod help_ui;
pub mod portfolio_ui;
pub mod runner;
pub mod sparkline;
pub mod theme_ui;
pub mod ui;

pub use app::{
    App, CodePreviewMode, DiscoveryCategory, InputMode, PlatformStatus, PreviewMode, SearchMode,
};
pub use runner::run_tui;
