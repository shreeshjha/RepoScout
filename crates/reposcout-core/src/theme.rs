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

    /// Get Solarized Dark theme
    pub fn solarized_dark() -> Self {
        Self {
            name: "Solarized Dark".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x002b36),
                foreground: Color::rgb(0x839496),
                border: Color::rgb(0x073642),
                border_focused: Color::rgb(0x268bd2),

                success: Color::rgb(0x859900),
                warning: Color::rgb(0xb58900),
                error: Color::rgb(0xdc322f),
                info: Color::rgb(0x2aa198),

                title: Color::rgb(0x6c71c4),
                subtitle: Color::rgb(0x93a1a1),
                selected: Color::rgb(0x268bd2),
                selected_bg: Color::rgb(0x073642),
                tab_active: Color::rgb(0xd33682),
                tab_inactive: Color::rgb(0x586e75),

                primary: Color::rgb(0x268bd2),
                secondary: Color::rgb(0xd33682),
                accent: Color::rgb(0xb58900),
                muted: Color::rgb(0x586e75),

                health_healthy: Color::rgb(0x859900),
                health_moderate: Color::rgb(0xb58900),
                health_warning: Color::rgb(0xcb4b16),
                health_critical: Color::rgb(0xdc322f),

                stars: Color::rgb(0xb58900),
                forks: Color::rgb(0x2aa198),
                issues: Color::rgb(0xdc322f),
                language: Color::rgb(0x6c71c4),
            },
        }
    }

    /// Get Solarized Light theme
    pub fn solarized_light() -> Self {
        Self {
            name: "Solarized Light".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0xfdf6e3),
                foreground: Color::rgb(0x657b83),
                border: Color::rgb(0xeee8d5),
                border_focused: Color::rgb(0x268bd2),

                success: Color::rgb(0x859900),
                warning: Color::rgb(0xb58900),
                error: Color::rgb(0xdc322f),
                info: Color::rgb(0x2aa198),

                title: Color::rgb(0x6c71c4),
                subtitle: Color::rgb(0x586e75),
                selected: Color::rgb(0x268bd2),
                selected_bg: Color::rgb(0xeee8d5),
                tab_active: Color::rgb(0xd33682),
                tab_inactive: Color::rgb(0x93a1a1),

                primary: Color::rgb(0x268bd2),
                secondary: Color::rgb(0xd33682),
                accent: Color::rgb(0xb58900),
                muted: Color::rgb(0x93a1a1),

                health_healthy: Color::rgb(0x859900),
                health_moderate: Color::rgb(0xb58900),
                health_warning: Color::rgb(0xcb4b16),
                health_critical: Color::rgb(0xdc322f),

                stars: Color::rgb(0xb58900),
                forks: Color::rgb(0x2aa198),
                issues: Color::rgb(0xdc322f),
                language: Color::rgb(0x6c71c4),
            },
        }
    }

    /// Get One Dark theme
    pub fn one_dark() -> Self {
        Self {
            name: "One Dark".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x282c34),
                foreground: Color::rgb(0xabb2bf),
                border: Color::rgb(0x3e4451),
                border_focused: Color::rgb(0x61afef),

                success: Color::rgb(0x98c379),
                warning: Color::rgb(0xe5c07b),
                error: Color::rgb(0xe06c75),
                info: Color::rgb(0x56b6c2),

                title: Color::rgb(0xc678dd),
                subtitle: Color::rgb(0x5c6370),
                selected: Color::rgb(0x61afef),
                selected_bg: Color::rgb(0x3e4451),
                tab_active: Color::rgb(0xc678dd),
                tab_inactive: Color::rgb(0x5c6370),

                primary: Color::rgb(0x61afef),
                secondary: Color::rgb(0xc678dd),
                accent: Color::rgb(0xe5c07b),
                muted: Color::rgb(0x5c6370),

                health_healthy: Color::rgb(0x98c379),
                health_moderate: Color::rgb(0xe5c07b),
                health_warning: Color::rgb(0xd19a66),
                health_critical: Color::rgb(0xe06c75),

                stars: Color::rgb(0xe5c07b),
                forks: Color::rgb(0x56b6c2),
                issues: Color::rgb(0xe06c75),
                language: Color::rgb(0xc678dd),
            },
        }
    }

    /// Get Tokyo Night theme
    pub fn tokyo_night() -> Self {
        Self {
            name: "Tokyo Night".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x1a1b26),
                foreground: Color::rgb(0xa9b1d6),
                border: Color::rgb(0x414868),
                border_focused: Color::rgb(0x7aa2f7),

                success: Color::rgb(0x9ece6a),
                warning: Color::rgb(0xe0af68),
                error: Color::rgb(0xf7768e),
                info: Color::rgb(0x7dcfff),

                title: Color::rgb(0xbb9af7),
                subtitle: Color::rgb(0x565f89),
                selected: Color::rgb(0x7aa2f7),
                selected_bg: Color::rgb(0x24283b),
                tab_active: Color::rgb(0xff9e64),
                tab_inactive: Color::rgb(0x565f89),

                primary: Color::rgb(0x7aa2f7),
                secondary: Color::rgb(0xbb9af7),
                accent: Color::rgb(0xe0af68),
                muted: Color::rgb(0x565f89),

                health_healthy: Color::rgb(0x9ece6a),
                health_moderate: Color::rgb(0xe0af68),
                health_warning: Color::rgb(0xff9e64),
                health_critical: Color::rgb(0xf7768e),

                stars: Color::rgb(0xe0af68),
                forks: Color::rgb(0x7dcfff),
                issues: Color::rgb(0xf7768e),
                language: Color::rgb(0xbb9af7),
            },
        }
    }

    /// Get Monokai Pro theme
    pub fn monokai() -> Self {
        Self {
            name: "Monokai Pro".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x2d2a2e),
                foreground: Color::rgb(0xfcfcfa),
                border: Color::rgb(0x403e41),
                border_focused: Color::rgb(0xffd866),

                success: Color::rgb(0xa9dc76),
                warning: Color::rgb(0xffd866),
                error: Color::rgb(0xff6188),
                info: Color::rgb(0x78dce8),

                title: Color::rgb(0xab9df2),
                subtitle: Color::rgb(0x727072),
                selected: Color::rgb(0xffd866),
                selected_bg: Color::rgb(0x403e41),
                tab_active: Color::rgb(0xff6188),
                tab_inactive: Color::rgb(0x727072),

                primary: Color::rgb(0x78dce8),
                secondary: Color::rgb(0xab9df2),
                accent: Color::rgb(0xffd866),
                muted: Color::rgb(0x727072),

                health_healthy: Color::rgb(0xa9dc76),
                health_moderate: Color::rgb(0xffd866),
                health_warning: Color::rgb(0xfc9867),
                health_critical: Color::rgb(0xff6188),

                stars: Color::rgb(0xffd866),
                forks: Color::rgb(0x78dce8),
                issues: Color::rgb(0xff6188),
                language: Color::rgb(0xab9df2),
            },
        }
    }

    /// Get Catppuccin Macchiato theme
    pub fn catppuccin_macchiato() -> Self {
        Self {
            name: "Catppuccin Macchiato".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x24273a),
                foreground: Color::rgb(0xcad3f5),
                border: Color::rgb(0x494d64),
                border_focused: Color::rgb(0x8aadf4),

                success: Color::rgb(0xa6da95),
                warning: Color::rgb(0xeed49f),
                error: Color::rgb(0xed8796),
                info: Color::rgb(0x91d7e3),

                title: Color::rgb(0xc6a0f6),
                subtitle: Color::rgb(0xa5adcb),
                selected: Color::rgb(0x8aadf4),
                selected_bg: Color::rgb(0x363a4f),
                tab_active: Color::rgb(0xf5bde6),
                tab_inactive: Color::rgb(0x6e738d),

                primary: Color::rgb(0x8aadf4),
                secondary: Color::rgb(0xf5bde6),
                accent: Color::rgb(0xeed49f),
                muted: Color::rgb(0x6e738d),

                health_healthy: Color::rgb(0xa6da95),
                health_moderate: Color::rgb(0xeed49f),
                health_warning: Color::rgb(0xf5a97f),
                health_critical: Color::rgb(0xed8796),

                stars: Color::rgb(0xeed49f),
                forks: Color::rgb(0x8bd5ca),
                issues: Color::rgb(0xed8796),
                language: Color::rgb(0xc6a0f6),
            },
        }
    }

    /// Get Catppuccin Frappe theme
    pub fn catppuccin_frappe() -> Self {
        Self {
            name: "Catppuccin Frappe".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x303446),
                foreground: Color::rgb(0xc6d0f5),
                border: Color::rgb(0x51576d),
                border_focused: Color::rgb(0x8caaee),

                success: Color::rgb(0xa6d189),
                warning: Color::rgb(0xe5c890),
                error: Color::rgb(0xe78284),
                info: Color::rgb(0x99d1db),

                title: Color::rgb(0xca9ee6),
                subtitle: Color::rgb(0xa5adce),
                selected: Color::rgb(0x8caaee),
                selected_bg: Color::rgb(0x414559),
                tab_active: Color::rgb(0xf4b8e4),
                tab_inactive: Color::rgb(0x737994),

                primary: Color::rgb(0x8caaee),
                secondary: Color::rgb(0xf4b8e4),
                accent: Color::rgb(0xe5c890),
                muted: Color::rgb(0x737994),

                health_healthy: Color::rgb(0xa6d189),
                health_moderate: Color::rgb(0xe5c890),
                health_warning: Color::rgb(0xef9f76),
                health_critical: Color::rgb(0xe78284),

                stars: Color::rgb(0xe5c890),
                forks: Color::rgb(0x81c8be),
                issues: Color::rgb(0xe78284),
                language: Color::rgb(0xca9ee6),
            },
        }
    }

    /// Get Everforest Dark theme
    pub fn everforest() -> Self {
        Self {
            name: "Everforest".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x2d353b),
                foreground: Color::rgb(0xd3c6aa),
                border: Color::rgb(0x475258),
                border_focused: Color::rgb(0x83c092),

                success: Color::rgb(0xa7c080),
                warning: Color::rgb(0xdbbc7f),
                error: Color::rgb(0xe67e80),
                info: Color::rgb(0x7fbbb3),

                title: Color::rgb(0xd699b6),
                subtitle: Color::rgb(0x9da9a0),
                selected: Color::rgb(0x83c092),
                selected_bg: Color::rgb(0x3d484d),
                tab_active: Color::rgb(0xe69875),
                tab_inactive: Color::rgb(0x859289),

                primary: Color::rgb(0x83c092),
                secondary: Color::rgb(0xd699b6),
                accent: Color::rgb(0xdbbc7f),
                muted: Color::rgb(0x859289),

                health_healthy: Color::rgb(0xa7c080),
                health_moderate: Color::rgb(0xdbbc7f),
                health_warning: Color::rgb(0xe69875),
                health_critical: Color::rgb(0xe67e80),

                stars: Color::rgb(0xdbbc7f),
                forks: Color::rgb(0x7fbbb3),
                issues: Color::rgb(0xe67e80),
                language: Color::rgb(0xd699b6),
            },
        }
    }

    /// Get Rosé Pine theme
    pub fn rose_pine() -> Self {
        Self {
            name: "Rosé Pine".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x191724),
                foreground: Color::rgb(0xe0def4),
                border: Color::rgb(0x26233a),
                border_focused: Color::rgb(0x31748f),

                success: Color::rgb(0x9ccfd8),
                warning: Color::rgb(0xf6c177),
                error: Color::rgb(0xeb6f92),
                info: Color::rgb(0x31748f),

                title: Color::rgb(0xc4a7e7),
                subtitle: Color::rgb(0x908caa),
                selected: Color::rgb(0x31748f),
                selected_bg: Color::rgb(0x26233a),
                tab_active: Color::rgb(0xebbcba),
                tab_inactive: Color::rgb(0x6e6a86),

                primary: Color::rgb(0x31748f),
                secondary: Color::rgb(0xc4a7e7),
                accent: Color::rgb(0xf6c177),
                muted: Color::rgb(0x6e6a86),

                health_healthy: Color::rgb(0x9ccfd8),
                health_moderate: Color::rgb(0xf6c177),
                health_warning: Color::rgb(0xebbcba),
                health_critical: Color::rgb(0xeb6f92),

                stars: Color::rgb(0xf6c177),
                forks: Color::rgb(0x9ccfd8),
                issues: Color::rgb(0xeb6f92),
                language: Color::rgb(0xc4a7e7),
            },
        }
    }

    /// Get Kanagawa theme
    pub fn kanagawa() -> Self {
        Self {
            name: "Kanagawa".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0x1f1f28),
                foreground: Color::rgb(0xdcd7ba),
                border: Color::rgb(0x2a2a37),
                border_focused: Color::rgb(0x7e9cd8),

                success: Color::rgb(0x98bb6c),
                warning: Color::rgb(0xe6c384),
                error: Color::rgb(0xc34043),
                info: Color::rgb(0x7fb4ca),

                title: Color::rgb(0x957fb8),
                subtitle: Color::rgb(0x727169),
                selected: Color::rgb(0x7e9cd8),
                selected_bg: Color::rgb(0x2a2a37),
                tab_active: Color::rgb(0xd27e99),
                tab_inactive: Color::rgb(0x54546d),

                primary: Color::rgb(0x7e9cd8),
                secondary: Color::rgb(0x957fb8),
                accent: Color::rgb(0xe6c384),
                muted: Color::rgb(0x54546d),

                health_healthy: Color::rgb(0x98bb6c),
                health_moderate: Color::rgb(0xe6c384),
                health_warning: Color::rgb(0xffa066),
                health_critical: Color::rgb(0xc34043),

                stars: Color::rgb(0xe6c384),
                forks: Color::rgb(0x7fb4ca),
                issues: Color::rgb(0xc34043),
                language: Color::rgb(0x957fb8),
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
            Self::solarized_dark(),
            Self::solarized_light(),
            Self::one_dark(),
            Self::tokyo_night(),
            Self::monokai(),
            Self::catppuccin_macchiato(),
            Self::catppuccin_frappe(),
            Self::everforest(),
            Self::rose_pine(),
            Self::kanagawa(),
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
