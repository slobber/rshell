use super::*;
pub(super) fn empty_pane(app: &mut ShellCoreApp, ui: &mut Ui, pane: &mut Pane, palette: Palette) {
    let has_hosts = app.has_hosts();
    let content_height = if has_hosts { 260.0 } else { 308.0 };
    let top = ((ui.available_height() - content_height) * 0.5).max(20.0);
    let watermark_text = Color32::from_rgba_unmultiplied(
        palette.muted.r(),
        palette.muted.g(),
        palette.muted.b(),
        62,
    );
    ui.add_space(top);
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        crate::icons::app_logo_muted(ui, 192.0);
        ui.add_space(12.0);
        ui.label(
            RichText::new("RShell")
                .size(44.0)
                .strong()
                .color(watermark_text),
        );
        if !has_hosts {
            ui.add_space(16.0);
            if ui
                .add_sized(
                    [128.0, 30.0],
                    egui::Button::new(
                        RichText::new(format!("+ {}", app.t("new_host")))
                            .size(13.0)
                            .strong()
                            .color(palette.on_accent),
                    )
                    .fill(palette.accent)
                    .stroke(Stroke::NONE),
                )
                .clicked()
            {
                app.open_new_session();
            }
        }
        pane.active = 0;
    });
}

pub(super) fn drop_zone(rect: Rect, pointer: egui::Pos2) -> DropZone {
    let rel_x = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    let rel_y = ((pointer.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    if rel_x < 0.25 {
        DropZone::Left
    } else if rel_x > 0.75 {
        DropZone::Right
    } else if rel_y < 0.25 {
        DropZone::Top
    } else if rel_y > 0.75 {
        DropZone::Bottom
    } else {
        DropZone::Center
    }
}

pub(super) fn paint_drop_preview(
    ui: &mut Ui,
    pane_rect: Rect,
    zone: DropZone,
    title: &str,
    palette: Palette,
    language: crate::i18n::Language,
) {
    let preview = match zone {
        DropZone::Center => pane_rect.shrink(5.0),
        DropZone::Left => Rect::from_min_max(
            pane_rect.min,
            egui::pos2(pane_rect.center().x, pane_rect.max.y),
        ),
        DropZone::Right => Rect::from_min_max(
            egui::pos2(pane_rect.center().x, pane_rect.min.y),
            pane_rect.max,
        ),
        DropZone::Top => Rect::from_min_max(
            pane_rect.min,
            egui::pos2(pane_rect.max.x, pane_rect.center().y),
        ),
        DropZone::Bottom => Rect::from_min_max(
            egui::pos2(pane_rect.min.x, pane_rect.center().y),
            pane_rect.max,
        ),
    };
    ui.painter().rect_filled(
        preview,
        CornerRadius::same(3),
        Color32::from_rgba_unmultiplied(
            palette.accent.r(),
            palette.accent.g(),
            palette.accent.b(),
            38,
        ),
    );
    ui.painter().rect_stroke(
        preview,
        CornerRadius::same(3),
        Stroke::new(1.5, palette.accent),
        egui::StrokeKind::Inside,
    );
    let hint = match zone {
        DropZone::Center => format!("{}: {title}", crate::i18n::tr(language, "move_here")),
        DropZone::Left => format!("{}: {title}", crate::i18n::tr(language, "split_left")),
        DropZone::Right => format!("{}: {title}", crate::i18n::tr(language, "split_right")),
        DropZone::Top => format!("{}: {title}", crate::i18n::tr(language, "split_top")),
        DropZone::Bottom => format!("{}: {title}", crate::i18n::tr(language, "split_bottom")),
    };
    ui.painter().text(
        preview.center(),
        egui::Align2::CENTER_CENTER,
        hint,
        FontId::proportional(13.0),
        palette.accent,
    );
}
