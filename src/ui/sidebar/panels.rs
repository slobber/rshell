use super::*;

pub(super) fn draw_tunnels(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    if app.tunnels.is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            crate::icons::image(ui, Icon::Network, 26.0, palette.muted);
            ui.label(
                RichText::new(app.t("no_active_tunnels"))
                    .size(13.0)
                    .color(palette.text),
            );
            ui.label(
                RichText::new(app.t("tunnel_creation_phase"))
                    .size(13.0)
                    .color(palette.muted),
            );
        });
        return;
    }
    for tunnel in &app.tunnels {
        Frame::new()
            .fill(palette.panel)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(Margin::same(6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    status_dot(
                        ui,
                        match tunnel.state {
                            TunnelState::Online => palette.green,
                            TunnelState::Reconnecting => palette.amber,
                            TunnelState::Stopped => palette.red,
                        },
                        3.0,
                    );
                    ui.label(RichText::new(&tunnel.name).size(13.0).color(palette.text));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(&tunnel.traffic)
                                .size(13.0)
                                .monospace()
                                .color(palette.muted),
                        );
                    });
                });
                ui.label(
                    RichText::new(format!("{} → {}", tunnel.local, tunnel.remote))
                        .size(13.0)
                        .monospace()
                        .color(palette.cyan),
                );
            });
        ui.add_space(3.0);
    }
}

pub(super) fn properties(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    let mut edit_host_id = None;
    Frame::new()
        .fill(palette.panel)
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.spacing_mut().interact_size.y = 17.0;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}:", app.t("properties")))
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                );
                if let Some(host) = app.active_host() {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(if app.connected_hosts.contains(&host.id) {
                                app.t("connected")
                            } else {
                                app.t("not_connected")
                            })
                            .size(13.0)
                            .monospace()
                            .color(
                                if app.connected_hosts.contains(&host.id) {
                                    palette.green
                                } else {
                                    palette.muted
                                },
                            ),
                        );
                        if icon_button(
                            ui,
                            palette,
                            Icon::Pencil,
                            14.0,
                            app.t("edit_connection"),
                            false,
                        )
                        .clicked()
                        {
                            edit_host_id = Some(host.id);
                        }
                    });
                }
            });
            if let Some(host) = app.active_host_clone() {
                if host.backend == crate::domain::ConnectionBackend::LocalWsl {
                    property_row(ui, palette, app.t("host"), &host.name);
                    property_row(ui, palette, app.t("connection_type"), app.t("backend_wsl"));
                    property_row(
                        ui,
                        palette,
                        app.t("wsl_distro"),
                        host.wsl_distro.as_deref().unwrap_or("default"),
                    );
                    property_row(ui, palette, app.t("environment"), &host.environment);
                } else if host.backend == crate::domain::ConnectionBackend::Rdp {
                    property_row(ui, palette, app.t("host"), &host.ip);
                    property_row(ui, palette, app.t("port"), &format!("{} (RDP)", host.port));
                    property_row(
                        ui,
                        palette,
                        app.t("user"),
                        if host.user.is_empty() {
                            "—"
                        } else {
                            host.user.as_str()
                        },
                    );
                    property_row(
                        ui,
                        palette,
                        app.t("rdp_domain"),
                        if host.gateway.is_empty() {
                            "—"
                        } else {
                            host.gateway.as_str()
                        },
                    );
                    property_row(ui, palette, app.t("environment"), &host.environment);
                } else {
                    property_row(ui, palette, app.t("host"), &host.ip);
                    property_row(
                        ui,
                        palette,
                        app.t("port"),
                        &format!("{} (SSH-2)", host.port),
                    );
                    property_row(
                        ui,
                        palette,
                        app.t("user"),
                        if host.user.is_empty() {
                            "—"
                        } else {
                            host.user.as_str()
                        },
                    );
                    let credential = match host.auth {
                        crate::domain::AuthKind::Password => "••••••••".to_owned(),
                        crate::domain::AuthKind::Agent => app.t("ssh_agent").to_owned(),
                        crate::domain::AuthKind::Key => match host.key_source {
                            crate::domain::KeySource::Managed => host
                                .key_id
                                .and_then(|id| app.saved_keys.iter().find(|key| key.id == id))
                                .map(|key| key.name.clone())
                                .unwrap_or_else(|| app.t("missing_managed_key").to_owned()),
                            crate::domain::KeySource::FileSystem => {
                                if host.secret.is_empty() {
                                    "—".to_owned()
                                } else {
                                    host.secret.clone()
                                }
                            }
                        },
                    };
                    property_row(ui, palette, app.t("auth_key"), &credential);
                    property_row(ui, palette, app.t("gateway"), &host.gateway);
                    property_row(
                        ui,
                        palette,
                        app.t("keep_alive"),
                        &format!("{}s (server)", host.keep_alive),
                    );
                    if host.credentials_unavailable {
                        ui.label(
                            RichText::new(app.t("credentials_unavailable"))
                                .size(13.0)
                                .color(palette.red),
                        );
                    }
                }
            }
        });
    if let Some(host_id) = edit_host_id {
        app.open_edit_host(host_id);
    }
}

fn property_row(ui: &mut Ui, palette: Palette, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(RichText::new(key).size(13.0).color(palette.muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(value)
                    .size(13.0)
                    .monospace()
                    .color(palette.text_soft),
            );
        });
    });
}
