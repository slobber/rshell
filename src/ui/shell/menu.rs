use super::*;

pub fn menus(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button(RichText::new(app.t("file")).size(13.0), |ui| {
            if menu_item(ui, palette, app.t("new_session"), "Ctrl+N") {
                app.open_new_session();
            }
            if menu_item(ui, palette, app.t("quick_connect"), "") {
                app.open_quick_connect();
            }
            ui.separator();
            if menu_item(ui, palette, app.t("new_terminal"), "Ctrl+T") {
                app.new_terminal_tab();
            }
            if menu_item(ui, palette, app.t("new_sftp"), "Ctrl+Shift+S") {
                app.new_sftp_tab();
            }
            if menu_item(ui, palette, app.t("close_tab"), "Ctrl+W") {
                app.close_focused_tab();
            }
            ui.separator();
            if menu_item(ui, palette, app.t("disconnect"), "") {
                if let Some(host) = app.active_host_clone() {
                    app.disconnect_host(host.id);
                }
            }
            if menu_item(ui, palette, app.t("reconnect"), "") {
                app.reconnect_active();
            }
            ui.separator();
            if menu_item(ui, palette, app.t("quit"), "Ctrl+Q") {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }
        });
        ui.menu_button(RichText::new(app.t("edit")).size(13.0), |ui| {
            if menu_item(ui, palette, app.t("copy_address"), "") {
                if let Some(host) = app.active_host() {
                    ui.ctx().copy_text(host.address());
                    app.toast(app.t("copied"));
                }
            }
            if menu_item(ui, palette, app.t("clear_terminal"), "") {
                if let Some(tab) = app.focused_tab_mut() {
                    tab.terminal.clear();
                    tab.input.clear();
                }
            }
            if menu_item(ui, palette, app.t("clear_logs"), "") {
                app.logs.clear();
                app.log_selection = None;
            }
        });
        ui.menu_button(RichText::new(app.t("view")).size(13.0), |ui| {
            if menu_item(ui, palette, app.t("toggle_theme"), "Ctrl+Shift+L") {
                app.toggle_theme(ui.ctx());
            }
            if menu_item(ui, palette, app.t("toggle_sidebar"), "") {
                app.settings.sidebar_collapsed = !app.settings.sidebar_collapsed;
                app.persist_settings();
            }
            if menu_item(ui, palette, app.t("toggle_console"), "Ctrl+`") {
                app.settings.console_collapsed = !app.settings.console_collapsed;
                app.persist_settings();
            }
            ui.separator();
            if menu_item(ui, palette, app.t("zoom_in"), "Ctrl++") {
                app.settings.terminal_zoom = (app.settings.terminal_zoom + 0.1).min(1.6);
                app.persist_settings();
            }
            if menu_item(ui, palette, app.t("zoom_out"), "Ctrl+-") {
                app.settings.terminal_zoom = (app.settings.terminal_zoom - 0.1).max(0.75);
                app.persist_settings();
            }
            if menu_item(ui, palette, app.t("zoom_reset"), "") {
                app.settings.terminal_zoom = 1.0;
                app.persist_settings();
            }
        });
        ui.menu_button(RichText::new(app.t("operation")).size(13.0), |ui| {
            if menu_item(ui, palette, app.t("split_columns"), "Ctrl+D") {
                app.split_active(Axis::Horizontal);
            }
            if menu_item(ui, palette, app.t("split_rows"), "Ctrl+Shift+D") {
                app.split_active(Axis::Vertical);
            }
            if menu_item(ui, palette, app.t("split_single"), "") {
                app.workspace = crate::domain::Workspace::new();
            }
            ui.separator();
            let broadcast_label = if app.broadcast {
                format!("✓ {}", app.t("broadcast"))
            } else {
                app.t("broadcast").to_owned()
            };
            if menu_item(ui, palette, &broadcast_label, "Ctrl+B") {
                app.broadcast = !app.broadcast;
            }
            if menu_item(ui, palette, app.t("open_sftp"), "") {
                app.open_or_focus_sftp();
            }
            if menu_item(ui, palette, app.t("upload_selected"), "") {
                if let Some(tab) = app.focused_tab() {
                    let id = tab.id;
                    app.queue_transfer(id, TransferDirection::Upload);
                }
            }
            if menu_item(ui, palette, app.t("download_selected"), "") {
                if let Some(tab) = app.focused_tab() {
                    let id = tab.id;
                    app.queue_transfer(id, TransferDirection::Download);
                }
            }
        });
        ui.menu_button(RichText::new(app.t("tools")).size(13.0), |ui| {
            if menu_item(ui, palette, app.t("command_palette"), "Ctrl+Shift+P") {
                app.command_palette_open = true;
            }
            if menu_item(ui, palette, app.t("keyring"), "") {
                app.keyring_open = true;
            }
            if menu_item(ui, palette, app.t("snippets"), "") {
                app.snippets_open = true;
            }
            if menu_item(ui, palette, app.t("tunnels"), "") {
                app.console_tab = ConsoleTab::Tunnels;
                app.settings.console_collapsed = false;
            }
            ui.separator();
            if menu_item(ui, palette, app.t("settings"), "Ctrl+,") {
                app.settings_open = true;
            }
        });
        ui.menu_button(RichText::new(app.t("window")).size(13.0), |ui| {
            if menu_item(ui, palette, app.t("split_columns"), "") {
                app.split_active(Axis::Horizontal);
            }
            if menu_item(ui, palette, app.t("split_rows"), "") {
                app.split_active(Axis::Vertical);
            }
            if menu_item(ui, palette, app.t("split_grid"), "") {
                app.split_active(Axis::Horizontal);
                app.split_active(Axis::Vertical);
            }
            if menu_item(ui, palette, app.t("split_single"), "") {
                app.workspace = crate::domain::Workspace::new();
            }
        });
        ui.menu_button(RichText::new(app.t("help")).size(13.0), |ui| {
            if menu_item(ui, palette, app.t("settings"), "") {
                app.settings_open = true;
            }
            if menu_item(ui, palette, app.t("about"), "") {
                app.about_open = true;
            }
        });
    });
}

fn menu_item(ui: &mut Ui, palette: Palette, label: &str, shortcut: &str) -> bool {
    let text = if shortcut.is_empty() {
        label.to_owned()
    } else {
        format!("{label}\t{shortcut}")
    };
    let button = egui::Button::new(RichText::new(text).size(13.0).color(palette.text_soft))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .min_size(Vec2::new(240.0, 30.0));
    let clicked = ui.add(button).clicked();
    if clicked {
        ui.close();
    }
    clicked
}
