use eframe::egui::{
    self, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};

use crate::app::{ConsoleTab, LogSelection, ShellCoreApp};
use crate::domain::{format_bytes, LogEntry, LogLevel, TransferStatus, TunnelState};
use crate::icons::Icon;
use crate::theme::Palette;
use crate::ui::clipboard::copy_text;
use crate::ui::components::{
    context_menu_item, context_menu_separator, icon_button, prepare_context_menu, progress_bar,
};

pub fn draw(app: &mut ShellCoreApp, ui: &mut Ui) {
    let palette = Palette::for_mode(app.settings.theme);
    let height = if app.settings.console_collapsed {
        30.0
    } else {
        200.0
    };
    egui::Panel::bottom("output_console")
        .exact_size(height)
        .resizable(false)
        .frame(Frame::new().fill(palette.canvas).inner_margin(Margin::ZERO))
        .show(ui, |ui| {
            header(app, ui, palette);
            if !app.settings.console_collapsed {
                let body_height = ui.available_height();
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), body_height),
                    Layout::top_down(Align::Min),
                    |ui| match app.console_tab {
                        ConsoleTab::Logs => logs(app, ui, palette),
                        ConsoleTab::Transfers => transfers(app, ui, palette),
                        ConsoleTab::Tunnels => tunnels(app, ui, palette),
                    },
                );
            }
        });
}

fn header(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1.0, palette.border),
    );
    let active_transfers = app
        .transfers
        .iter()
        .filter(|transfer| transfer.status == TransferStatus::Active)
        .count();
    let log_width = 225.0_f32.min(rect.width() * 0.3);
    let transfer_width = 196.0_f32.min(rect.width() * 0.27);
    let tunnel_width = 178.0_f32.min(rect.width() * 0.24);
    let mut x = rect.left() + 4.0;
    let online_tunnels = app
        .tunnels
        .iter()
        .filter(|tunnel| tunnel.state == TunnelState::Online)
        .count();
    let tabs = [
        (
            ConsoleTab::Logs,
            format!("{} ({})", app.t("connection_log"), app.logs.len()),
            log_width,
            Icon::Terminal,
        ),
        (
            ConsoleTab::Transfers,
            format!(
                "{} ({} {})",
                app.t("transfer_queue"),
                active_transfers,
                app.t("queue_active")
            ),
            transfer_width,
            Icon::CloudUpload,
        ),
        (
            ConsoleTab::Tunnels,
            format!("{} ({online_tunnels})", app.t("port_tunnels")),
            tunnel_width,
            Icon::Network,
        ),
    ];
    for (index, (tab, label, width, icon)) in tabs.into_iter().enumerate() {
        let tab_rect = egui::Rect::from_min_size(
            egui::pos2(x, rect.top() + 1.0),
            Vec2::new(width, rect.height() - 2.0),
        );
        let response = ui.interact(
            tab_rect,
            egui::Id::unique(("console_tab", index)),
            Sense::click(),
        );
        let active = app.console_tab == tab;
        if active {
            ui.painter()
                .rect_filled(tab_rect, CornerRadius::same(3), palette.panel_alt);
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(tab_rect.left() + 3.0, tab_rect.bottom() - 2.0),
                    egui::pos2(tab_rect.right() - 3.0, tab_rect.bottom()),
                ),
                CornerRadius::same(1),
                palette.accent,
            );
        } else if response.hovered() {
            ui.painter()
                .rect_filled(tab_rect, CornerRadius::same(3), palette.hover);
        }
        crate::icons::paint(
            ui,
            icon,
            egui::Rect::from_center_size(
                egui::pos2(tab_rect.left() + 12.0, tab_rect.center().y),
                Vec2::splat(13.0),
            ),
            if active {
                palette.accent
            } else {
                palette.muted
            },
        );
        ui.painter()
            .with_clip_rect(tab_rect.shrink2(Vec2::new(20.0, 0.0)))
            .text(
                egui::pos2(tab_rect.left() + 21.0, tab_rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(13.0),
                if active { palette.text } else { palette.muted },
            );
        if response.clicked() {
            app.console_tab = tab;
        }
        x += width;
    }

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_max(
                egui::pos2(rect.right() - 168.0, rect.top()),
                egui::pos2(rect.right() - 8.0, rect.bottom()),
            ))
            .layout(Layout::right_to_left(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            if icon_button(
                ui,
                palette,
                if app.settings.console_collapsed {
                    Icon::ChevronUp
                } else {
                    Icon::ChevronDown
                },
                13.0,
                if app.settings.console_collapsed {
                    app.t("expand_console")
                } else {
                    app.t("collapse_console")
                },
                false,
            )
            .clicked()
            {
                app.settings.console_collapsed = !app.settings.console_collapsed;
                app.persist_settings();
            }
            if icon_button(ui, palette, Icon::Trash, 13.0, app.t("clear"), false).clicked() {
                match app.console_tab {
                    ConsoleTab::Logs => {
                        app.logs.clear();
                        app.log_selection = None;
                    }
                    ConsoleTab::Transfers => app.transfers.clear(),
                    ConsoleTab::Tunnels => app.tunnels.clear(),
                }
            }
            let mut auto_scroll = app.settings.auto_scroll;
            if ui
                .checkbox(
                    &mut auto_scroll,
                    RichText::new(app.t("auto_scroll")).size(13.0),
                )
                .changed()
            {
                app.settings.auto_scroll = auto_scroll;
                app.persist_settings();
            }
        },
    );
}

fn logs(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    Frame::new()
        .fill(palette.canvas)
        .inner_margin(Margin::symmetric(7, 3))
        .show(ui, |ui| {
            let font = egui::FontId::monospace(13.0);
            let (char_width, row_height) = ui
                .ctx()
                .fonts_mut(|fonts| (fonts.glyph_width(&font, ' '), fonts.row_height(&font)));
            let row_height = row_height.max(10.0);
            ScrollArea::vertical()
                .id_salt("console_logs")
                .stick_to_bottom(app.settings.auto_scroll)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.set_width(ui.available_width());
                    if app.logs.is_empty() {
                        app.log_selection = None;
                        return;
                    }
                    if app.log_selection.is_some_and(|selection| {
                        selection.anchor_row >= app.logs.len()
                            || selection.cursor_row >= app.logs.len()
                    }) {
                        app.log_selection = None;
                    }

                    let row_count = app.logs.len();
                    let (content_rect, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), row_height * row_count as f32),
                        Sense::click_and_drag(),
                    );
                    let row_at = |position: egui::Pos2| {
                        ((position.y - content_rect.top()) / row_height)
                            .floor()
                            .clamp(0.0, row_count.saturating_sub(1) as f32)
                            as usize
                    };
                    let hovered_row = if response.hovered() {
                        ui.ctx().pointer_hover_pos().map(row_at)
                    } else {
                        None
                    };
                    let mut select_all = false;
                    let mut copy_requested = false;

                    if response.drag_started() {
                        let pointer = ui
                            .input(|input| input.pointer.press_origin())
                            .or_else(|| response.interact_pointer_pos());
                        if let Some(pointer) = pointer {
                            let anchor_row = row_at(pointer);
                            let cursor_row = response
                                .interact_pointer_pos()
                                .map(row_at)
                                .unwrap_or(anchor_row);
                            app.log_selection = Some(LogSelection {
                                anchor_row,
                                cursor_row,
                            });
                            response.request_focus();
                        }
                    } else if response.dragged() {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            let row = row_at(pointer);
                            if let Some(selection) = &mut app.log_selection {
                                selection.cursor_row = row;
                            }
                        }
                    } else if response.clicked() {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            let row = row_at(pointer);
                            app.log_selection = Some(LogSelection {
                                anchor_row: row,
                                cursor_row: row,
                            });
                            response.request_focus();
                        }
                    }

                    let copy_label = app.t("copy");
                    let select_all_label = app.t("select_all");
                    let has_selection = app.log_selection.is_some();
                    response.context_menu(|ui| {
                        prepare_context_menu(ui, palette);
                        if context_menu_item(
                            ui,
                            palette,
                            Icon::Copy,
                            copy_label,
                            "Ctrl+C",
                            has_selection,
                            false,
                        ) {
                            copy_requested = true;
                            ui.close();
                        }
                        context_menu_separator(ui, palette);
                        if context_menu_item(
                            ui,
                            palette,
                            Icon::SelectAll,
                            select_all_label,
                            "Ctrl+A",
                            true,
                            false,
                        ) {
                            select_all = true;
                            ui.close();
                        }
                    });

                    if response.has_focus() {
                        if ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::COMMAND, egui::Key::A)
                        }) {
                            select_all = true;
                        }
                        if ui.input(|input| {
                            input.events.iter().any(|event| {
                                matches!(event, egui::Event::Copy)
                                    || matches!(
                                        event,
                                        egui::Event::Key {
                                            key: egui::Key::C,
                                            pressed: true,
                                            modifiers,
                                            ..
                                        } if modifiers.ctrl || modifiers.command
                                    )
                            })
                        }) {
                            copy_requested = true;
                        }
                    }

                    if select_all {
                        app.log_selection = Some(LogSelection {
                            anchor_row: 0,
                            cursor_row: row_count - 1,
                        });
                        response.request_focus();
                    }
                    if copy_requested {
                        if let Some(selection) = app.log_selection {
                            copy_text(ui.ctx(), selected_log_text(&app.logs, selection));
                        }
                    }
                    if response.secondary_clicked() {
                        response.request_focus();
                    }

                    let selected_rows = app.log_selection.map(LogSelection::ordered);
                    for (index, entry) in app.logs.iter().enumerate() {
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(
                                content_rect.left(),
                                content_rect.top() + index as f32 * row_height,
                            ),
                            Vec2::new(content_rect.width(), row_height),
                        );
                        let row_selected = selected_rows
                            .is_some_and(|(start, end)| (start..=end).contains(&index));
                        if row_selected {
                            ui.painter()
                                .rect_filled(rect, CornerRadius::ZERO, palette.selection);
                        } else if hovered_row == Some(index) {
                            ui.painter()
                                .rect_filled(rect, CornerRadius::ZERO, palette.hover);
                        }

                        let level = log_level_label(entry.level);
                        let painter = ui.painter().with_clip_rect(rect);
                        painter.text(
                            egui::pos2(rect.left(), rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            format!("[{}]", entry.time),
                            font.clone(),
                            palette.muted,
                        );
                        let level_x = rect.left() + char_width * 14.0;
                        paint_without_ligatures(
                            &painter,
                            egui::pos2(level_x, rect.center().y),
                            &format!("[{level}]"),
                            &font,
                            char_width,
                            palette.log_color(entry.level),
                        );
                        painter.text(
                            egui::pos2(level_x + char_width * 6.0 + 4.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &entry.message,
                            font.clone(),
                            if entry.level == LogLevel::Error {
                                palette.red
                            } else {
                                palette.text_soft
                            },
                        );
                    }

                    if let Some(row) = hovered_row {
                        response.on_hover_text(&app.logs[row].message);
                    }
                });
        });
}

fn log_level_label(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Info => "INFO",
        LogLevel::Success => " OK ",
        LogLevel::Warning => "WARN",
        LogLevel::Error => "ERR ",
        LogLevel::Transfer => "XFR ",
    }
}

fn selected_log_text(logs: &[LogEntry], selection: LogSelection) -> String {
    let (start, end) = selection.ordered();
    let end = end.min(logs.len().saturating_sub(1));
    logs.get(start..=end)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    format!(
                        "[{}] [{}] {}",
                        entry.time,
                        log_level_label(entry.level),
                        entry.message
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn paint_without_ligatures(
    painter: &egui::Painter,
    start: egui::Pos2,
    text: &str,
    font: &egui::FontId,
    char_width: f32,
    color: egui::Color32,
) {
    for (index, character) in text.chars().enumerate() {
        painter.text(
            egui::pos2(start.x + index as f32 * char_width, start.y),
            egui::Align2::LEFT_CENTER,
            character,
            font.clone(),
            color,
        );
    }
}

fn transfers(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    Frame::new()
        .fill(palette.canvas)
        .inner_margin(Margin::symmetric(7, 5))
        .show(ui, |ui| {
            if app.transfers.is_empty() {
                empty_console(ui, palette, Icon::CloudUpload, app.t("no_transfer_tasks"));
                return;
            }
            ScrollArea::vertical()
                .id_salt("console_transfers")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for transfer in &app.transfers {
                        Frame::new()
                            .fill(palette.panel)
                            .stroke(Stroke::new(1.0, palette.border))
                            .corner_radius(CornerRadius::same(3))
                            .inner_margin(Margin::symmetric(7, 4))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let icon = match transfer.direction {
                                        crate::domain::TransferDirection::Upload => {
                                            Icon::CloudUpload
                                        }
                                        crate::domain::TransferDirection::Download => {
                                            Icon::CloudDownload
                                        }
                                        crate::domain::TransferDirection::RemoteToRemote => {
                                            Icon::ArrowsExchange
                                        }
                                    };
                                    crate::icons::image(ui, icon, 13.0, palette.cyan);
                                    ui.label(
                                        RichText::new(&transfer.name)
                                            .size(13.0)
                                            .color(palette.text),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(
                                                if transfer.status == TransferStatus::Done {
                                                    format!(
                                                        "DONE · {}",
                                                        format_bytes(transfer.total_bytes as u64)
                                                    )
                                                } else {
                                                    format!(
                                                        "{:.0}% · {:.1} MB/s · ETA {}",
                                                        transfer.progress() * 100.0,
                                                        transfer.speed_bps / (1024.0 * 1024.0),
                                                        transfer.eta()
                                                    )
                                                },
                                            )
                                            .size(13.0)
                                            .monospace()
                                            .color(
                                                if transfer.status == TransferStatus::Done {
                                                    palette.green
                                                } else {
                                                    palette.text_soft
                                                },
                                            ),
                                        );
                                        progress_bar(
                                            ui,
                                            transfer.progress(),
                                            170.0,
                                            5.0,
                                            if transfer.status == TransferStatus::Done {
                                                palette.green
                                            } else {
                                                palette.cyan
                                            },
                                            palette.raised,
                                        );
                                    });
                                });
                            });
                        ui.add_space(3.0);
                    }
                });
        });
}

fn tunnels(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    Frame::new()
        .fill(palette.canvas)
        .inner_margin(Margin::symmetric(7, 5))
        .show(ui, |ui| {
            if app.tunnels.is_empty() {
                empty_console(ui, palette, Icon::Network, app.t("no_active_tunnels"));
                return;
            }
            ScrollArea::vertical()
                .id_salt("console_tunnels")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for tunnel in &app.tunnels {
                        ui.horizontal(|ui| {
                            let color = match tunnel.state {
                                TunnelState::Online => palette.green,
                                TunnelState::Reconnecting => palette.amber,
                                TunnelState::Stopped => palette.red,
                            };
                            ui.painter().circle_filled(
                                egui::pos2(ui.cursor().left() + 4.0, ui.cursor().top() + 9.0),
                                3.0,
                                color,
                            );
                            ui.add_space(12.0);
                            ui.label(RichText::new(&tunnel.name).size(13.0).color(palette.text));
                            ui.label(
                                RichText::new(format!("{} → {}", tunnel.local, tunnel.remote))
                                    .size(13.0)
                                    .monospace()
                                    .color(palette.cyan),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(&tunnel.traffic)
                                        .size(13.0)
                                        .monospace()
                                        .color(palette.muted),
                                );
                            });
                        });
                    }
                });
        });
}

fn empty_console(ui: &mut Ui, palette: Palette, icon: Icon, text: &str) {
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.add_space(18.0);
        crate::icons::image(ui, icon, 24.0, palette.muted);
        ui.label(RichText::new(text).size(13.0).color(palette.muted));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_logs_copy_in_display_order() {
        let logs = vec![
            LogEntry {
                time: "10:00:00.000".to_owned(),
                level: LogLevel::Info,
                message: "first".to_owned(),
            },
            LogEntry {
                time: "10:00:01.000".to_owned(),
                level: LogLevel::Success,
                message: "second".to_owned(),
            },
            LogEntry {
                time: "10:00:02.000".to_owned(),
                level: LogLevel::Warning,
                message: "third".to_owned(),
            },
        ];
        let selection = LogSelection {
            anchor_row: 2,
            cursor_row: 1,
        };

        assert_eq!(
            selected_log_text(&logs, selection),
            "[10:00:01.000] [ OK ] second\n[10:00:02.000] [WARN] third"
        );
    }

    #[test]
    fn selected_logs_ignore_out_of_range_rows() {
        let logs = vec![LogEntry {
            time: "10:00:00.000".to_owned(),
            level: LogLevel::Info,
            message: "only".to_owned(),
        }];
        let selection = LogSelection {
            anchor_row: 4,
            cursor_row: 0,
        };

        assert_eq!(
            selected_log_text(&logs, selection),
            "[10:00:00.000] [INFO] only"
        );
    }
}
