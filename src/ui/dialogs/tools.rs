use super::*;

pub(super) fn about_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let (close_requested, _) = dialog_window(
        ctx,
        "about",
        tr(lang, "about_title"),
        Icon::Info,
        420.0,
        None,
        false,
        palette,
        |ui| {
            ui.vertical_centered(|ui| {
                crate::icons::app_logo(ui, 56.0);
                ui.add_space(7.0);
                ui.label(
                    RichText::new("RShell")
                        .size(23.0)
                        .strong()
                        .color(palette.text),
                );
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                );
                ui.add_space(7.0);
                ui.label(
                    RichText::new(tr(lang, "about_body"))
                        .size(13.0)
                        .color(palette.muted),
                );
                ui.add_space(5.0);
                ui.label(
                    RichText::new(tr(lang, "about_stack"))
                        .size(13.0)
                        .monospace()
                        .color(palette.text_soft),
                );
                ui.add_space(5.0);
                ui.label(
                    RichText::new(format!("{}: slobber · me@nandi.wang", tr(lang, "author")))
                        .size(13.0)
                        .color(palette.text_soft),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Tabler Icons · MIT License · Simple Icons · CC0")
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                );
                ui.label(
                    RichText::new(format!("SQLite: {}", app.config.database_path.display()))
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                );
            });
        },
    );
    app.about_open = !close_requested;
}

pub(super) fn keyring_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let mut import = false;
    let mut delete_id = None;
    let (close_requested, _) = dialog_window(
        ctx,
        "keyring",
        tr(lang, "keyring"),
        Icon::Key,
        560.0,
        None,
        true,
        palette,
        |ui| {
            ui.label(
                RichText::new(tr(lang, "managed_keys_title"))
                    .size(13.0)
                    .strong()
                    .color(palette.text),
            );
            ui.label(
                RichText::new(tr(lang, "managed_keys_desc"))
                    .size(13.0)
                    .color(palette.muted),
            );
            ui.add_space(5.0);
            if app.saved_keys.is_empty() {
                Frame::new()
                    .fill(palette.panel)
                    .stroke(Stroke::new(1.0, palette.border))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::same(9))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(tr(lang, "no_managed_keys_yet"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                    });
            } else {
                for key in &app.saved_keys {
                    Frame::new()
                        .fill(palette.panel)
                        .stroke(Stroke::new(1.0, palette.border))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(9, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                crate::icons::image(ui, Icon::Key, 18.0, palette.accent);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&key.name).size(13.0).color(palette.text),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {}",
                                            key.fingerprint,
                                            key.created_at.format("%Y-%m-%d")
                                        ))
                                        .size(13.0)
                                        .monospace()
                                        .color(palette.muted),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.small_button(tr(lang, "delete")).clicked() {
                                        delete_id = Some(key.id);
                                    }
                                });
                            });
                        });
                    ui.add_space(3.0);
                }
            }
            ui.add_space(8.0);
            Frame::new()
                .fill(palette.panel)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(Margin::same(9))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(tr(lang, "import_ssh_key"))
                            .size(13.0)
                            .color(palette.text),
                    );
                    ui.label(
                        RichText::new(tr(lang, "enter_private_key_path"))
                            .size(13.0)
                            .color(palette.muted),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(tr(lang, "key_name"))
                            .size(13.0)
                            .color(palette.muted),
                    );
                    singleline_input(
                        ui,
                        palette,
                        lang,
                        &mut app.key_name,
                        ui.available_width(),
                        false,
                        None,
                    );
                    ui.label(
                        RichText::new(tr(lang, "private_key_file"))
                            .size(13.0)
                            .color(palette.muted),
                    );
                    path_input_with_browse(
                        ui,
                        palette,
                        lang,
                        &mut app.key_file_path,
                        "Select SSH private key",
                        "~/.ssh/id_ed25519",
                    );
                    ui.add_space(6.0);
                    if secondary_button(ui, palette, tr(lang, "import_managed_key")).clicked() {
                        import = true;
                    }
                });
            if let Some(error) = &app.keyring_error {
                ui.add_space(4.0);
                ui.colored_label(palette.red, RichText::new(error).size(13.0));
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new(tr(lang, "filesystem_key_warning"))
                    .size(13.0)
                    .color(palette.amber),
            );
            ui.label(
                RichText::new(format!(
                    "{}: {} · {} {}",
                    tr(lang, "software_password"),
                    if app.security_record.has_password() {
                        tr(lang, "enabled")
                    } else {
                        tr(lang, "not_set")
                    },
                    tr(lang, "remember"),
                    remember_expiry_label(app.security_record.remember_until)
                ))
                .size(13.0)
                .monospace()
                .color(palette.muted),
            );
        },
    );
    if let Some(id) = delete_id {
        app.delete_saved_key(id);
    }
    if import {
        if let Err(error) = app.import_saved_key_from_path() {
            app.keyring_error = Some(error);
        }
    }
    app.keyring_open = !close_requested;
}

pub(super) fn snippets_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let mut run = None;
    let tab_id = app.focused_tab().map(|tab| tab.id);
    let (close_requested, _) = dialog_window(
        ctx,
        "snippets",
        tr(lang, "snippets"),
        Icon::Code,
        480.0,
        None,
        false,
        palette,
        |ui| {
            for (title, command) in [
                ("Disk usage", "df -h"),
                ("Memory usage", "free -h"),
                ("Kubernetes pods", "kubectl get pods"),
                ("Process snapshot", "ps aux --sort=-%mem | head -n 6"),
                ("Nginx status", "systemctl status nginx"),
                ("Tail nginx errors", "tail -f /var/log/nginx/error.log"),
            ] {
                Frame::new()
                    .fill(palette.panel)
                    .stroke(Stroke::new(1.0, palette.border))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(8, 5))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            crate::icons::image(ui, Icon::Terminal, 14.0, palette.accent);
                            ui.label(RichText::new(title).size(13.0).color(palette.text));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(command)
                                        .size(13.0)
                                        .monospace()
                                        .color(palette.muted),
                                );
                                if ui.small_button(tr(lang, "run")).clicked() {
                                    run = Some(command.to_owned());
                                }
                            });
                        });
                    });
                ui.add_space(3.0);
            }
        },
    );
    app.snippets_open = !close_requested;
    if let (Some(tab_id), Some(command)) = (tab_id, run) {
        app.execute_terminal_command(tab_id, command);
        app.snippets_open = false;
    }
}
