use super::*;

pub fn quick_ribbon(app: &mut ShellCoreApp, ui: &mut Ui) {
    let palette = Palette::for_mode(app.settings.theme);
    let window_choices = app
        .workspace
        .all_tabs()
        .into_iter()
        .map(|tab| (tab.id, tab.title.clone()))
        .collect::<Vec<_>>();
    let current_tab = app.workspace.active_tab().cloned();
    let current_host = app.active_host_clone();
    let connected = current_host
        .as_ref()
        .is_some_and(|host| app.connected_hosts.contains(&host.id));

    egui::Panel::top("quick_ribbon")
        .exact_size(36.0)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(9, 0)),
        )
        .show(ui, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                if icon_button(
                    ui,
                    palette,
                    if app.settings.sidebar_collapsed {
                        Icon::LayoutSidebarCollapseRight
                    } else {
                        Icon::LayoutSidebarCollapseLeft
                    },
                    14.0,
                    app.t("toggle_sidebar"),
                    false,
                )
                .clicked()
                {
                    app.settings.sidebar_collapsed = !app.settings.sidebar_collapsed;
                    app.persist_settings();
                }
                ui.separator();
                let mut selected = current_tab.as_ref().map(|tab| tab.id);
                let selected_text = current_tab
                    .as_ref()
                    .map(|tab| tab.title.clone())
                    .unwrap_or_else(|| app.t("no_active_terminal").to_owned());
                ComboBox::from_id_salt("quick_host_selector")
                    .icon(combo_icon)
                    .selected_text(
                        RichText::new(selected_text)
                            .size(13.0)
                            .monospace()
                            .color(palette.text),
                    )
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for (id, title) in &window_choices {
                            ui.selectable_value(
                                &mut selected,
                                Some(*id),
                                RichText::new(title).size(13.0),
                            );
                        }
                    });
                if selected != current_tab.as_ref().map(|tab| tab.id) {
                    if let Some(tab_id) = selected {
                        app.activate_tab(tab_id);
                    }
                }
                if icon_button(ui, palette, Icon::Plug, 14.0, app.t("connect"), connected).clicked()
                {
                    app.reconnect_active();
                }
                if icon_button(ui, palette, Icon::PlugX, 14.0, app.t("disconnect"), false).clicked()
                {
                    if let Some(host) = app.active_host_clone() {
                        app.disconnect_host(host.id);
                    }
                }
                if icon_button(ui, palette, Icon::Refresh, 14.0, app.t("reconnect"), false)
                    .clicked()
                {
                    app.reconnect_active();
                }
                ui.separator();
                if labeled_icon_button(
                    ui,
                    palette,
                    Icon::Code,
                    app.t("snippets"),
                    false,
                    palette.accent,
                )
                .clicked()
                {
                    app.snippets_open = true;
                }
                ui.separator();
                if icon_button(ui, palette, Icon::Minus, 18.0, app.t("zoom_out"), false).clicked() {
                    app.settings.terminal_zoom = (app.settings.terminal_zoom - 0.1).max(0.75);
                    app.persist_settings();
                }
                ui.label(
                    RichText::new(format!("{:.0}%", app.settings.terminal_zoom * 100.0))
                        .size(13.0)
                        .monospace()
                        .color(palette.text_soft),
                );
                if icon_button(ui, palette, Icon::Plus, 18.0, app.t("zoom_in"), false).clicked() {
                    app.settings.terminal_zoom = (app.settings.terminal_zoom + 0.1).min(1.6);
                    app.persist_settings();
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(app.t("scrollback_lines"))
                            .size(crate::theme::STATUS_FONT_SIZE)
                            .monospace()
                            .color(palette.muted),
                    );
                    let transport_label = if connected {
                        if current_host.as_ref().is_some_and(|host| {
                            host.backend == crate::domain::ConnectionBackend::LocalWsl
                        }) {
                            app.t("live_wsl")
                        } else if current_host.as_ref().is_some_and(|host| {
                            host.backend == crate::domain::ConnectionBackend::Rdp
                        }) {
                            app.t("live_rdp")
                        } else {
                            app.t("live_ssh")
                        }
                    } else {
                        app.t("no_transport")
                    };
                    ui.label(
                        RichText::new(transport_label)
                            .size(crate::theme::STATUS_FONT_SIZE)
                            .monospace()
                            .color(if connected {
                                palette.cyan
                            } else {
                                palette.muted
                            }),
                    );
                    if let Some(host) = &current_host {
                        ui.label(
                            RichText::new(status_text(host.status, app))
                                .size(crate::theme::STATUS_FONT_SIZE)
                                .monospace()
                                .color(status_color(host.status, palette)),
                        );
                    }
                });
            });
        });
}

fn status_text(status: HostStatus, app: &ShellCoreApp) -> &'static str {
    match status {
        HostStatus::Offline => app.t("not_connected"),
        HostStatus::Connecting => app.t("connecting"),
        HostStatus::Online => app.t("connected"),
        HostStatus::Error => app.t("connection_error"),
    }
}

fn status_color(status: HostStatus, palette: Palette) -> Color32 {
    match status {
        HostStatus::Online => palette.green,
        HostStatus::Connecting => palette.amber,
        HostStatus::Offline => palette.muted,
        HostStatus::Error => palette.red,
    }
}
