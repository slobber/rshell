use super::*;

pub(super) fn host_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let Some(mut draft) = app.host_dialog.take() else {
        return;
    };
    let editing = draft.editing_id.is_some();
    let mut save = false;
    let mut close_requested = false;
    let mut folders = Vec::new();
    flatten_folders(&app.hosts, "", &mut folders);
    if draft.auth == crate::domain::AuthKind::Key
        && draft.key_source == crate::domain::KeySource::Managed
        && !app
            .saved_keys
            .iter()
            .any(|key| Some(key.id) == draft.key_id)
    {
        draft.key_id = app.saved_keys.first().map(|key| key.id);
    }
    let window_title = if draft.title.is_empty() {
        tr(lang, "new_session_title").to_owned()
    } else {
        draft.title.clone()
    };

    let (header_close, _) = dialog_window(
        ctx,
        "host_dialog",
        &window_title,
        if editing { Icon::Pencil } else { Icon::Server2 },
        460.0,
        None,
        false,
        palette,
        |ui| {
            field(ui, palette, lang, tr(lang, "name_label"), &mut draft.name);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let previous_backend = draft.backend;
                    ui.label(
                        RichText::new(tr(lang, "connection_type"))
                            .size(13.0)
                            .color(palette.muted),
                    );
                    ComboBox::from_id_salt("host_backend")
                        .icon(combo_icon)
                        .selected_text(match draft.backend {
                            ConnectionBackend::Ssh => tr(lang, "backend_ssh"),
                            ConnectionBackend::LocalWsl => tr(lang, "backend_wsl"),
                            ConnectionBackend::Rdp => tr(lang, "backend_rdp"),
                        })
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut draft.backend,
                                ConnectionBackend::Ssh,
                                tr(lang, "backend_ssh"),
                            );
                            if cfg!(windows) {
                                ui.selectable_value(
                                    &mut draft.backend,
                                    ConnectionBackend::LocalWsl,
                                    tr(lang, "backend_wsl"),
                                );
                            }
                            ui.selectable_value(
                                &mut draft.backend,
                                ConnectionBackend::Rdp,
                                tr(lang, "backend_rdp"),
                            );
                        });
                    if draft.backend != previous_backend {
                        match draft.backend {
                            ConnectionBackend::Rdp if draft.port == "22" => {
                                draft.port = "3389".to_owned();
                            }
                            ConnectionBackend::Ssh
                                if draft.port == "3389" || draft.port == "5986" =>
                            {
                                draft.port = "22".to_owned();
                            }
                            _ => {}
                        }
                        if draft.backend == ConnectionBackend::Rdp && draft.gateway == "direct" {
                            draft.gateway.clear();
                        } else if draft.backend != ConnectionBackend::Rdp
                            && draft.gateway.is_empty()
                        {
                            draft.gateway = "direct".to_owned();
                        }
                        if draft.backend == ConnectionBackend::Rdp {
                            draft.auth = crate::domain::AuthKind::Password;
                            if draft.secret == "~/.ssh/id_ed25519" {
                                draft.secret.clear();
                            }
                        }
                    }
                });
            });
            if draft.backend == ConnectionBackend::Ssh {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "ip_label"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(ui, palette, lang, &mut draft.ip, 300.0, false, None);
                    });
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "port_label"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(ui, palette, lang, &mut draft.port, 70.0, false, None);
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "user_label"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(ui, palette, lang, &mut draft.user, 210.0, false, None);
                    });
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "auth_label"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        let previous_auth = draft.auth;
                        ComboBox::from_id_salt("host_auth")
                            .icon(combo_icon)
                            .selected_text(draft.auth.label())
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut draft.auth,
                                    crate::domain::AuthKind::Key,
                                    tr(lang, "public_key"),
                                );
                                ui.selectable_value(
                                    &mut draft.auth,
                                    crate::domain::AuthKind::Password,
                                    tr(lang, "password"),
                                );
                                ui.selectable_value(
                                    &mut draft.auth,
                                    crate::domain::AuthKind::Agent,
                                    tr(lang, "ssh_agent"),
                                );
                            });
                        if draft.auth != previous_auth {
                            draft.secret.clear();
                            if draft.auth == crate::domain::AuthKind::Key
                                && draft.key_source == crate::domain::KeySource::FileSystem
                            {
                                draft.secret = "~/.ssh/id_ed25519".to_owned();
                            }
                        }
                    });
                });
                ui.add_space(4.0);
                match draft.auth {
                    crate::domain::AuthKind::Key => {
                        ui.label(
                            RichText::new(tr(lang, "key_source"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        ui.horizontal(|ui| {
                            let previous_source = draft.key_source;
                            ui.selectable_value(
                                &mut draft.key_source,
                                crate::domain::KeySource::Managed,
                                tr(lang, "managed_key"),
                            );
                            ui.selectable_value(
                                &mut draft.key_source,
                                crate::domain::KeySource::FileSystem,
                                tr(lang, "filesystem_key"),
                            );
                            if draft.key_source != previous_source {
                                draft.secret.clear();
                                if draft.key_source == crate::domain::KeySource::FileSystem {
                                    draft.secret = "~/.ssh/id_ed25519".to_owned();
                                }
                            }
                        });
                        match draft.key_source {
                            crate::domain::KeySource::Managed => {
                                if app.saved_keys.is_empty() {
                                    ui.colored_label(
                                        palette.amber,
                                        RichText::new(tr(lang, "no_managed_keys")).size(13.0),
                                    );
                                } else {
                                    ComboBox::from_id_salt("host_managed_key")
                                        .icon(combo_icon)
                                        .selected_text(
                                            draft
                                                .key_id
                                                .and_then(|id| {
                                                    app.saved_keys.iter().find(|key| key.id == id)
                                                })
                                                .map(|key| key.name.as_str())
                                                .unwrap_or(tr(lang, "select_key")),
                                        )
                                        .width(260.0)
                                        .show_ui(ui, |ui| {
                                            for key in &app.saved_keys {
                                                ui.selectable_value(
                                                    &mut draft.key_id,
                                                    Some(key.id),
                                                    format!("{} · {}", key.name, key.fingerprint),
                                                );
                                            }
                                        });
                                }
                                if ui.small_button(tr(lang, "manage_keys")).clicked() {
                                    app.keyring_open = true;
                                }
                            }
                            crate::domain::KeySource::FileSystem => {
                                ui.label(
                                    RichText::new(tr(lang, "private_key_path"))
                                        .size(13.0)
                                        .color(palette.muted),
                                );
                                path_input_with_browse(
                                    ui,
                                    palette,
                                    lang,
                                    &mut draft.secret,
                                    "Select SSH private key",
                                    "~/.ssh/id_ed25519",
                                );
                                ui.colored_label(
                                    palette.amber,
                                    RichText::new(tr(lang, "external_key_warning")).size(13.0),
                                );
                            }
                        }
                    }
                    crate::domain::AuthKind::Password => {
                        ui.label(
                            RichText::new(tr(lang, "secret_label"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(
                            ui,
                            palette,
                            lang,
                            &mut draft.secret,
                            ui.available_width(),
                            true,
                            None,
                        );
                    }
                    crate::domain::AuthKind::Agent => {
                        ui.label(
                            RichText::new(tr(lang, "ssh_agent_hint"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                    }
                }
            } else if draft.backend == ConnectionBackend::LocalWsl {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(tr(lang, "wsl_distro"))
                        .size(13.0)
                        .color(palette.muted),
                );
                singleline_input(
                    ui,
                    palette,
                    lang,
                    &mut draft.wsl_distro,
                    ui.available_width(),
                    false,
                    None,
                );
                ui.label(
                    RichText::new(tr(lang, "wsl_distro_hint"))
                        .size(13.0)
                        .color(palette.muted),
                );
            } else {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "ip_label"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(ui, palette, lang, &mut draft.ip, 300.0, false, None);
                    });
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "port_label"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(ui, palette, lang, &mut draft.port, 70.0, false, None);
                    });
                });
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "user_label"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(ui, palette, lang, &mut draft.user, 210.0, false, None);
                    });
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "rdp_password"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(ui, palette, lang, &mut draft.secret, 210.0, true, None);
                    });
                });
                ui.label(
                    RichText::new(tr(lang, "rdp_domain"))
                        .size(13.0)
                        .color(palette.muted),
                );
                singleline_input(
                    ui,
                    palette,
                    lang,
                    &mut draft.gateway,
                    ui.available_width(),
                    false,
                    None,
                );
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(tr(lang, "folder_label"))
                            .size(13.0)
                            .color(palette.muted),
                    );
                    ComboBox::from_id_salt("host_folder")
                        .icon(combo_icon)
                        .selected_text(
                            draft
                                .folder_id
                                .and_then(|id| {
                                    folders.iter().find(|(folder_id, _)| *folder_id == id)
                                })
                                .map(|(_, label)| label.as_str())
                                .unwrap_or(tr(lang, "sessions_auto")),
                        )
                        .width(260.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut draft.folder_id,
                                None,
                                tr(lang, "sessions_auto"),
                            );
                            for (id, label) in &folders {
                                ui.selectable_value(&mut draft.folder_id, Some(*id), label);
                            }
                        });
                });
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(tr(lang, "environment"))
                            .size(13.0)
                            .color(palette.muted),
                    );
                    singleline_input(
                        ui,
                        palette,
                        lang,
                        &mut draft.environment,
                        130.0,
                        false,
                        None,
                    );
                });
            });
            ui.add_space(4.0);
            if draft.backend == ConnectionBackend::Ssh {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "gateway"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(ui, palette, lang, &mut draft.gateway, 300.0, false, None);
                    });
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "keep_alive"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        singleline_input(
                            ui,
                            palette,
                            lang,
                            &mut draft.keep_alive,
                            70.0,
                            false,
                            None,
                        );
                    });
                });
            }
            ui.add_space(4.0);
            if !editing {
                ui.checkbox(
                    &mut draft.connect_after_save,
                    RichText::new(tr(lang, "connect_after_save")).size(13.0),
                );
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let key_ready = match draft.auth {
                    crate::domain::AuthKind::Key => match draft.key_source {
                        crate::domain::KeySource::Managed => draft.key_id.is_some(),
                        crate::domain::KeySource::FileSystem => !draft.secret.trim().is_empty(),
                    },
                    crate::domain::AuthKind::Password => !draft.secret.is_empty(),
                    crate::domain::AuthKind::Agent => true,
                };
                let valid = !draft.name.trim().is_empty()
                    && match draft.backend {
                        ConnectionBackend::Ssh => !draft.ip.trim().is_empty() && key_ready,
                        ConnectionBackend::LocalWsl => true,
                        ConnectionBackend::Rdp => {
                            !draft.ip.trim().is_empty() && !draft.user.trim().is_empty()
                        }
                    };
                if ui
                    .add_enabled(
                        valid,
                        egui::Button::new(
                            RichText::new(tr(lang, "save"))
                                .size(13.0)
                                .color(palette.on_accent),
                        )
                        .fill(palette.accent)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(100.0, 30.0)),
                    )
                    .clicked()
                {
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
        app.save_host(draft);
    } else {
        app.host_dialog = Some(draft);
    }
}

pub(super) fn folder_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let Some(mut draft) = app.folder_dialog.take() else {
        return;
    };
    let editing = draft.editing_id.is_some();
    let mut save = false;
    let mut close_requested = false;
    let mut folders = Vec::new();
    flatten_folders(&app.hosts, "", &mut folders);
    let (header_close, _) = dialog_window(
        ctx,
        "folder_dialog",
        if editing {
            tr(lang, "rename_folder")
        } else {
            tr(lang, "add_folder_title")
        },
        if editing {
            Icon::Pencil
        } else {
            Icon::FolderPlus
        },
        420.0,
        None,
        false,
        palette,
        |ui| {
            field(ui, palette, lang, tr(lang, "folder_name"), &mut draft.name);
            field(ui, palette, lang, tr(lang, "subtitle"), &mut draft.subtitle);
            if !editing {
                ui.label(
                    RichText::new(tr(lang, "parent_folder"))
                        .size(13.0)
                        .color(palette.muted),
                );
                ComboBox::from_id_salt("folder_parent")
                    .icon(combo_icon)
                    .selected_text(
                        draft
                            .parent_id
                            .and_then(|id| folders.iter().find(|(folder_id, _)| *folder_id == id))
                            .map(|(_, label)| label.as_str())
                            .unwrap_or(tr(lang, "top_level")),
                    )
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.parent_id, None, tr(lang, "top_level"));
                        for (id, label) in &folders {
                            ui.selectable_value(&mut draft.parent_id, Some(*id), label);
                        }
                    });
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !draft.name.trim().is_empty(),
                        egui::Button::new(
                            RichText::new(if editing {
                                tr(lang, "save")
                            } else {
                                tr(lang, "add")
                            })
                            .size(13.0)
                            .color(palette.on_accent),
                        )
                        .fill(palette.accent)
                        .stroke(Stroke::NONE),
                    )
                    .clicked()
                {
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
        if editing {
            app.rename_folder(draft);
        } else {
            app.add_folder(draft);
        }
    } else {
        app.folder_dialog = Some(draft);
    }
}

pub(super) fn delete_folder_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let Some(folder_id) = app.folder_delete else {
        return;
    };
    let Some(folder) = crate::domain::find_folder(&app.hosts, folder_id).cloned() else {
        app.folder_delete = None;
        return;
    };
    let mut confirm = false;
    let mut close_requested = false;
    let warning = tr(lang, "delete_folder_warning")
        .replace("{name}", &folder.title)
        .replace("{hosts}", &folder.host_count().to_string())
        .replace("{folders}", &folder.folder_count().to_string());

    let (header_close, _) = dialog_window(
        ctx,
        "folder_delete_dialog",
        tr(lang, "delete_folder"),
        Icon::Trash,
        460.0,
        None,
        false,
        palette,
        |ui| {
            ui.label(
                RichText::new(&folder.title)
                    .size(16.0)
                    .strong()
                    .color(palette.text),
            );
            ui.add_space(6.0);
            ui.label(RichText::new(warning).size(13.0).color(palette.text_soft));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(tr(lang, "delete"))
                                .size(13.0)
                                .color(Color32::WHITE),
                        )
                        .fill(palette.red)
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
        app.folder_delete = None;
        return;
    }
    if confirm {
        app.delete_folder(folder_id);
        app.folder_delete = None;
    }
}
