use super::browser::join_path;
use super::*;

pub(super) fn action_bar(
    app: &mut ShellCoreApp,
    ui: &mut Ui,
    tab: &mut SessionTab,
    palette: Palette,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.panel_alt);
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, palette.border));
    let (selected, selected_count) = if !tab.remote_selected.is_empty() {
        (
            tab.remote_selected.last().cloned(),
            tab.remote_selected.len(),
        )
    } else {
        (tab.local_selected.last().cloned(), tab.local_selected.len())
    };
    let left_remote = tab.left_host_id.is_some();
    let label = match (&selected, selected_count) {
        (Some(name), 1) => format!("{}: {name}", app.t("selected")),
        (Some(_), count) => format!("{} {count}", app.t("selected_items")),
        _ => app.t("nothing_selected").to_owned(),
    };
    ui.painter().text(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::monospace(13.0),
        palette.text_soft,
    );
    let button_width = 92.0;
    let mut x = rect.right() - 8.0;
    for (label, action) in [
        (app.t("hash"), 2),
        (app.t("chmod"), 1),
        (app.t("edit_remote"), 0),
    ] {
        x -= button_width;
        let button_rect = Rect::from_min_size(
            egui::pos2(x, rect.top() + 3.0),
            Vec2::new(button_width - 4.0, 26.0),
        );
        let response = ui.interact(
            button_rect,
            egui::Id::unique(("sftp_action", tab.id, action)),
            Sense::click(),
        );
        ui.painter().rect_filled(
            button_rect,
            CornerRadius::same(3),
            if response.hovered() {
                palette.hover
            } else {
                palette.raised
            },
        );
        ui.painter().rect_stroke(
            button_rect,
            CornerRadius::same(3),
            Stroke::new(1.0, palette.border),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            button_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(13.0),
            palette.text_soft,
        );
        if response.clicked() {
            match action {
                0 => {
                    if let Some(name) = tab.remote_selected.last() {
                        let path = join_path(&tab.remote_cwd, name);
                        app.editor_dialog = Some(crate::app::EditorDialog {
                            path: path.clone(),
                            remote: true,
                            tab_id: Some(tab.id),
                            content: "Loading remote file…".to_owned(),
                            truncated: false,
                        });
                        app.ssh.read_remote_file(tab.id, path);
                    } else if left_remote {
                        if let (Some(name), Some(session_id)) =
                            (tab.local_selected.last().cloned(), tab.left_session_id)
                        {
                            let path = join_path(&tab.local_cwd, &name);
                            app.editor_dialog = Some(crate::app::EditorDialog {
                                path: path.clone(),
                                remote: true,
                                tab_id: Some(session_id),
                                content: "Loading remote file…".to_owned(),
                                truncated: false,
                            });
                            app.ssh.read_remote_file(session_id, path);
                        } else {
                            app.toast(app.t("select_file_first"));
                        }
                    } else {
                        app.toast(app.t("select_file_first"));
                    }
                }
                1 => {
                    if let Some(name) = tab.remote_selected.last() {
                        app.chmod_dialog = Some(crate::app::ChmodDialog {
                            path: join_path(&tab.remote_cwd, name),
                            mode: "644".to_owned(),
                            tab_id: Some(tab.id),
                        });
                    } else if left_remote {
                        if let (Some(name), Some(session_id)) =
                            (tab.local_selected.last().cloned(), tab.left_session_id)
                        {
                            app.chmod_dialog = Some(crate::app::ChmodDialog {
                                path: join_path(&tab.local_cwd, &name),
                                mode: "644".to_owned(),
                                tab_id: Some(session_id),
                            });
                        } else {
                            app.toast(app.t("select_file_first"));
                        }
                    } else {
                        app.toast(app.t("select_file_first"));
                    }
                }
                _ => {
                    if let Some(name) = tab.remote_selected.last().cloned() {
                        let path = join_path(&tab.remote_cwd, &name);
                        app.hash_dialog = Some(crate::app::HashDialog {
                            path: path.clone(),
                            hashes: None,
                            copied: None,
                        });
                        app.ssh.hash_remote(tab.id, path);
                    } else if left_remote {
                        if let (Some(name), Some(session_id)) =
                            (tab.local_selected.last().cloned(), tab.left_session_id)
                        {
                            let path = join_path(&tab.local_cwd, &name);
                            app.hash_dialog = Some(crate::app::HashDialog {
                                path: path.clone(),
                                hashes: None,
                                copied: None,
                            });
                            app.ssh.hash_remote(session_id, path);
                        } else {
                            app.toast(app.t("select_file_first"));
                        }
                    } else if let Some(name) = tab.local_selected.last().cloned() {
                        let path = crate::local_fs::join(&tab.local_cwd, &name);
                        match crate::ssh::hash_local(&path) {
                            Ok(hashes) => {
                                app.hash_dialog = Some(crate::app::HashDialog {
                                    path,
                                    hashes: Some(hashes),
                                    copied: None,
                                });
                            }
                            Err(error) => app.log(crate::domain::LogLevel::Error, error),
                        }
                    } else {
                        app.toast(app.t("select_file_first"));
                    }
                }
            }
        }
    }
}
