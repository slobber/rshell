use super::*;
use std::time::{Duration, Instant};

pub fn quick_ribbon(app: &mut ShellCoreApp, ui: &mut Ui) {
    let palette = Palette::for_mode(app.settings.theme);
    let window_choices = app
        .workspace
        .all_tabs()
        .into_iter()
        .map(|tab| (tab.id, tab.title.clone()))
        .collect::<Vec<_>>();
    let current_tab = app.workspace.active_tab().cloned();
    let current_host = current_tab
        .as_ref()
        .and_then(|tab| crate::domain::find_host(&app.hosts, tab.host_id).cloned())
        .or_else(|| app.active_host_clone());
    let connected = current_host
        .as_ref()
        .is_some_and(|host| app.connected_hosts.contains(&host.id));

    egui::Panel::top("quick_ribbon")
        .exact_size(36.0)
        .show_separator_line(false)
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
                    if let Some(host) = &current_host {
                        let address = if host.user.trim().is_empty() {
                            host.address()
                        } else {
                            format!("{}@{}:{}", host.user, host.ip, host.port)
                        };
                        let active_tab_id = current_tab.as_ref().map(|tab| tab.id);
                        let copied = active_tab_id.is_some_and(|tab_id| {
                            app.copy_feedback.as_ref().is_some_and(|(copied_tab, at)| {
                                *copied_tab == tab_id && at.elapsed() < Duration::from_secs(1)
                            })
                        });
                        let copy = icon_button(
                            ui,
                            palette,
                            if copied {
                                Icon::CircleCheck
                            } else {
                                Icon::Copy
                            },
                            13.0,
                            if copied {
                                app.t("copied")
                            } else {
                                app.t("copy_address")
                            },
                            copied,
                        )
                        .clicked();
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&address)
                                .size(crate::theme::STATUS_FONT_SIZE)
                                .monospace()
                                .color(palette.text_soft),
                        );
                        if copy {
                            if let Some(tab_id) = active_tab_id {
                                app.copy_feedback = Some((tab_id, Instant::now()));
                                ui.ctx().request_repaint();
                                ui.ctx().request_repaint_after(Duration::from_secs(1));
                            }
                            ui.ctx().copy_text(address);
                        }
                    }
                });
            });
        });
}
