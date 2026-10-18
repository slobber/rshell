use eframe::egui::{
    self, Color32, CornerRadius, FontId, Frame, Margin, Response, RichText, Sense, Stroke, Ui, Vec2,
};

use crate::domain::{ConnectionBackend, Host, HostOs};
use crate::icons::{self, Icon};
use crate::theme::Palette;

pub fn host_icon(host: &Host) -> Icon {
    match host.backend {
        ConnectionBackend::LocalWsl => Icon::BrandLinux,
        ConnectionBackend::Rdp => Icon::BrandWindows,
        ConnectionBackend::Ssh => match host.os {
            HostOs::Ubuntu => Icon::BrandUbuntu,
            HostOs::Debian => Icon::BrandDebian,
            HostOs::Deepin => Icon::BrandDeepin,
            HostOs::UnionTech => Icon::BrandUnionTech,
            HostOs::Kylin => Icon::BrandKylin,
            HostOs::CentOs => Icon::BrandCentOs,
            HostOs::RedHat => Icon::BrandRedHat,
            HostOs::Fedora => Icon::BrandFedora,
            HostOs::Arch => Icon::BrandArchLinux,
            HostOs::Alpine => Icon::BrandAlpine,
            HostOs::OpenSuse => Icon::BrandOpenSuse,
            HostOs::Rocky => Icon::BrandRocky,
            HostOs::AlmaLinux => Icon::BrandAlma,
            HostOs::MacOs => Icon::BrandApple,
            HostOs::Windows => Icon::BrandWindows,
            HostOs::Linux | HostOs::Unknown => Icon::BrandLinux,
        },
    }
}

pub fn icon_button(
    ui: &mut Ui,
    palette: Palette,
    icon: Icon,
    size: f32,
    tooltip: &str,
    active: bool,
) -> Response {
    let rendered_size = size.max(18.0);
    let response = ui.allocate_response(Vec2::splat(rendered_size + 8.0), Sense::click());
    let fill = if active {
        palette.accent_soft
    } else if response.hovered() {
        palette.hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(response.rect, CornerRadius::same(4), fill);
    }
    let icon_rect =
        egui::Rect::from_center_size(response.rect.center(), Vec2::splat(rendered_size));
    let color = if active {
        palette.accent
    } else if response.hovered() {
        palette.text
    } else {
        palette.text_soft
    };
    icons::paint(ui, icon, icon_rect, color);
    if tooltip.is_empty() {
        response
    } else {
        response.on_hover_text(tooltip)
    }
}

pub fn combo_icon(ui: &Ui, rect: egui::Rect, visuals: &egui::style::WidgetVisuals, _open: bool) {
    icons::paint(
        ui,
        Icon::ChevronDown,
        rect.shrink(2.0),
        visuals.fg_stroke.color,
    );
}

pub fn labeled_icon_button(
    ui: &mut Ui,
    palette: Palette,
    icon: Icon,
    label: &str,
    active: bool,
    accent: Color32,
) -> Response {
    let font = FontId::proportional(13.0);
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        font,
        if active { accent } else { palette.text_soft },
    );
    let width = 24.0 + 7.0 + galley.size().x + 14.0;
    let response = ui.allocate_response(Vec2::new(width, 30.0), Sense::click());
    let fill = if active {
        palette.accent_soft
    } else if response.hovered() {
        palette.hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(response.rect, CornerRadius::same(4), fill);
    }
    let color = if active { accent } else { palette.text_soft };
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(response.rect.left() + 14.0, response.rect.center().y),
        Vec2::splat(20.0),
    );
    icons::paint(ui, icon, icon_rect, color);
    ui.painter().galley(
        egui::pos2(
            response.rect.left() + 28.0,
            response.rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
    response
}

pub fn primary_button(ui: &mut Ui, palette: Palette, text: &str) -> Response {
    ui.add(
        egui::Button::new(
            RichText::new(text)
                .size(13.0)
                .strong()
                .color(palette.on_accent),
        )
        .fill(palette.accent)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(4))
        .min_size(Vec2::new(104.0, 30.0)),
    )
}

pub fn secondary_button(ui: &mut Ui, palette: Palette, text: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).color(palette.text_soft))
            .fill(palette.raised)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(CornerRadius::same(4))
            .min_size(Vec2::new(88.0, 30.0)),
    )
}

pub fn status_dot(ui: &mut Ui, color: Color32, radius: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(radius * 2.0 + 4.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), radius, color);
}

pub fn progress_bar(
    ui: &mut Ui,
    value: f32,
    width: f32,
    height: f32,
    fill: Color32,
    track: Color32,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same((height / 2.0) as u8), track);
    let filled = egui::Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width() * value.clamp(0.0, 1.0), rect.height()),
    );
    ui.painter()
        .rect_filled(filled, CornerRadius::same((height / 2.0) as u8), fill);
}

pub fn section_label(ui: &mut Ui, palette: Palette, text: &str) {
    ui.label(
        RichText::new(text.to_ascii_uppercase())
            .size(13.0)
            .monospace()
            .color(palette.muted),
    );
}

pub fn modal_frame(palette: Palette) -> Frame {
    Frame::new()
        .fill(palette.panel_alt)
        .stroke(Stroke::new(1.0, palette.border_strong))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
}

/// Shared geometry and palette for every right-click menu in the app.
pub fn prepare_context_menu(ui: &mut Ui, palette: Palette) {
    ui.set_min_width(264.0);
    ui.set_max_width(264.0);
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    ui.spacing_mut().button_padding = Vec2::new(8.0, 5.0);
    ui.visuals_mut().window_fill = palette.panel_alt;
    ui.visuals_mut().window_stroke = Stroke::new(1.0, palette.border_strong);
    ui.visuals_mut().menu_corner_radius = CornerRadius::same(6);
}

pub fn context_menu_item(
    ui: &mut Ui,
    palette: Palette,
    icon: Icon,
    label: &str,
    shortcut: &str,
    enabled: bool,
    danger: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 30.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hovered = enabled && response.hovered();
    if hovered {
        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(1.0, 0.0)),
            CornerRadius::same(4),
            palette.hover,
        );
    }

    let disabled = palette.muted.gamma_multiply(0.5);
    let text_color = if !enabled {
        disabled
    } else if danger {
        palette.red
    } else if hovered {
        palette.text
    } else {
        palette.text_soft
    };
    let icon_color = if !enabled {
        disabled
    } else if danger {
        palette.red
    } else if hovered {
        palette.accent
    } else {
        palette.muted
    };
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        Vec2::splat(17.0),
    );
    icons::paint(ui, icon, icon_rect, icon_color);
    ui.painter().text(
        egui::pos2(rect.left() + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        text_color,
    );
    if !shortcut.is_empty() {
        ui.painter().text(
            egui::pos2(rect.right() - 14.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            shortcut,
            FontId::monospace(12.0),
            if enabled { palette.muted } else { disabled },
        );
    }
    enabled && response.clicked()
}

pub fn context_menu_separator(ui: &mut Ui, palette: Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 9.0), Sense::hover());
    ui.painter().hline(
        rect.x_range().shrink(8.0),
        rect.center().y,
        Stroke::new(1.0, palette.border),
    );
}
