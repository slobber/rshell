use eframe::egui::{
    self, Align, CornerRadius, FontId, Layout, Rect, RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};

use crate::app::ShellCoreApp;
use crate::domain::{find_host, FileEntry, SessionTab, TransferDirection};
use crate::icons::Icon;
use crate::theme::Palette;
use crate::ui::components::icon_button;

mod actions;
mod browser;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SftpPaneSide {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub(super) struct SftpDragPayload {
    pub(super) source: SftpPaneSide,
    pub(super) names: Vec<String>,
}

pub fn draw(app: &mut ShellCoreApp, ui: &mut Ui, tab: &mut SessionTab, palette: Palette) {
    let Some(host) = find_host(&app.hosts, tab.host_id).cloned() else {
        return;
    };
    let mut connected_hosts = app.connected_ssh_hosts();
    let left_host = tab
        .left_host_id
        .and_then(|host_id| find_host(&app.hosts, host_id).cloned());
    let left_remote = tab.left_host_id.is_some();
    if let Some(source) = &left_host {
        if !connected_hosts.iter().any(|(id, _)| *id == source.id) {
            connected_hosts.push((source.id, source.name.clone()));
        }
    }
    let mut right_choices = connected_hosts.clone();
    if !right_choices.iter().any(|(id, _)| *id == host.id) {
        right_choices.push((host.id, host.name.clone()));
    }
    let available = ui.available_size();
    let bottom_height = 34.0;
    let content_height = (available.y - bottom_height).max(80.0);

    ui.allocate_ui_with_layout(
        Vec2::new(available.x, content_height),
        Layout::top_down(Align::Min),
        |ui| {
            let body_height = ui.available_height();
            let total_width = ui.available_width();
            let center_width = 43.0;
            let pane_width = ((total_width - center_width) * 0.5).max(120.0);
            let mut local_action = browser::BrowserAction::default();
            let mut remote_action = browser::BrowserAction::default();
            let mut upload = false;
            let mut download = false;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.allocate_ui_with_layout(
                    Vec2::new(pane_width, body_height),
                    Layout::top_down(Align::Min),
                    |ui| {
                        local_action = browser::browser_pane(
                            ui,
                            app,
                            SftpPaneSide::Left,
                            &tab.local_cwd,
                            &mut tab.local_path_input,
                            &tab.local_entries,
                            &mut tab.local_selected,
                            &mut tab.local_anchor,
                            &mut tab.local_columns,
                            left_remote,
                            left_host.as_ref(),
                            Some(&connected_hosts),
                            true,
                            tab.left_loading,
                            tab.local_error.as_deref(),
                            palette,
                        );
                    },
                );
                ui.allocate_ui_with_layout(
                    Vec2::new(center_width, body_height),
                    Layout::top_down(Align::Center),
                    |ui| {
                        ui.set_min_size(Vec2::new(center_width, body_height));
                        ui.set_max_size(Vec2::new(center_width, body_height));
                        let rect = ui.max_rect();
                        ui.painter()
                            .rect_filled(rect, CornerRadius::ZERO, palette.panel);
                        ui.painter().vline(
                            rect.left(),
                            rect.y_range(),
                            Stroke::new(1.0, palette.border),
                        );
                        ui.painter().vline(
                            rect.right(),
                            rect.y_range(),
                            Stroke::new(1.0, palette.border),
                        );
                        ui.add_space((body_height * 0.33).max(40.0));
                        if icon_button(
                            ui,
                            palette,
                            Icon::ChevronsRight,
                            18.0,
                            app.t("upload"),
                            !tab.local_selected.is_empty(),
                        )
                        .clicked()
                        {
                            upload = true;
                        }
                        ui.add_space(4.0);
                        if icon_button(
                            ui,
                            palette,
                            Icon::ChevronsLeft,
                            18.0,
                            app.t("download"),
                            !tab.remote_selected.is_empty(),
                        )
                        .clicked()
                        {
                            download = true;
                        }
                        ui.add_space(8.0);
                        if icon_button(
                            ui,
                            palette,
                            Icon::CirclesRelation,
                            16.0,
                            app.t("sync_browse"),
                            tab.sync_browse,
                        )
                        .clicked()
                        {
                            tab.sync_browse = !tab.sync_browse;
                        }
                    },
                );
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), body_height),
                    Layout::top_down(Align::Min),
                    |ui| {
                        remote_action = browser::browser_pane(
                            ui,
                            app,
                            SftpPaneSide::Right,
                            &tab.remote_cwd,
                            &mut tab.remote_path_input,
                            &tab.remote_entries,
                            &mut tab.remote_selected,
                            &mut tab.remote_anchor,
                            &mut tab.remote_columns,
                            true,
                            Some(&host),
                            Some(&right_choices),
                            false,
                            tab.remote_loading,
                            tab.remote_error.as_deref(),
                            palette,
                        );
                    },
                );
            });

            if let Some(source) = local_action.source_change {
                app.set_sftp_left_source(tab, source);
            } else {
                if local_action.go_up {
                    if left_remote {
                        tab.local_cwd = browser::parent_path(&tab.local_cwd, true);
                        tab.local_selected.clear();
                        tab.local_anchor = None;
                        request_left_directory(app, tab);
                    } else {
                        tab.local_cwd = crate::local_fs::parent(&tab.local_cwd);
                        tab.local_selected.clear();
                        tab.local_anchor = None;
                        crate::app::sessions::refresh_local_directory(tab);
                    }
                }
                if let Some(path) = local_action.navigate {
                    tab.local_cwd = path;
                    tab.local_selected.clear();
                    tab.local_anchor = None;
                    if left_remote {
                        request_left_directory(app, tab);
                    } else {
                        crate::app::sessions::refresh_local_directory(tab);
                    }
                }
                if let Some(dir) = local_action.open_dir {
                    tab.local_cwd = if left_remote {
                        browser::join_path(&tab.local_cwd, &dir)
                    } else {
                        crate::local_fs::join(&tab.local_cwd, &dir)
                    };
                    tab.local_selected.clear();
                    tab.local_anchor = None;
                    if left_remote {
                        request_left_directory(app, tab);
                    } else {
                        crate::app::sessions::refresh_local_directory(tab);
                    }
                }
                if local_action.refresh {
                    if left_remote {
                        request_left_directory(app, tab);
                    } else {
                        crate::app::sessions::refresh_local_directory(tab);
                    }
                }
                if let Some(name) = local_action.open_file {
                    tab.local_selected.clear();
                    tab.local_selected.push(name.clone());
                    tab.local_anchor = None;
                    let path = if left_remote {
                        browser::join_path(&tab.local_cwd, &name)
                    } else {
                        crate::local_fs::join(&tab.local_cwd, &name)
                    };
                    if left_remote {
                        if let Some(session_id) = tab.left_session_id {
                            app.editor_dialog = Some(crate::app::EditorDialog {
                                path: path.clone(),
                                remote: true,
                                tab_id: Some(session_id),
                                content: "Loading remote file…".to_owned(),
                                truncated: false,
                            });
                            app.ssh.read_remote_file(session_id, path);
                        }
                    } else {
                        match crate::local_fs::read_text(&path, 1024 * 1024) {
                            Ok((content, truncated)) => {
                                app.editor_dialog = Some(crate::app::EditorDialog {
                                    path,
                                    remote: false,
                                    tab_id: Some(tab.id),
                                    content,
                                    truncated,
                                });
                            }
                            Err(error) => app.log(crate::domain::LogLevel::Error, error),
                        }
                    }
                }
            }
            if let Some(host_id) = remote_action.source_change.flatten() {
                app.set_sftp_right_host(tab, host_id);
            }
            if remote_action.go_up {
                tab.remote_cwd = browser::parent_path(&tab.remote_cwd, true);
                tab.remote_selected.clear();
                tab.remote_anchor = None;
                request_remote_directory(app, tab);
            }
            if let Some(path) = remote_action.navigate {
                tab.remote_cwd = path;
                tab.remote_selected.clear();
                tab.remote_anchor = None;
                request_remote_directory(app, tab);
            }
            if let Some(dir) = remote_action.open_dir {
                tab.remote_cwd = browser::join_path(&tab.remote_cwd, &dir);
                tab.remote_selected.clear();
                tab.remote_anchor = None;
                request_remote_directory(app, tab);
            }
            if remote_action.refresh {
                request_remote_directory(app, tab);
            }
            if let Some(name) = remote_action.open_file {
                tab.remote_selected.clear();
                tab.remote_selected.push(name.clone());
                tab.remote_anchor = None;
                app.editor_dialog = Some(crate::app::EditorDialog {
                    path: browser::join_path(&tab.remote_cwd, &name),
                    remote: true,
                    tab_id: Some(tab.id),
                    content: "Loading remote file…".to_owned(),
                    truncated: false,
                });
                app.ssh
                    .read_remote_file(tab.id, browser::join_path(&tab.remote_cwd, &name));
            }
            if let Some(payload) = local_action.drop_files {
                if payload.source == SftpPaneSide::Right {
                    app.queue_transfers_for_tab(tab, TransferDirection::Download, &payload.names);
                }
            }
            if let Some(payload) = remote_action.drop_files {
                if payload.source == SftpPaneSide::Left {
                    app.queue_transfers_for_tab(tab, TransferDirection::Upload, &payload.names);
                }
            }
            if upload {
                app.queue_transfer_for_tab(tab, TransferDirection::Upload);
            }
            if download {
                app.queue_transfer_for_tab(tab, TransferDirection::Download);
            }
        },
    );

    actions::action_bar(app, ui, tab, palette);
}

fn request_remote_directory(app: &mut ShellCoreApp, tab: &mut SessionTab) {
    tab.remote_loading = true;
    tab.remote_error = None;
    tab.remote_request_path = tab.remote_cwd.clone();
    if !app.ssh.list_remote(tab.id, tab.remote_cwd.clone()) {
        tab.remote_loading = false;
        tab.remote_error = Some("SFTP channel is not available".to_owned());
    }
}

fn request_left_directory(app: &mut ShellCoreApp, tab: &mut SessionTab) {
    let Some(session_id) = tab.left_session_id else {
        return;
    };
    tab.left_loading = true;
    tab.local_error = None;
    tab.left_request_path = tab.local_cwd.clone();
    if !app.ssh.list_remote(session_id, tab.local_cwd.clone()) {
        tab.left_loading = false;
        tab.local_error = Some("SFTP channel is not available".to_owned());
    }
}
