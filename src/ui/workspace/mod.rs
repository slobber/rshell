use std::mem;

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Frame, Layout, Margin, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, Vec2,
};

use crate::app::{DropTarget, ShellCoreApp, TabDrag};
use crate::domain::{
    find_pane_mut, Axis, DropZone, Pane, SessionTab, SplitNode, TabKind, Workspace,
};
use crate::icons::Icon;
use crate::theme::Palette;
use crate::ui::components::icon_button;

mod empty;
mod tabs;

pub fn draw(app: &mut ShellCoreApp, ui: &mut Ui) {
    let palette = Palette::for_mode(app.settings.theme);
    let mut workspace = mem::replace(&mut app.workspace, Workspace::new());
    let mut focused_pane = workspace.focused_pane;
    app.drop_target = None;

    egui::CentralPanel::default()
        .frame(Frame::new().fill(palette.canvas).inner_margin(Margin::ZERO))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let available = ui.available_size();
            if let Some(maximized) = app.maximized_pane {
                if let Some(pane) = find_pane_mut(&mut workspace.root, maximized) {
                    focused_pane = pane.id;
                    draw_leaf(app, ui, pane, &mut focused_pane, palette, available);
                } else {
                    app.maximized_pane = None;
                    draw_node(app, ui, &mut workspace.root, &mut focused_pane, palette);
                }
            } else {
                draw_node(app, ui, &mut workspace.root, &mut focused_pane, palette);
            }
        });

    workspace.focused_pane = focused_pane;
    workspace.prune_empty_leaves();
    app.workspace = workspace;
}

fn draw_node(
    app: &mut ShellCoreApp,
    ui: &mut Ui,
    node: &mut SplitNode,
    focused_pane: &mut u64,
    palette: Palette,
) {
    match node {
        SplitNode::Leaf(pane) => {
            let size = ui.available_size();
            draw_leaf(app, ui, pane, focused_pane, palette, size);
        }
        SplitNode::Split {
            axis,
            ratio,
            first,
            second,
        } => match axis {
            Axis::Horizontal => {
                let total = ui.available_size();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    ui.set_min_size(total);
                    let sash_width = 6.0;
                    let first_width = ((total.x - sash_width) * *ratio).max(80.0);
                    let second_width = (total.x - sash_width - first_width).max(80.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(first_width, total.y),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_min_size(Vec2::new(first_width, total.y));
                            draw_node(app, ui, first, focused_pane, palette);
                        },
                    );
                    let response = split_sash(ui, Axis::Horizontal, total.y, palette);
                    if response.dragged() {
                        let delta = ui.ctx().input(|input| input.pointer.delta().x);
                        *ratio =
                            (*ratio + delta / (total.x - sash_width).max(1.0)).clamp(0.16, 0.84);
                    }
                    ui.allocate_ui_with_layout(
                        Vec2::new(second_width, total.y),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_min_size(Vec2::new(second_width, total.y));
                            draw_node(app, ui, second, focused_pane, palette);
                        },
                    );
                });
            }
            Axis::Vertical => {
                let total = ui.available_size();
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    ui.set_min_size(total);
                    let sash_height = 6.0;
                    let first_height = ((total.y - sash_height) * *ratio).max(70.0);
                    let second_height = (total.y - sash_height - first_height).max(70.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(total.x, first_height),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_min_size(Vec2::new(total.x, first_height));
                            draw_node(app, ui, first, focused_pane, palette);
                        },
                    );
                    let response = split_sash(ui, Axis::Vertical, total.x, palette);
                    if response.dragged() {
                        let delta = ui.ctx().input(|input| input.pointer.delta().y);
                        *ratio =
                            (*ratio + delta / (total.y - sash_height).max(1.0)).clamp(0.16, 0.84);
                    }
                    ui.allocate_ui_with_layout(
                        Vec2::new(total.x, second_height),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_min_size(Vec2::new(total.x, second_height));
                            draw_node(app, ui, second, focused_pane, palette);
                        },
                    );
                });
            }
        },
    }
}

fn split_sash(ui: &mut Ui, axis: Axis, extent: f32, palette: Palette) -> egui::Response {
    let size = match axis {
        Axis::Horizontal => Vec2::new(6.0, extent),
        Axis::Vertical => Vec2::new(extent, 6.0),
    };
    let response = ui.allocate_response(size, Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(match axis {
            Axis::Horizontal => egui::CursorIcon::ResizeHorizontal,
            Axis::Vertical => egui::CursorIcon::ResizeVertical,
        });
    }
    let center = response.rect.center();
    let _ = match axis {
        Axis::Horizontal => ui.painter().vline(
            center.x,
            response.rect.y_range(),
            Stroke::new(
                if response.hovered() || response.dragged() {
                    2.0
                } else {
                    1.0
                },
                if response.hovered() || response.dragged() {
                    palette.accent
                } else {
                    palette.border
                },
            ),
        ),
        Axis::Vertical => ui.painter().hline(
            response.rect.x_range(),
            center.y,
            Stroke::new(
                if response.hovered() || response.dragged() {
                    2.0
                } else {
                    1.0
                },
                if response.hovered() || response.dragged() {
                    palette.accent
                } else {
                    palette.border
                },
            ),
        ),
    };
    response
}

fn draw_leaf(
    app: &mut ShellCoreApp,
    ui: &mut Ui,
    pane: &mut Pane,
    focused_pane: &mut u64,
    palette: Palette,
    size: Vec2,
) {
    let active = *focused_pane == pane.id;
    let frame = Frame::new()
        .fill(palette.terminal_bg)
        .stroke(Stroke::new(
            1.0,
            if active {
                palette.border_strong
            } else {
                palette.border
            },
        ))
        .inner_margin(Margin::ZERO);
    frame.show(ui, |ui| {
        ui.set_min_size(size);
        ui.set_max_size(size);
        tabs::tab_strip(app, ui, pane, focused_pane, palette);
        let content_height = (ui.available_height()).max(40.0);
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), content_height),
            Layout::top_down(Align::Min),
            |ui| {
                if let Some(tab) = pane.active_tab_mut() {
                    match tab.kind {
                        TabKind::Terminal => {
                            crate::ui::terminal::draw(app, ui, tab, palette);
                        }
                        TabKind::Sftp => {
                            crate::ui::sftp::draw(app, ui, tab, palette);
                        }
                        TabKind::Rdp => {
                            crate::ui::rdp::draw(app, ui, tab, palette);
                        }
                    }
                } else {
                    empty::empty_pane(app, ui, pane, palette);
                }
            },
        );
    });

    let pane_rect = ui.min_rect();
    if let Some(drag) = &app.drag {
        if let Some(pointer) = ui.ctx().pointer_hover_pos() {
            if pane_rect.contains(pointer) {
                let zone = empty::drop_zone(pane_rect, pointer);
                app.drop_target = Some(DropTarget {
                    pane_id: pane.id,
                    zone,
                });
                empty::paint_drop_preview(
                    ui,
                    pane_rect,
                    zone,
                    &drag.title,
                    palette,
                    app.settings.language,
                );
            }
        }
    }
}
