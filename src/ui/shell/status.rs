use super::*;
use crate::domain::{find_host, ConnectionBackend, TabKind};

pub fn status_bar(app: &mut ShellCoreApp, ui: &mut Ui) {
    let palette = Palette::for_mode(app.settings.theme);
    let connected = app.connected_hosts.len();
    let sftp_channels = app
        .workspace
        .tab_count_by_kind(crate::domain::TabKind::Sftp);
    let tunnels = app
        .tunnels
        .iter()
        .filter(|tunnel| tunnel.state == crate::domain::TunnelState::Online)
        .count();
    let active_status = active_window_status(app);
    egui::Panel::bottom("system_status_bar")
        .exact_size(24.0)
        .frame(
            Frame::new()
                .fill(palette.panel_alt)
                .inner_margin(Margin::symmetric(9, 0)),
        )
        .show(ui, |ui| {
            let rect = ui.max_rect();
            ui.painter()
                .hline(rect.x_range(), rect.top(), Stroke::new(1.0, palette.border));
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                status_dot(
                    ui,
                    if connected > 0 {
                        palette.green
                    } else {
                        palette.muted
                    },
                    3.0,
                );
                ui.label(
                    RichText::new(format!("{connected} {}", app.t("sessions_online")))
                        .size(13.0)
                        .monospace()
                        .color(palette.green),
                );
                ui.label(
                    RichText::new(format!("{}: {tunnels}", app.t("tunnels_label")))
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                );
                ui.label(
                    RichText::new(format!("{}: {sftp_channels}", app.t("sftp_channels")))
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                );
                if let Some(error) = &app.storage_error {
                    ui.label(
                        RichText::new(format!("{}: {error}", app.t("storage")))
                            .size(13.0)
                            .monospace()
                            .color(palette.red),
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some((_, label, color)) = &active_status {
                        ui.label(RichText::new(*label).size(13.0).monospace().color(*color));
                    }
                    if let Some((detail, _, _)) = &active_status {
                        ui.label(
                            RichText::new(detail)
                                .size(13.0)
                                .monospace()
                                .color(palette.muted),
                        );
                    }
                    ui.label(
                        RichText::new("UTF-8 · LF (Unix)")
                            .size(13.0)
                            .monospace()
                            .color(palette.muted),
                    );
                    ui.label(
                        RichText::new(app.settings.language.as_str())
                            .size(13.0)
                            .monospace()
                            .color(palette.muted),
                    );
                });
            });
        });
}

fn active_window_status(app: &ShellCoreApp) -> Option<(String, &'static str, Color32)> {
    let palette = Palette::for_mode(app.settings.theme);
    let tab = app.workspace.active_tab()?;
    let host = find_host(&app.hosts, tab.host_id)?;
    let connected = app.connected_hosts.contains(&host.id);
    let detail = match tab.kind {
        TabKind::Terminal => {
            let (rows, cols) = tab.terminal.size();
            match host.backend {
                ConnectionBackend::Ssh => format!(
                    "SSH-2.0 (UTF-8)  ·  xterm-256color  ·  {cols}×{rows}  ·  keep-alive {}s",
                    host.keep_alive
                ),
                ConnectionBackend::LocalWsl => format!(
                    "WSL · ConPTY  ·  {}  ·  {cols}×{rows}",
                    host.wsl_distro.as_deref().unwrap_or("default")
                ),
                ConnectionBackend::Rdp => "RDP · framebuffer".to_owned(),
            }
        }
        TabKind::Sftp => format!("SFTP · {}", host.address()),
        TabKind::Rdp => format!("RDP · {}", host.address()),
    };
    let (label, color) = if connected {
        (
            match host.backend {
                ConnectionBackend::LocalWsl => "WSL",
                ConnectionBackend::Rdp => app.t("connected"),
                ConnectionBackend::Ssh => app.t("live_ssh"),
            },
            palette.green,
        )
    } else {
        match host.status {
            crate::domain::HostStatus::Connecting => (app.t("connecting"), palette.amber),
            crate::domain::HostStatus::Error => (app.t("connection_error"), palette.red),
            crate::domain::HostStatus::Offline | crate::domain::HostStatus::Online => {
                (app.t("offline"), palette.muted)
            }
        }
    };
    Some((detail, label, color))
}

pub fn resize_handles(app: &mut ShellCoreApp, ctx: &egui::Context) {
    let _ = app;
    let screen = ctx.content_rect();
    let t = 5.0;
    let handles = [
        (
            Rect::from_min_max(screen.min, Pos2::new(screen.max.x, screen.min.y + t)),
            egui::ResizeDirection::North,
            egui::CursorIcon::ResizeNorth,
        ),
        (
            Rect::from_min_max(
                Pos2::new(screen.min.x, screen.max.y - t),
                Pos2::new(screen.max.x, screen.max.y),
            ),
            egui::ResizeDirection::South,
            egui::CursorIcon::ResizeSouth,
        ),
        (
            Rect::from_min_max(screen.min, Pos2::new(screen.min.x + t, screen.max.y)),
            egui::ResizeDirection::West,
            egui::CursorIcon::ResizeWest,
        ),
        (
            Rect::from_min_max(Pos2::new(screen.max.x - t, screen.min.y), screen.max),
            egui::ResizeDirection::East,
            egui::CursorIcon::ResizeEast,
        ),
        (
            Rect::from_min_max(screen.min, screen.min + Vec2::splat(t)),
            egui::ResizeDirection::NorthWest,
            egui::CursorIcon::ResizeNorthWest,
        ),
        (
            Rect::from_min_max(
                Pos2::new(screen.max.x - t, screen.min.y),
                Pos2::new(screen.max.x, screen.min.y + t),
            ),
            egui::ResizeDirection::NorthEast,
            egui::CursorIcon::ResizeNorthEast,
        ),
        (
            Rect::from_min_max(
                Pos2::new(screen.min.x, screen.max.y - t),
                Pos2::new(screen.min.x + t, screen.max.y),
            ),
            egui::ResizeDirection::SouthWest,
            egui::CursorIcon::ResizeSouthWest,
        ),
        (
            Rect::from_min_max(Pos2::new(screen.max.x - t, screen.max.y - t), screen.max),
            egui::ResizeDirection::SouthEast,
            egui::CursorIcon::ResizeSouthEast,
        ),
    ];
    for (index, (rect, direction, cursor)) in handles.into_iter().enumerate() {
        egui::Area::new(egui::Id::unique(("window_resize", index)))
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(rect.size(), Sense::drag());
                if response.hovered() {
                    ui.ctx().set_cursor_icon(cursor);
                }
                if response.drag_started() {
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::BeginResize(direction));
                }
            });
    }
}
