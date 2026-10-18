use std::collections::HashMap;

use crate::domain::{AppSettings, LogLevel, TerminalColorTheme, ThemeMode};
use crate::i18n::Language;

use super::ShellCoreApp;

impl ShellCoreApp {
    pub fn persist_hosts(&mut self) {
        let (Some(storage), Some(cipher)) = (&mut self.storage, &self.cipher) else {
            return;
        };
        if let Err(error) = storage.save_host_tree(&self.hosts, cipher) {
            self.storage_error = Some(error.clone());
            self.log(LogLevel::Error, error);
        }
    }

    pub fn persist_settings(&mut self) {
        let Some(storage) = &mut self.storage else {
            return;
        };
        let values = settings_map(&self.settings);
        if let Err(error) = storage.save_settings(&values) {
            self.storage_error = Some(error);
        }
    }
}

pub(super) fn apply_settings_map(settings: &mut AppSettings, values: &HashMap<String, String>) {
    if let Some(value) = values.get("theme") {
        settings.theme = ThemeMode::parse(value);
    }
    if let Some(value) = values.get("terminal_theme") {
        settings.terminal_theme = TerminalColorTheme::parse(value);
    }
    if let Some(value) = values.get("language") {
        if let Some(language) = Language::parse(value) {
            settings.language = language;
        }
    }
    if let Some(value) = values.get("ui_font") {
        settings.ui_font = value.clone();
    }
    if let Some(value) = values.get("ui_font_size") {
        settings.ui_font_size = value.parse::<f32>().unwrap_or(14.0).clamp(
            crate::theme::UI_FONT_SIZE_MIN,
            crate::theme::UI_FONT_SIZE_MAX,
        );
    }
    if let Some(value) = values.get("terminal_font") {
        settings.terminal_font = value.clone();
    }
    if let Some(value) = values.get("terminal_zoom") {
        settings.terminal_zoom = value.parse::<f32>().unwrap_or(1.0).clamp(0.75, 1.6);
    }
    if let Some(value) = values.get("sidebar_collapsed") {
        settings.sidebar_collapsed = value == "true";
    }
    if let Some(value) = values.get("console_collapsed") {
        settings.console_collapsed = value == "true";
    }
    if let Some(value) = values.get("auto_scroll") {
        settings.auto_scroll = value != "false";
    }
    if let Some(value) = values.get("split_ratio") {
        settings.split_ratio = value.parse::<f32>().unwrap_or(0.5).clamp(0.2, 0.8);
    }
    if let Some(value) = values.get("taskbar_progress") {
        settings.taskbar_progress = value != "false";
    }
    if let Some(value) = values.get("taskbar_attention") {
        settings.taskbar_attention = value != "false";
    }
}

fn settings_map(settings: &AppSettings) -> HashMap<String, String> {
    HashMap::from([
        ("theme".to_owned(), settings.theme.as_str().to_owned()),
        (
            "terminal_theme".to_owned(),
            settings.terminal_theme.as_str().to_owned(),
        ),
        ("language".to_owned(), settings.language.as_str().to_owned()),
        ("ui_font".to_owned(), settings.ui_font.clone()),
        (
            "ui_font_size".to_owned(),
            format!("{:.1}", settings.ui_font_size),
        ),
        ("terminal_font".to_owned(), settings.terminal_font.clone()),
        (
            "terminal_zoom".to_owned(),
            format!("{:.2}", settings.terminal_zoom),
        ),
        (
            "sidebar_collapsed".to_owned(),
            settings.sidebar_collapsed.to_string(),
        ),
        (
            "console_collapsed".to_owned(),
            settings.console_collapsed.to_string(),
        ),
        ("auto_scroll".to_owned(), settings.auto_scroll.to_string()),
        (
            "split_ratio".to_owned(),
            format!("{:.3}", settings.split_ratio),
        ),
        (
            "taskbar_progress".to_owned(),
            settings.taskbar_progress.to_string(),
        ),
        (
            "taskbar_attention".to_owned(),
            settings.taskbar_attention.to_string(),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::{apply_settings_map, settings_map};
    use crate::domain::AppSettings;
    use std::collections::HashMap;

    #[test]
    fn ui_font_size_round_trips_and_clamps() {
        let mut settings = AppSettings::default();
        apply_settings_map(
            &mut settings,
            &HashMap::from([("ui_font_size".to_owned(), "16.5".to_owned())]),
        );
        assert_eq!(settings.ui_font_size, 16.5);
        assert_eq!(
            settings_map(&settings).get("ui_font_size"),
            Some(&"16.5".to_owned())
        );

        apply_settings_map(
            &mut settings,
            &HashMap::from([("ui_font_size".to_owned(), "99".to_owned())]),
        );
        assert_eq!(settings.ui_font_size, crate::theme::UI_FONT_SIZE_MAX);

        apply_settings_map(
            &mut settings,
            &HashMap::from([("ui_font_size".to_owned(), "1".to_owned())]),
        );
        assert_eq!(settings.ui_font_size, crate::theme::UI_FONT_SIZE_MIN);
    }
}
