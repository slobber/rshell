use super::*;
use crate::app::{HashAlgorithm, SftpFileOperation};
use crate::ui::text_input::TextInput;
use std::time::{Duration, Instant};

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
    let viewport = ctx.content_rect().size();
    let dialog_width = (viewport.x - 48.0).clamp(480.0, 760.0);
    let dialog_height = (viewport.y - 48.0).clamp(320.0, 560.0);
    let editor_height = (dialog_height - 160.0).clamp(150.0, 400.0);
    let (header_close, _) = dialog_window(
        ctx,
        "editor_dialog",
        &title,
        Icon::FileCode,
        dialog_width,
        Some(dialog_height),
        false,
        palette,
        |ui| {
            ui.horizontal(|ui| {
                crate::icons::image(ui, Icon::FileCode, 15.0, palette.accent);
                let path_width = (ui.available_width() - 4.0).max(80.0);
                ui.add_sized(
                    [path_width, 20.0],
                    egui::Label::new(
                        RichText::new(&dialog.path)
                            .size(13.0)
                            .monospace()
                            .color(palette.muted),
                    )
                    .truncate(),
                )
                .on_hover_text(&dialog.path);
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
            ScrollArea::both()
                .auto_shrink([false, false])
                .max_height(editor_height)
                .show(ui, |ui| {
                    TextInput::multiline(&mut dialog.content)
                        .desired_size([ui.available_width(), editor_height])
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
            ui.horizontal(|ui| {
                if primary_button(ui, palette, tr(lang, "apply")).clicked() {
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

pub(super) fn hash_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let Some(mut dialog) = app.hash_dialog.take() else {
        return;
    };
    if dialog
        .copied
        .is_some_and(|(_, copied_at)| copied_at.elapsed() >= Duration::from_secs(1))
    {
        dialog.copied = None;
    }
    let (header_close, _) = dialog_window(
        ctx,
        "hash_dialog",
        tr(lang, "hash"),
        Icon::ShieldLock,
        680.0,
        None,
        false,
        palette,
        |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(&dialog.path)
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                )
                .truncate(),
            )
            .on_hover_text(&dialog.path);
            ui.add_space(7.0);
            let Some(hashes) = &dialog.hashes else {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new(tr(lang, "calculating_hashes"))
                            .size(13.0)
                            .color(palette.muted),
                    );
                });
                return;
            };
            let rows = [
                (HashAlgorithm::Md5, "MD5", &hashes.md5),
                (HashAlgorithm::Sha1, "SHA-1", &hashes.sha1),
                (HashAlgorithm::Sha256, "SHA-256", &hashes.sha256),
                (HashAlgorithm::Sha512, "SHA-512", &hashes.sha512),
            ];
            for (algorithm, label, value) in rows {
                let copied = dialog.copied.is_some_and(|(copied_algorithm, copied_at)| {
                    copied_algorithm == algorithm && copied_at.elapsed() < Duration::from_secs(1)
                });
                Frame::new()
                    .fill(palette.canvas)
                    .stroke(Stroke::new(1.0, palette.border))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(9, 7))
                    .show(ui, |ui| {
                        let mut copy_clicked = false;
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(label)
                                    .size(13.0)
                                    .strong()
                                    .monospace()
                                    .color(palette.text_soft),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    copy_clicked = crate::ui::components::icon_button(
                                        ui,
                                        palette,
                                        if copied {
                                            Icon::CircleCheck
                                        } else {
                                            Icon::Copy
                                        },
                                        13.0,
                                        if copied {
                                            tr(lang, "copied")
                                        } else {
                                            tr(lang, "copy")
                                        },
                                        copied,
                                    )
                                    .clicked();
                                },
                            );
                        });
                        ui.add(
                            egui::Label::new(
                                RichText::new(value)
                                    .size(13.0)
                                    .monospace()
                                    .color(palette.green),
                            )
                            .wrap(),
                        );
                        if copy_clicked {
                            ctx.copy_text(value.clone());
                            dialog.copied = Some((algorithm, Instant::now()));
                            ctx.request_repaint();
                            ctx.request_repaint_after(Duration::from_secs(1));
                        }
                    });
                ui.add_space(5.0);
            }
        },
    );
    if !header_close {
        app.hash_dialog = Some(dialog);
    }
}

pub(super) fn sftp_file_operation_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let Some(mut dialog) = app.sftp_file_dialog.take() else {
        return;
    };
    let mut confirm = false;
    let mut close_requested = false;
    let deleting = dialog.operation == SftpFileOperation::Delete;
    let title = match dialog.operation {
        SftpFileOperation::NewFolder => tr(lang, "new_folder"),
        SftpFileOperation::Rename => tr(lang, "rename"),
        SftpFileOperation::Delete => tr(lang, "delete"),
    };
    let icon = match dialog.operation {
        SftpFileOperation::NewFolder => Icon::FolderPlus,
        SftpFileOperation::Rename => Icon::Pencil,
        SftpFileOperation::Delete => Icon::Trash,
    };

    let (header_close, _) = dialog_window(
        ctx,
        "sftp_file_operation_dialog",
        title,
        icon,
        460.0,
        None,
        false,
        palette,
        |ui| {
            ui.label(
                RichText::new(&dialog.directory)
                    .size(13.0)
                    .monospace()
                    .color(palette.muted),
            );
            ui.add_space(7.0);
            if deleting {
                let warning = tr(lang, "delete_item_warning").replace("{name}", &dialog.item_name);
                ui.label(RichText::new(warning).size(13.0).color(palette.text_soft));
            } else {
                if dialog.operation == SftpFileOperation::Rename {
                    ui.label(
                        RichText::new(&dialog.item_name)
                            .size(13.0)
                            .monospace()
                            .color(palette.text_soft),
                    );
                    ui.add_space(5.0);
                }
                let label = if dialog.operation == SftpFileOperation::NewFolder {
                    tr(lang, "folder_name")
                } else {
                    tr(lang, "new_name")
                };
                ui.label(RichText::new(label).size(13.0).color(palette.muted));
                let response = TextInput::singleline(&mut dialog.name_input)
                    .desired_width(ui.available_width())
                    .vertical_align(egui::Align::Center)
                    .show(ui, palette, lang);
                if response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    confirm = true;
                }
            }
            if let Some(error) = &dialog.error {
                ui.add_space(6.0);
                ui.label(RichText::new(error).size(13.0).color(palette.red));
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let action_label = if deleting {
                    tr(lang, "delete")
                } else {
                    tr(lang, "apply")
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(action_label)
                                .size(13.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(if deleting {
                            palette.red
                        } else {
                            palette.accent
                        })
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(100.0, 30.0)),
                    )
                    .clicked()
                {
                    confirm = true;
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
    if confirm {
        match app.submit_sftp_file_operation(&dialog) {
            Ok(()) => {}
            Err(error) => {
                dialog.error = Some(error);
                app.sftp_file_dialog = Some(dialog);
            }
        }
    } else {
        app.sftp_file_dialog = Some(dialog);
    }
}
