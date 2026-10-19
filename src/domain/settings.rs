use chrono::{DateTime, Utc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn toggled(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("light") {
            Self::Light
        } else {
            Self::Dark
        }
    }

    pub fn is_dark(self) -> bool {
        self == Self::Dark
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalColorTheme {
    RShellDark,
    RShellLight,
    SolarizedDark,
    Dracula,
    Paper,
}

impl TerminalColorTheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RShellDark => "rshell-dark",
            Self::RShellLight => "rshell-light",
            Self::SolarizedDark => "solarized-dark",
            Self::Dracula => "dracula",
            Self::Paper => "paper",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "rshell-light" => Self::RShellLight,
            "solarized-dark" => Self::SolarizedDark,
            "dracula" => Self::Dracula,
            "paper" => Self::Paper,
            _ => Self::RShellDark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RShellDark => "RShell Dark",
            Self::RShellLight => "RShell Light",
            Self::SolarizedDark => "Solarized Dark",
            Self::Dracula => "Dracula",
            Self::Paper => "Paper",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppSettings {
    pub theme: ThemeMode,
    pub terminal_theme: TerminalColorTheme,
    pub language: crate::i18n::Language,
    pub ui_font: String,
    pub ui_font_size: f32,
    pub terminal_font: String,
    pub terminal_zoom: f32,
    pub command_history_limit: usize,
    pub sidebar_collapsed: bool,
    pub console_collapsed: bool,
    pub auto_scroll: bool,
    pub split_ratio: f32,
    pub taskbar_progress: bool,
    pub taskbar_attention: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            terminal_theme: TerminalColorTheme::RShellDark,
            language: crate::i18n::Language::detect(),
            ui_font: String::new(),
            ui_font_size: 14.0,
            terminal_font: String::new(),
            terminal_zoom: 1.0,
            command_history_limit: 500,
            sidebar_collapsed: false,
            console_collapsed: false,
            auto_scroll: true,
            split_ratio: 0.5,
            taskbar_progress: true,
            taskbar_attention: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RememberDuration {
    None,
    Week,
    Month,
    Forever,
}

impl RememberDuration {
    pub fn expiry(self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::None => None,
            Self::Week => Some(now + chrono::Duration::days(7)),
            Self::Month => Some(now + chrono::Duration::days(30)),
            Self::Forever => Some(now + chrono::Duration::days(365 * 100)),
        }
    }
}
