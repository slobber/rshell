use super::*;
pub(super) fn tab_strip(
    app: &mut ShellCoreApp,
    ui: &mut Ui,
    pane: &mut Pane,
    focused_pane: &mut u64,
    palette: Palette,
) {
    let strip_height = 32.0;
    let (strip_rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), strip_height),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(strip_rect, CornerRadius::ZERO, palette.canvas);
    ui.painter().hline(
        strip_rect.x_range(),
        strip_rect.bottom(),
        Stroke::new(1.0, palette.border),
    );

    let tools_width = if pane.tabs.is_empty() { 0.0 } else { 104.0 };
    let tabs_width = (strip_rect.width() - tools_width).max(90.0);
    let tabs_rect = Rect::from_min_size(strip_rect.min, Vec2::new(tabs_width, strip_rect.height()));
    let mut close_tab = None;
    let mut select_tab = None;
    let mut start_drag = None;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(tabs_rect)
            .layout(Layout::left_to_right(Align::Min)),
        |ui| {
            ScrollArea::horizontal()
                .id_salt(("pane_tabs", pane.id))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        for (index, tab) in pane.tabs.iter().enumerate() {
                            let connected = app.connected_hosts.contains(&tab.host_id);
                            let response =
                                draw_tab(ui, tab, index == pane.active, connected, palette);
                            if response.drag_started() {
                                start_drag = Some(TabDrag {
                                    tab_id: tab.id,
                                    source_pane: pane.id,
                                    title: tab.title.clone(),
                                });
                            }
                            if response.clicked() {
                                select_tab = Some((index, tab.id));
                            }
                            if let Some(close_response) = response.close {
                                if close_response.clicked() {
                                    close_tab = Some(tab.id);
                                }
                            }
                        }
                        if !pane.tabs.is_empty() {
                            ui.add_space(3.0);
                            if icon_button(
                                ui,
                                palette,
                                Icon::Plus,
                                13.0,
                                app.t("new_terminal"),
                                false,
                            )
                            .clicked()
                            {
                                if let Some(tab) = create_terminal_tab(app) {
                                    app.start_session(&tab);
                                    pane.add_tab(tab);
                                    *focused_pane = pane.id;
                                }
                            }
                            if icon_button(
                                ui,
                                palette,
                                Icon::SwitchHorizontal,
                                13.0,
                                app.t("new_sftp"),
                                false,
                            )
                            .clicked()
                            {
                                if let Some(tab) = create_sftp_tab(app) {
                                    app.start_session(&tab);
                                    pane.add_tab(tab);
                                    *focused_pane = pane.id;
                                }
                            }
                        }
                    });
                });
        },
    );

    if !pane.tabs.is_empty() {
        let tools_rect = Rect::from_min_max(
            egui::pos2(strip_rect.right() - tools_width, strip_rect.top()),
            egui::pos2(strip_rect.right() - 8.0, strip_rect.bottom()),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(tools_rect)
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let pane_maximized = app.maximized_pane == Some(pane.id);
                let single_pane = matches!(app.workspace.root, SplitNode::Leaf(_));
                let window_maximized = ui
                    .ctx()
                    .input(|input| input.viewport().maximized.unwrap_or(false));
                let active_tab = pane.active_tab().map(|tab| (tab.id, tab.kind));
                if let Some((tab_id, _)) = active_tab.filter(|(_, kind)| *kind == TabKind::Rdp) {
                    if icon_button(
                        ui,
                        palette,
                        Icon::WindowMaximize,
                        13.0,
                        app.t("rdp_fullscreen"),
                        false,
                    )
                    .clicked()
                    {
                        app.enter_rdp_fullscreen(tab_id, ui.ctx());
                    }
                } else if icon_button(
                    ui,
                    palette,
                    if single_pane {
                        if window_maximized {
                            Icon::Square
                        } else {
                            Icon::WindowMaximize
                        }
                    } else if pane_maximized {
                        Icon::Minimize
                    } else {
                        Icon::Maximize
                    },
                    13.0,
                    if single_pane {
                        if window_maximized {
                            app.t("restore_window")
                        } else {
                            app.t("maximize_window")
                        }
                    } else {
                        app.t("maximize_pane")
                    },
                    if single_pane {
                        window_maximized
                    } else {
                        pane_maximized
                    },
                )
                .clicked()
                {
                    if single_pane {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Maximized(!window_maximized));
                    } else {
                        app.maximized_pane = if pane_maximized { None } else { Some(pane.id) };
                    }
                }
                if icon_button(
                    ui,
                    palette,
                    Icon::LayoutColumns,
                    13.0,
                    app.t("split_columns"),
                    false,
                )
                .clicked()
                {
                    *focused_pane = pane.id;
                    app.pending_split = Some(Axis::Horizontal);
                }
                if icon_button(
                    ui,
                    palette,
                    Icon::Search,
                    13.0,
                    app.t("terminal_search"),
                    false,
                )
                .clicked()
                {
                    app.command_palette_open = true;
                }
            },
        );
    }

    if let Some((index, tab_id)) = select_tab {
        pane.active = index;
        *focused_pane = pane.id;
        app.active_host_id = Some(tab_id_host(pane, tab_id));
    }
    if let Some(drag) = start_drag {
        app.drag = Some(drag);
        client_focus(app, pane.id, focused_pane);
    }
    if let Some(tab_id) = close_tab {
        app.ssh.close_tab(tab_id);
        if let Some(index) = pane.tabs.iter().position(|tab| tab.id == tab_id) {
            let removed = pane.tabs.remove(index);
            if pane.active >= pane.tabs.len() {
                pane.active = pane.tabs.len().saturating_sub(1);
            }
            app.log(
                crate::domain::LogLevel::Info,
                format!("Closed tab: {}", removed.title),
            );
        }
        app.drag = None;
    }
}

struct TabVisualResponse {
    response: egui::Response,
    close: Option<egui::Response>,
}

impl std::ops::Deref for TabVisualResponse {
    type Target = egui::Response;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

fn draw_tab(
    ui: &mut Ui,
    tab: &SessionTab,
    active: bool,
    connected: bool,
    palette: Palette,
) -> TabVisualResponse {
    let width = (tab.title.chars().count() as f32 * 6.6 + 58.0).clamp(120.0, 220.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click_and_drag());
    let fill = if active {
        palette.panel_alt
    } else if response.hovered() {
        palette.hover
    } else {
        palette.panel
    };
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(rect.width() - 1.0, rect.height())),
        CornerRadius::ZERO,
        fill,
    );
    if active {
        let accent = match tab.kind {
            TabKind::Sftp => palette.purple,
            TabKind::Rdp => palette.cyan,
            TabKind::Terminal => palette.accent,
        };
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(rect.width(), 2.0)),
            CornerRadius::ZERO,
            accent,
        );
    }
    ui.painter().vline(
        rect.right() - 0.5,
        rect.y_range(),
        Stroke::new(1.0, palette.border),
    );
    let dot = match (tab.kind, connected) {
        (TabKind::Sftp, _) => palette.purple,
        (TabKind::Rdp, _) => palette.cyan,
        (_, true) => palette.green,
        _ => palette.muted,
    };
    ui.painter()
        .circle_filled(egui::pos2(rect.left() + 9.0, rect.center().y), 2.5, dot);
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 22.0, rect.center().y),
        Vec2::splat(16.0),
    );
    crate::icons::paint(
        ui,
        match tab.kind {
            TabKind::Sftp => Icon::SwitchHorizontal,
            TabKind::Rdp => Icon::Desktop,
            TabKind::Terminal => Icon::Terminal,
        },
        icon_rect,
        if active {
            palette.accent
        } else {
            palette.muted
        },
    );
    let title_clip = Rect::from_min_max(
        egui::pos2(rect.left() + 30.0, rect.top()),
        egui::pos2(rect.right() - 36.0, rect.bottom()),
    );
    ui.painter().with_clip_rect(title_clip).text(
        egui::pos2(rect.left() + 30.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &tab.title,
        FontId::proportional(13.0),
        if active {
            palette.text
        } else {
            palette.text_soft
        },
    );
    let close_rect = Rect::from_center_size(
        egui::pos2(rect.right() - 14.0, rect.center().y),
        Vec2::splat(22.0),
    );
    let close = ui.interact(
        close_rect,
        egui::Id::unique(("close_tab", tab.id)),
        Sense::click(),
    );
    if close.hovered() {
        ui.painter()
            .rect_filled(close_rect, CornerRadius::same(3), palette.raised);
    }
    crate::icons::paint(
        ui,
        Icon::X,
        close_rect.shrink(3.0),
        if close.hovered() {
            palette.red
        } else {
            palette.muted
        },
    );
    TabVisualResponse {
        response,
        close: Some(close),
    }
}

fn client_focus(app: &mut ShellCoreApp, pane_id: u64, focused_pane: &mut u64) {
    *focused_pane = pane_id;
    let _ = app;
}

fn tab_id_host(pane: &Pane, tab_id: u64) -> u64 {
    pane.tabs
        .iter()
        .find(|tab| tab.id == tab_id)
        .map(|tab| tab.host_id)
        .unwrap_or(0)
}

pub(super) fn create_terminal_tab(app: &mut ShellCoreApp) -> Option<SessionTab> {
    let Some(host) = app.active_host_clone() else {
        app.open_new_session();
        return None;
    };
    app.ensure_connected(host.id);
    let id = app.allocate_tab_id();
    Some(SessionTab::terminal(id, &host))
}

pub(super) fn create_sftp_tab(app: &mut ShellCoreApp) -> Option<SessionTab> {
    let Some(host) = app.active_host_clone() else {
        app.open_new_session();
        return None;
    };
    app.ensure_connected(host.id);
    let id = app.allocate_tab_id();
    let mut tab = SessionTab::sftp(id, &host);
    tab.remote_cwd.clear();
    tab.local_cwd = crate::local_fs::display_path(&crate::local_fs::home_dir());
    crate::app::sessions::refresh_local_directory(&mut tab);
    tab.remote_loading = true;
    tab.remote_request_path = tab.remote_cwd.clone();
    app.log(
        crate::domain::LogLevel::Info,
        format!("SFTP channel opened for {}", host.address()),
    );
    Some(tab)
}
