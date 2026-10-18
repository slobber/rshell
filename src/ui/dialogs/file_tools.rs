use super::*;
use crate::ui::text_input::TextInput;

pub(super) fn editor_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let Some(mut dialog) = app.editor_dialog.take() else {
        return;
    };
    let mut save = false;
    let mut close_requested = false;
    let title = format!("{} · {}", tr(lang, "edit_remote"), dialog.path);
    let (header_close, _) = dialog_window(
        ctx,
        "editor_dialog",
        &title,
        Icon::FileCode,
        760.0,
        Some(520.0),
        true,
        palette,
        |ui| {
            ui.horizontal(|ui| {
                crate::icons::image(ui, Icon::FileCode, 15.0, palette.accent);
                ui.label(
                    RichText::new(&dialog.path)
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                );
            });
            ui.add_space(5.0);
            if dialog.truncated {
                ui.colored_label(
                    palette.amber,
                    RichText::new(tr(lang, "preview_truncated")).size(13.0),
                );
            }
            if dialog.remote && dialog.content == "Loading remote file…" {
                ui.colored_label(palette.muted, RichText::new(tr(lang, "loading")).size(13.0));
            }
            ScrollArea::both().show(ui, |ui| {
                TextInput::multiline(&mut dialog.content)
                    .desired_size([ui.available_width(), 400.0])
                    .font(FontId::monospace(13.0))
                    .code_editor()
                    .show(ui, palette, lang);
            });
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                if primary_button(ui, palette, tr(lang, "save")).clicked() {
                    save = true;
                }
                if secondary_button(ui, palette, tr(lang, "cancel")).clicked() {
                    close_requested = true;
                }
            });
        },
    );
    if close_requested || header_close {
        return;
    }
    if save {
        match app.save_editor(&dialog) {
            Ok(()) if !dialog.remote => {
                app.log(
                    crate::domain::LogLevel::Success,
                    format!("Saved {}", dialog.path),
                );
                app.toast(tr(lang, "saved"));
            }
            Ok(()) => {}
            Err(error) => app.log(crate::domain::LogLevel::Error, error),
        }
    } else {
        app.editor_dialog = Some(dialog);
    }
}

pub(super) fn chmod_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let Some(mut dialog) = app.chmod_dialog.take() else {
        return;
    };
    let mut save = false;
    let mut close_requested = false;
    let (header_close, _) = dialog_window(
        ctx,
        "chmod_dialog",
        tr(lang, "chmod"),
        Icon::ShieldLock,
        380.0,
        None,
        false,
        palette,
        |ui| {
            ui.label(
                RichText::new(&dialog.path)
                    .size(13.0)
                    .monospace()
                    .color(palette.muted),
            );
            ui.add_space(5.0);
            ui.label(
                RichText::new(tr(lang, "mode"))
                    .size(13.0)
                    .color(palette.muted),
            );
            TextInput::singleline(&mut dialog.mode)
                .desired_width(120.0)
                .show(ui, palette, lang);
            ui.add_space(8.0);
            if primary_button(ui, palette, tr(lang, "apply")).clicked() {
                save = true;
            }
            if secondary_button(ui, palette, tr(lang, "cancel")).clicked() {
                close_requested = true;
            }
        },
    );
    if close_requested || header_close {
        return;
    }
    if save {
        let Some(tab_id) = dialog.tab_id else {
            app.log(
                crate::domain::LogLevel::Error,
                "chmod is missing its SFTP tab",
            );
            return;
        };
        if !app
            .ssh
            .set_remote_permissions(tab_id, dialog.path.clone(), dialog.mode.clone())
        {
            app.log(
                crate::domain::LogLevel::Error,
                "SFTP channel is not available",
            );
        }
    } else {
        app.chmod_dialog = Some(dialog);
    }
}

pub(super) fn md5_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let Some(dialog) = app.md5_dialog.take() else {
        return;
    };
    let mut copy = false;
    let (header_close, _) = dialog_window(
        ctx,
        "md5_dialog",
        tr(lang, "md5"),
        Icon::ShieldLock,
        450.0,
        None,
        false,
        palette,
        |ui| {
            ui.label(RichText::new(&dialog.path).size(13.0).color(palette.muted));
            ui.add_space(7.0);
            Frame::new()
                .fill(palette.canvas)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(Margin::same(9))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(&dialog.hash)
                            .size(13.0)
                            .monospace()
                            .color(palette.green),
                    );
                });
            ui.add_space(7.0);
            if secondary_button(ui, palette, tr(lang, "copy")).clicked() {
                copy = true;
            }
        },
    );
    if !header_close {
        app.md5_dialog = Some(dialog.clone());
    }
    if copy {
        ctx.copy_text(dialog.hash);
    }
}
