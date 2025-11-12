use serde::{Deserialize, Serialize};

/// Color theme for the TUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

/// All color definitions for a theme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    // Base colors
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub border_focused: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // UI element colors
    pub title: Color,
    pub subtitle: Color,
    pub selected: Color,
    pub selected_bg: Color,
    pub tab_active: Color,
    pub tab_inactive: Color,

    // Data colors
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub muted: Color,

    // Specific element colors
    pub health_healthy: Color,
    pub health_moderate: Color,
    pub health_warning: Color,
    pub health_critical: Color,

    pub stars: Color,
    pub forks: Color,
    pub issues: Color,
    pub language: Color,
}

/// RGB color representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn rgb(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }
}

impl Theme {
    /// Get default dark theme
    pub fn default_dark() -> Self {
        Self {
            name: "Default Dark".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x1e1e2e),
                foreground: Color::rgb(0xcdd6f4),
                border: Color::rgb(0x45475a),
                border_focused: Color::rgb(0x89b4fa),

                success: Color::rgb(0xa6e3a1),
                warning: Color::rgb(0xf9e2af),
                error: Color::rgb(0xf38ba8),
                info: Color::rgb(0x89dceb),

                title: Color::rgb(0xcba6f7),
                subtitle: Color::rgb(0xa6adc8),
                selected: Color::rgb(0x89b4fa),
                selected_bg: Color::rgb(0x313244),
                tab_active: Color::rgb(0xf5c2e7),
                tab_inactive: Color::rgb(0x6c7086),

                primary: Color::rgb(0x89b4fa),
                secondary: Color::rgb(0xf5c2e7),
                accent: Color::rgb(0xf9e2af),
                muted: Color::rgb(0x6c7086),

                health_healthy: Color::rgb(0xa6e3a1),
                health_moderate: Color::rgb(0xf9e2af),
                health_warning: Color::rgb(0xfab387),
                health_critical: Color::rgb(0xf38ba8),

                stars: Color::rgb(0xf9e2af),
                forks: Color::rgb(0x94e2d5),
                issues: Color::rgb(0xf38ba8),
                language: Color::rgb(0xcba6f7),
            },
        }
    }

    /// Get light theme
    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0xeff1f5),
                foreground: Color::rgb(0x4c4f69),
                border: Color::rgb(0xbcc0cc),
                border_focused: Color::rgb(0x1e66f5),

                success: Color::rgb(0x40a02b),
                warning: Color::rgb(0xdf8e1d),
                error: Color::rgb(0xd20f39),
                info: Color::rgb(0x209fb5),

                title: Color::rgb(0x8839ef),
                subtitle: Color::rgb(0x6c6f85),
                selected: Color::rgb(0x1e66f5),
                selected_bg: Color::rgb(0xdce0e8),
                tab_active: Color::rgb(0xea76cb),
                tab_inactive: Color::rgb(0x9ca0b0),

                primary: Color::rgb(0x1e66f5),
                secondary: Color::rgb(0xea76cb),
                accent: Color::rgb(0xdf8e1d),
                muted: Color::rgb(0x9ca0b0),

                health_healthy: Color::rgb(0x40a02b),
                health_moderate: Color::rgb(0xdf8e1d),
                health_warning: Color::rgb(0xfe640b),
                health_critical: Color::rgb(0xd20f39),

                stars: Color::rgb(0xdf8e1d),
                forks: Color::rgb(0x04a5e5),
                issues: Color::rgb(0xd20f39),
                language: Color::rgb(0x8839ef),
            },
        }
    }

    /// Get Nord theme
    pub fn nord() -> Self {
        Self {
            name: "Nord".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x2e3440),
                foreground: Color::rgb(0xeceff4),
                border: Color::rgb(0x4c566a),
                border_focused: Color::rgb(0x88c0d0),

                success: Color::rgb(0xa3be8c),
                warning: Color::rgb(0xebcb8b),
                error: Color::rgb(0xbf616a),
                info: Color::rgb(0x81a1c1),

                title: Color::rgb(0xb48ead),
                subtitle: Color::rgb(0xd8dee9),
                selected: Color::rgb(0x88c0d0),
                selected_bg: Color::rgb(0x3b4252),
                tab_active: Color::rgb(0x8fbcbb),
                tab_inactive: Color::rgb(0x4c566a),

                primary: Color::rgb(0x88c0d0),
                secondary: Color::rgb(0xb48ead),
                accent: Color::rgb(0xebcb8b),
                muted: Color::rgb(0x4c566a),

                health_healthy: Color::rgb(0xa3be8c),
                health_moderate: Color::rgb(0xebcb8b),
                health_warning: Color::rgb(0xd08770),
                health_critical: Color::rgb(0xbf616a),

                stars: Color::rgb(0xebcb8b),
                forks: Color::rgb(0x8fbcbb),
                issues: Color::rgb(0xbf616a),
                language: Color::rgb(0xb48ead),
            },
        }
    }

    /// Get Dracula theme
    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x282a36),
                foreground: Color::rgb(0xf8f8f2),
                border: Color::rgb(0x44475a),
                border_focused: Color::rgb(0xbd93f9),

                success: Color::rgb(0x50fa7b),
                warning: Color::rgb(0xf1fa8c),
                error: Color::rgb(0xff5555),
                info: Color::rgb(0x8be9fd),

                title: Color::rgb(0xff79c6),
                subtitle: Color::rgb(0xf8f8f2),
                selected: Color::rgb(0xbd93f9),
                selected_bg: Color::rgb(0x44475a),
                tab_active: Color::rgb(0xff79c6),
                tab_inactive: Color::rgb(0x6272a4),

                primary: Color::rgb(0xbd93f9),
                secondary: Color::rgb(0xff79c6),
                accent: Color::rgb(0xf1fa8c),
                muted: Color::rgb(0x6272a4),

                health_healthy: Color::rgb(0x50fa7b),
                health_moderate: Color::rgb(0xf1fa8c),
                health_warning: Color::rgb(0xffb86c),
                health_critical: Color::rgb(0xff5555),

                stars: Color::rgb(0xf1fa8c),
                forks: Color::rgb(0x8be9fd),
                issues: Color::rgb(0xff5555),
                language: Color::rgb(0xbd93f9),
            },
        }
    }

    /// Get Gruvbox theme
    pub fn gruvbox() -> Self {
        Self {
            name: "Gruvbox Dark".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x282828),
                foreground: Color::rgb(0xebdbb2),
                border: Color::rgb(0x504945),
                border_focused: Color::rgb(0x83a598),

                success: Color::rgb(0xb8bb26),
                warning: Color::rgb(0xfabd2f),
                error: Color::rgb(0xfb4934),
                info: Color::rgb(0x83a598),

                title: Color::rgb(0xd3869b),
                subtitle: Color::rgb(0xa89984),
                selected: Color::rgb(0x83a598),
                selected_bg: Color::rgb(0x3c3836),
                tab_active: Color::rgb(0xd3869b),
                tab_inactive: Color::rgb(0x665c54),

                primary: Color::rgb(0x83a598),
                secondary: Color::rgb(0xd3869b),
                accent: Color::rgb(0xfabd2f),
                muted: Color::rgb(0x665c54),

                health_healthy: Color::rgb(0xb8bb26),
                health_moderate: Color::rgb(0xfabd2f),
                health_warning: Color::rgb(0xfe8019),
                health_critical: Color::rgb(0xfb4934),

                stars: Color::rgb(0xfabd2f),
                forks: Color::rgb(0x8ec07c),
                issues: Color::rgb(0xfb4934),
                language: Color::rgb(0xd3869b),
            },
        }
    }

    /// Get all available themes
    pub fn all_themes() -> Vec<Theme> {
        vec![
            Self::default_dark(),
            Self::light(),
            Self::nord(),
            Self::dracula(),
            Self::gruvbox(),
        ]
    }

    /// Get theme by name
    pub fn by_name(name: &str) -> Option<Theme> {
        Self::all_themes()
            .into_iter()
            .find(|t| t.name.to_lowercase() == name.to_lowercase())
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_dark()
    }
}
