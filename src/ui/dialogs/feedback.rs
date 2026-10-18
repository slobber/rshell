use super::*;

pub fn drag_ghost(app: &mut ShellCoreApp, ctx: &egui::Context) {
    let palette = Palette::for_mode(app.settings.theme);
    let Some(drag) = &app.drag else {
        return;
    };
    let Some(pointer) = ctx.pointer_hover_pos() else {
        return;
    };
    egui::Area::new(egui::Id::unique("tab_drag_ghost"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pointer + Vec2::new(13.0, 11.0))
        .interactable(false)
        .show(ctx, |ui| {
            Frame::new()
                .fill(palette.panel_alt)
                .stroke(Stroke::new(1.0, palette.accent))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        crate::icons::image(ui, Icon::Terminal, 14.0, palette.accent);
                        ui.label(RichText::new(&drag.title).size(13.0).color(palette.text));
                    });
                });
        });
}

pub fn toast(app: &mut ShellCoreApp, ctx: &egui::Context) {
    let Some((message, created)) = &app.toast else {
        return;
    };
    if created.elapsed().as_secs_f32() > 3.0 {
        return;
    }
    let palette = Palette::for_mode(app.settings.theme);
    let opacity = (1.0 - (created.elapsed().as_secs_f32() - 2.3).max(0.0) / 0.7).clamp(0.0, 1.0);
    let alpha = (opacity * 255.0) as u8;
    egui::Area::new(egui::Id::unique("rshell_toast"))
        .order(egui::Order::Tooltip)
        .anchor(Align2::RIGHT_TOP, [-18.0, 42.0])
        .interactable(false)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(
                    palette.panel_alt.r(),
                    palette.panel_alt.g(),
                    palette.panel_alt.b(),
                    alpha,
                ))
                .stroke(Stroke::new(1.0, palette.accent))
                .corner_radius(CornerRadius::same(5))
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        crate::icons::image(ui, Icon::CircleCheck, 16.0, palette.green);
                        ui.label(RichText::new(message).size(13.0).color(palette.text));
                    });
                });
        });
}
