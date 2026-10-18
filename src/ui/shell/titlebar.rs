use super::*;

pub fn title_bar(app: &mut ShellCoreApp, ui: &mut Ui) {
    let palette = Palette::for_mode(app.settings.theme);
    egui::Panel::top("window_title_bar")
        .exact_size(36.0)
        .frame(
            Frame::new()
                .fill(palette.canvas)
                .inner_margin(Margin::symmetric(9, 0)),
        )
        .show(ui, |ui| {
            let drag_rect = Rect::from_min_size(
                ui.max_rect().min,
                Vec2::new(
                    (ui.max_rect().width() - 220.0).max(280.0),
                    ui.max_rect().height(),
                ),
            );
            let drag = ui.interact(
                drag_rect,
                egui::Id::unique("window_title_drag_region"),
                Sense::click_and_drag(),
            );
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let maximized = ui
                    .ctx()
                    .input(|input| input.viewport().maximized.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
            }

            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                crate::icons::app_logo(ui, 20.0);
                ui.add_space(8.0);
                super::menus(app, ui, palette);
                if app.lock_state != AppLock::Unlocked {
                    badge(ui, palette, app.t("locked"), palette.raised, palette.amber);
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let ctx = ui.ctx().clone();
                    window_button(ui, palette, Icon::X, app.t("close_window"), true, || {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    });
                    window_button(
                        ui,
                        palette,
                        Icon::Square,
                        app.t("maximize_window"),
                        false,
                        || {
                            let maximized =
                                ctx.input(|input| input.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                        },
                    );
                    window_button(
                        ui,
                        palette,
                        Icon::Minus,
                        app.t("minimize_window"),
                        false,
                        || {
                            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                        },
                    );
                    ui.add_space(4.0);
                    if icon_button(
                        ui,
                        palette,
                        if app.settings.theme.is_dark() {
                            Icon::Sun
                        } else {
                            Icon::Moon
                        },
                        14.0,
                        app.t("toggle_theme"),
                        false,
                    )
                    .clicked()
                    {
                        app.toggle_theme(&ctx);
                    }
                    if icon_button(ui, palette, Icon::Settings, 14.0, app.t("settings"), false)
                        .clicked()
                    {
                        app.settings_open = true;
                    }
                });
            });
        });
}

fn badge(ui: &mut Ui, _palette: Palette, text: &str, fill: Color32, color: Color32) {
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(3))
        .inner_margin(Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(13.0).monospace().color(color));
        });
}

fn window_button(
    ui: &mut Ui,
    palette: Palette,
    icon: Icon,
    tooltip: &str,
    danger: bool,
    mut on_click: impl FnMut(),
) {
    let response = ui.allocate_response(Vec2::splat(28.0), Sense::click());
    let fill = if response.hovered() {
        if danger {
            Color32::from_rgba_unmultiplied(160, 35, 55, 160)
        } else {
            palette.hover
        }
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(response.rect, CornerRadius::same(3), fill);
    let icon_rect = Rect::from_center_size(response.rect.center(), Vec2::splat(16.0));
    crate::icons::paint(
        ui,
        icon,
        icon_rect,
        if response.hovered() {
            palette.text
        } else {
            palette.text_soft
        },
    );
    if response.clicked() {
        on_click();
    }
    response.on_hover_text(tooltip);
}
