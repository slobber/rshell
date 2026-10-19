mod common;
mod feedback;
mod file_tools;
mod history;
mod host;
mod palette;
mod security;
mod settings;
mod tools;

use eframe::egui::{
    self, Align, Align2, Color32, ComboBox, CornerRadius, FontFamily, FontId, Frame, Layout,
    Margin, RichText, ScrollArea, Stroke, Ui, Vec2,
};

use crate::app::{AppLock, ConsoleTab, SecurityAction, ShellCoreApp};
use crate::domain::{flatten_folders, Axis, ConnectionBackend, RememberDuration, ThemeMode};
use crate::i18n::{tr, Language};
use crate::icons::Icon;
use crate::security::remember_expiry_label;
use crate::theme::Palette;
use crate::ui::components::{
    combo_icon, modal_frame, primary_button, secondary_button, section_label,
};

use crate::ui::text_input::singleline_input;
use common::{dialog_window, field, path_input_with_browse, remember_combo};

pub use feedback::{drag_ghost, toast};
pub use security::lock_screen;

pub fn draw_modals(app: &mut ShellCoreApp, ctx: &egui::Context) {
    let palette = Palette::for_mode(app.settings.theme);
    let lang = app.settings.language;

    if app.settings_open {
        settings::settings_window(app, ctx, palette, lang);
    }
    if app.about_open {
        tools::about_window(app, ctx, palette, lang);
    }
    if app.keyring_open {
        tools::keyring_window(app, ctx, palette, lang);
    }
    if app.snippets_open {
        tools::snippets_window(app, ctx, palette, lang);
    }
    if app.host_dialog.is_some() {
        host::host_window(app, ctx, palette, lang);
    }
    if app.folder_dialog.is_some() {
        host::folder_window(app, ctx, palette, lang);
    }
    if app.folder_delete.is_some() {
        host::delete_folder_window(app, ctx, palette, lang);
    }
    if app.host_delete.is_some() {
        host::delete_host_window(app, ctx, palette, lang);
    }
    if app.command_history.is_some() {
        history::command_history_window(app, ctx, palette, lang);
    }
    if app.command_palette_open {
        palette::command_palette(app, ctx, palette, lang);
    }
    if app.editor_dialog.is_some() {
        file_tools::editor_window(app, ctx, palette, lang);
    }
    if app.chmod_dialog.is_some() {
        file_tools::chmod_window(app, ctx, palette, lang);
    }
    if app.md5_dialog.is_some() {
        file_tools::md5_window(app, ctx, palette, lang);
    }
}
