use super::*;
use crate::ui::text_input::singleline_input;
use eframe::egui::Sense;

pub(super) fn dialog_window<R>(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    icon: Icon,
    width: f32,
    height: Option<f32>,
    resizable: bool,
    palette: Palette,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> (bool, Option<R>) {
    let mut close_requested = false;
    let mut result = None;
    let mut window = egui::Window::new(id)
        .id(egui::Id::unique(id))
        .title_bar(false)
        .collapsible(false)
        .resizable(resizable)
        .default_width(width)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            Frame::new()
                .fill(palette.panel_alt)
                .stroke(Stroke::new(1.0, palette.border_strong))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::ZERO),
        );
    if let Some(height) = height {
        window = window.default_height(height);
    }
    window.show(ctx, |ui| {
        if dialog_header(ui, palette, icon, title) {
            close_requested = true;
            return;
        }
        Frame::new()
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width((width - 28.0).max(220.0));
                result = Some(add_contents(ui));
            });
    });
    (close_requested, result)
}

fn dialog_header(ui: &mut Ui, palette: Palette, icon: Icon, title: &str) -> bool {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
    ui.painter().rect_filled(
        egui::Rect::from_min_size(rect.min, Vec2::new(rect.width(), 40.0)),
        CornerRadius::same(7),
        palette.panel,
    );
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1.0, palette.border),
    );
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 17.0, rect.center().y),
        Vec2::splat(20.0),
    );
    crate::icons::paint(ui, icon, icon_rect, palette.accent);
    ui.painter()
        .with_clip_rect(egui::Rect::from_min_max(
            egui::pos2(rect.left() + 30.0, rect.top()),
            egui::pos2(rect.right() - 34.0, rect.bottom()),
        ))
        .text(
            egui::pos2(rect.left() + 31.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            title,
            FontId::proportional(13.0),
            palette.text,
        );
    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 16.0, rect.center().y),
        Vec2::splat(22.0),
    );
    let close = ui.interact(
        close_rect,
        egui::Id::unique(("dialog_close", title)),
        Sense::click(),
    );
    if close.hovered() {
        ui.painter()
            .rect_filled(close_rect, CornerRadius::same(3), palette.hover);
    }
    crate::icons::paint(
        ui,
        Icon::X,
        close_rect.shrink(4.0),
        if close.hovered() {
            palette.red
        } else {
            palette.muted
        },
    );
    close.clicked()
}

pub(super) fn field(
    ui: &mut Ui,
    palette: Palette,
    lang: Language,
    label: &str,
    value: &mut String,
) {
    ui.label(RichText::new(label).size(13.0).color(palette.muted));
    singleline_input(ui, palette, lang, value, ui.available_width(), false, None);
}

pub(super) fn path_input_with_browse(
    ui: &mut Ui,
    palette: Palette,
    lang: Language,
    value: &mut String,
    title: &str,
    hint: &str,
) -> bool {
    let mut browse = false;
    ui.horizontal(|ui| {
        let button_width = 30.0;
        let input_width =
            (ui.available_width() - button_width - ui.spacing().item_spacing.x).max(80.0);
        singleline_input(ui, palette, lang, value, input_width, false, Some(hint));
        if ui
            .add_sized(
                [button_width, 26.0],
                egui::Button::new(RichText::new("…").size(13.0).color(palette.text_soft))
                    .fill(palette.raised)
                    .stroke(Stroke::new(1.0, palette.border)),
            )
            .on_hover_text("Browse")
            .clicked()
        {
            browse = true;
        }
    });
    if browse {
        if let Some(path) = rfd::FileDialog::new().set_title(title).pick_file() {
            *value = path.display().to_string();
        }
    }
    browse
}

pub(super) fn remember_combo(
    ui: &mut Ui,
    _palette: Palette,
    lang: Language,
    value: &mut RememberDuration,
) {
    ComboBox::from_id_salt("remember_duration")
        .icon(combo_icon)
        .selected_text(remember_label(lang, *value))
        .width(160.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, RememberDuration::None, tr(lang, "remember_none"));
            ui.selectable_value(value, RememberDuration::Week, tr(lang, "remember_week"));
            ui.selectable_value(value, RememberDuration::Month, tr(lang, "remember_month"));
            ui.selectable_value(
                value,
                RememberDuration::Forever,
                tr(lang, "remember_forever"),
            );
        });
}

pub(super) fn remember_label(lang: Language, value: RememberDuration) -> &'static str {
    match value {
        RememberDuration::None => tr(lang, "remember_none"),
        RememberDuration::Week => tr(lang, "remember_week"),
        RememberDuration::Month => tr(lang, "remember_month"),
        RememberDuration::Forever => tr(lang, "remember_forever"),
    }
}
