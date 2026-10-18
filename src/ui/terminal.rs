use eframe::egui::{
    self, Color32, CornerRadius, FontId, Frame, Margin, Rect, RichText, Stroke, Ui, Vec2,
};

use crate::app::ShellCoreApp;
use crate::domain::{find_host, ConnectionBackend, SessionTab, TerminalSelection};
use crate::icons::Icon;
use crate::theme::{Palette, TerminalPalette};
use crate::ui::clipboard::copy_text;
use crate::ui::components::{context_menu_item, context_menu_separator, prepare_context_menu};

pub fn draw(app: &mut ShellCoreApp, ui: &mut Ui, tab: &mut SessionTab, palette: Palette) {
    let Some(host) = find_host(&app.hosts, tab.host_id).cloned() else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new(app.t("no_host_exists")).color(palette.red));
        });
        return;
    };
    let connected = app.connected_hosts.contains(&host.id);
    let zoom = app.settings.terminal_zoom;
    let terminal_palette = TerminalPalette::for_theme(app.settings.terminal_theme);
    let available = ui.available_size();
    let status_height = 22.0;
    let terminal_height = (available.y - status_height).max(70.0);

    Frame::new()
        .fill(terminal_palette.background)
        .inner_margin(Margin::symmetric(5, 4))
        .show(ui, |ui| {
            let width = ui.available_width();
            let font = FontId::monospace(crate::theme::MIN_UI_FONT_SIZE * zoom);
            let line_height = (font.size * 1.35).max(18.0);
            let char_width = ui
                .ctx()
                .fonts_mut(|fonts| fonts.glyph_width(&font, ' '))
                .max(4.0);
            let cols = ((width - 8.0) / char_width).floor().clamp(20.0, 300.0) as u16;
            let rows = ((terminal_height - 8.0) / line_height)
                .floor()
                .clamp(8.0, 120.0) as u16;
            let (current_rows, current_cols) = tab.terminal.size();
            if current_rows != rows || current_cols != cols {
                tab.terminal.resize(rows, cols);
                app.ssh.resize(tab.id, u32::from(cols), u32::from(rows));
            }

            let (screen_rect, response) = ui.allocate_exact_size(
                Vec2::new(width, terminal_height - 8.0),
                egui::Sense::click_and_drag(),
            );
            let focus_requested = app.pending_focus_tab == Some(tab.id);
            if focus_requested
                || response.clicked()
                || response.secondary_clicked()
                || response.drag_started()
            {
                response.request_focus();
            }
            if focus_requested {
                app.pending_focus_tab = None;
            }
            let mut copy_selection = false;
            let mut paste_from_clipboard = false;
            if response.has_focus() {
                ui.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        response.id,
                        egui::EventFilter {
                            tab: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                        },
                    );
                });
                let input = ui.input(|input| {
                    collect_terminal_input(
                        &input.events,
                        tab.terminal_selection.is_some(),
                        input.modifiers.ctrl,
                    )
                });
                copy_selection = input.copy_selection;
                paste_from_clipboard = input.paste_from_clipboard;
                let bytes = input.bytes;
                if !bytes.is_empty() {
                    app.ssh.send_input(tab.id, bytes);
                }
            }

            if response.drag_started() {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let (row, col) =
                        pointer_cell(pointer, screen_rect, char_width, line_height, rows, cols);
                    tab.terminal_selection = Some(TerminalSelection {
                        anchor_row: row,
                        anchor_col: col,
                        cursor_row: row,
                        cursor_col: col,
                    });
                }
            } else if response.dragged() || response.drag_stopped() {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let (row, col) =
                        pointer_cell(pointer, screen_rect, char_width, line_height, rows, cols);
                    if let Some(selection) = &mut tab.terminal_selection {
                        selection.cursor_row = row;
                        selection.cursor_col = col;
                    }
                }
            } else if response.clicked() {
                tab.terminal_selection = None;
            }

            let copy_label = app.t("copy");
            let paste_label = app.t("paste");
            let select_all_label = app.t("select_all");
            let has_selection = tab.terminal_selection.is_some();
            let mut select_all = false;
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
                    copy_selection = true;
                    ui.close();
                }
                if context_menu_item(
                    ui,
                    palette,
                    Icon::Clipboard,
                    paste_label,
                    "Ctrl+V",
                    true,
                    false,
                ) {
                    paste_from_clipboard = true;
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

            if select_all {
                tab.terminal_selection = Some(TerminalSelection {
                    anchor_row: 0,
                    anchor_col: 0,
                    cursor_row: rows.saturating_sub(1),
                    cursor_col: cols.saturating_sub(1),
                });
            }
            if copy_selection {
                let text = tab.terminal.with_screen(|screen| {
                    tab.terminal_selection
                        .map(|selection| selection_text(screen, selection))
                        .unwrap_or_default()
                });
                if let Some(text) = text.filter(|text| !text.is_empty()) {
                    copy_text(ui.ctx(), text);
                }
            }
            if paste_from_clipboard {
                if let Some(text) = clipboard_text() {
                    app.ssh.send_input(tab.id, paste_bytes(&text));
                    tab.scroll_to_bottom = true;
                }
            }
            if response.secondary_clicked() || copy_selection || paste_from_clipboard || select_all
            {
                response.request_focus();
            }

            if response.hovered() {
                let delta = ui.ctx().input(|input| input.smooth_scroll_delta.y);
                if delta.abs() > 0.1 {
                    let lines = (delta / line_height).round() as i32;
                    tab.terminal.with_screen_mut(|screen| {
                        let current = screen.scrollback() as i32;
                        screen.set_scrollback((current + lines).max(0) as usize);
                    });
                }
            }
            if tab.scroll_to_bottom {
                tab.terminal
                    .with_screen_mut(|screen| screen.set_scrollback(0));
                tab.scroll_to_bottom = false;
            }

            ui.painter().rect_filled(
                screen_rect,
                CornerRadius::same(2),
                terminal_palette.background,
            );
            tab.terminal.with_screen(|screen| {
                paint_screen(
                    ui,
                    screen_rect,
                    screen,
                    &font,
                    char_width,
                    line_height,
                    terminal_palette,
                    tab.terminal_selection,
                );
            });

            if !connected {
                let notice = Rect::from_min_size(
                    egui::pos2(screen_rect.left() + 8.0, screen_rect.bottom() - 31.0),
                    Vec2::new((width - 16.0).max(120.0), 24.0),
                );
                ui.painter().rect_filled(
                    notice,
                    CornerRadius::same(3),
                    Color32::from_rgba_unmultiplied(120, 25, 45, 210),
                );
                ui.painter().text(
                    egui::pos2(notice.left() + 8.0, notice.center().y),
                    egui::Align2::LEFT_CENTER,
                    app.t("not_connected"),
                    FontId::proportional(crate::theme::MIN_UI_FONT_SIZE),
                    palette.red,
                );
            }

            let status_rect = Rect::from_min_size(
                egui::pos2(ui.cursor().left(), ui.cursor().top()),
                Vec2::new(ui.available_width(), status_height),
            );
            ui.painter()
                .rect_filled(status_rect, CornerRadius::ZERO, palette.panel);
            ui.painter().hline(
                status_rect.x_range(),
                status_rect.top(),
                Stroke::new(1.0, palette.border),
            );
            let transport_line = match host.backend {
                ConnectionBackend::Ssh => format!(
                    "SSH-2.0 (UTF-8)  ·  xterm-256color  ·  {cols}×{rows}  ·  keep-alive {}s",
                    host.keep_alive
                ),
                ConnectionBackend::LocalWsl => format!(
                    "WSL · ConPTY  ·  {}  ·  {cols}×{rows}",
                    host.wsl_distro.as_deref().unwrap_or("default")
                ),
                ConnectionBackend::Rdp => "RDP · framebuffer".to_owned(),
            };
            ui.painter().text(
                egui::pos2(status_rect.left() + 7.0, status_rect.center().y),
                egui::Align2::LEFT_CENTER,
                transport_line,
                FontId::monospace(crate::theme::MIN_UI_FONT_SIZE),
                palette.muted,
            );
            ui.painter().text(
                egui::pos2(status_rect.right() - 7.0, status_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                if connected {
                    if host.backend == ConnectionBackend::LocalWsl {
                        "WSL".to_owned()
                    } else {
                        app.t("live_ssh").to_owned()
                    }
                } else {
                    app.t("offline").to_owned()
                },
                FontId::monospace(crate::theme::MIN_UI_FONT_SIZE),
                if connected {
                    palette.green
                } else {
                    palette.red
                },
            );
            ui.allocate_space(Vec2::new(ui.available_width(), status_height));
        });
}

fn paint_screen(
    ui: &Ui,
    rect: Rect,
    screen: &vt100::Screen,
    font: &FontId,
    char_width: f32,
    line_height: f32,
    terminal_palette: TerminalPalette,
    selection: Option<TerminalSelection>,
) {
    let (rows, cols) = screen.size();
    if let Some(selection) = selection {
        let ((start_row, start_col), (end_row, end_col)) = selection.ordered();
        let start_row = start_row.min(rows.saturating_sub(1));
        let end_row = end_row.min(rows.saturating_sub(1));
        let start_col = start_col.min(cols.saturating_sub(1));
        let end_col = end_col.min(cols.saturating_sub(1));
        for row in start_row..=end_row {
            let from_col = if row == start_row { start_col } else { 0 };
            let to_col = if row == end_row { end_col } else { cols - 1 };
            if from_col > to_col {
                continue;
            }
            let highlight = Rect::from_min_max(
                egui::pos2(
                    rect.left() + f32::from(from_col) * char_width,
                    rect.top() + f32::from(row) * line_height,
                ),
                egui::pos2(
                    rect.left() + f32::from(to_col + 1) * char_width,
                    rect.top() + f32::from(row + 1) * line_height,
                ),
            );
            ui.painter().rect_filled(
                highlight,
                CornerRadius::ZERO,
                Color32::from_rgba_unmultiplied(
                    terminal_palette.selection.r(),
                    terminal_palette.selection.g(),
                    terminal_palette.selection.b(),
                    72,
                ),
            );
        }
    }
    for row in 0..rows {
        let y = rect.top() + f32::from(row) * line_height + line_height * 0.5;
        let mut run_start = 0_u16;
        let mut run_color = terminal_palette.foreground;
        let mut run_text = String::new();
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if cell.is_wide_continuation() {
                continue;
            }
            let color = terminal_color(cell.fgcolor(), terminal_palette, cell.bold());
            let contents = if cell.contents().is_empty() {
                " "
            } else {
                cell.contents()
            };
            if !run_text.is_empty() && color != run_color {
                paint_run(
                    ui, rect, &run_text, run_start, y, font, char_width, run_color,
                );
                run_text.clear();
                run_start = col;
            } else if run_text.is_empty() {
                run_start = col;
            }
            run_text.push_str(contents);
            run_color = color;
        }
        if !run_text.is_empty() {
            paint_run(
                ui, rect, &run_text, run_start, y, font, char_width, run_color,
            );
        }
    }

    if !screen.hide_cursor() && screen.scrollback() == 0 {
        let (row, col) = screen.cursor_position();
        let cursor = Rect::from_min_size(
            egui::pos2(
                rect.left() + f32::from(col) * char_width,
                rect.top() + f32::from(row) * line_height + 1.0,
            ),
            Vec2::new(2.0, line_height - 2.0),
        );
        ui.painter()
            .rect_filled(cursor, CornerRadius::ZERO, terminal_palette.cursor);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_run(
    ui: &Ui,
    rect: Rect,
    text: &str,
    start_col: u16,
    y: f32,
    font: &FontId,
    char_width: f32,
    color: Color32,
) {
    ui.painter().text(
        egui::pos2(rect.left() + f32::from(start_col) * char_width, y),
        egui::Align2::LEFT_CENTER,
        text,
        font.clone(),
        color,
    );
}

fn terminal_color(color: vt100::Color, palette: TerminalPalette, bold: bool) -> Color32 {
    let base = match color {
        vt100::Color::Default => palette.foreground,
        vt100::Color::Idx(index) => ansi_color(index, palette),
        vt100::Color::Rgb(red, green, blue) => Color32::from_rgb(red, green, blue),
    };
    if bold {
        base.gamma_multiply(1.12)
    } else {
        base
    }
}

fn ansi_color(index: u8, palette: TerminalPalette) -> Color32 {
    match index {
        0..=15 => palette.color(index),
        16..=231 => {
            let cube = index - 16;
            let red = cube / 36;
            let green = (cube % 36) / 6;
            let blue = cube % 6;
            Color32::from_rgb(
                cube_component(red),
                cube_component(green),
                cube_component(blue),
            )
        }
        _ => {
            let level = 8 + (index - 232) * 10;
            Color32::from_gray(level)
        }
    }
}

fn cube_component(value: u8) -> u8 {
    if value == 0 {
        0
    } else {
        55 + value * 40
    }
}

fn pointer_cell(
    pointer: egui::Pos2,
    rect: Rect,
    char_width: f32,
    line_height: f32,
    rows: u16,
    cols: u16,
) -> (u16, u16) {
    let col = ((pointer.x - rect.left()) / char_width)
        .floor()
        .clamp(0.0, f32::from(cols.saturating_sub(1))) as u16;
    let row = ((pointer.y - rect.top()) / line_height)
        .floor()
        .clamp(0.0, f32::from(rows.saturating_sub(1))) as u16;
    (row, col)
}

#[derive(Default)]
struct TerminalInput {
    bytes: Vec<u8>,
    copy_selection: bool,
    paste_from_clipboard: bool,
}

fn collect_terminal_input(
    events: &[egui::Event],
    has_selection: bool,
    ctrl_down: bool,
) -> TerminalInput {
    let mut input = TerminalInput::default();
    let has_paste_event = events
        .iter()
        .any(|event| matches!(event, egui::Event::Paste(_)));
    for event in events {
        match event {
            egui::Event::Copy => {
                if has_selection {
                    input.copy_selection = true;
                } else if !cfg!(target_os = "macos") || ctrl_down {
                    input.bytes.push(0x03);
                }
            }
            egui::Event::Paste(text) => input.bytes.extend_from_slice(&paste_bytes(text)),
            egui::Event::Text(text) => input.bytes.extend_from_slice(text.as_bytes()),
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if (modifiers.ctrl || modifiers.command) && *key == egui::Key::C {
                    if has_selection {
                        input.copy_selection = true;
                    } else if modifiers.ctrl && !modifiers.command {
                        input.bytes.push(0x03);
                    }
                    continue;
                }
                if (modifiers.ctrl || modifiers.command) && *key == egui::Key::V {
                    if !has_paste_event {
                        input.paste_from_clipboard = true;
                    }
                    continue;
                }
                if modifiers.ctrl && !modifiers.command {
                    if let Some(byte) = ctrl_key_byte(*key) {
                        input.bytes.push(byte);
                        continue;
                    }
                }
                input.bytes.extend_from_slice(key_sequence(*key));
            }
            _ => {}
        }
    }
    input
}

fn paste_bytes(text: &str) -> Vec<u8> {
    text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
}

fn clipboard_text() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

fn selection_text(screen: &vt100::Screen, selection: TerminalSelection) -> String {
    let (rows, cols) = screen.size();
    let ((start_row, start_col), (end_row, end_col)) = selection.ordered();
    let start_row = start_row.min(rows.saturating_sub(1));
    let end_row = end_row.min(rows.saturating_sub(1));
    let start_col = start_col.min(cols.saturating_sub(1));
    let end_col = end_col.min(cols.saturating_sub(1));
    let mut output = String::new();
    for row in start_row..=end_row {
        let from_col = if row == start_row { start_col } else { 0 };
        let to_col = if row == end_row { end_col } else { cols - 1 };
        let mut line = String::new();
        let mut col = from_col;
        while col <= to_col {
            let Some(cell) = screen.cell(row, col) else {
                col = col.saturating_add(1);
                continue;
            };
            if !cell.is_wide_continuation() {
                if cell.has_contents() {
                    line.push_str(cell.contents());
                } else {
                    line.push(' ');
                }
            }
            col = col.saturating_add(1);
        }
        output.push_str(line.trim_end());
        if row < end_row {
            output.push('\n');
        }
    }
    output
}

fn key_sequence(key: egui::Key) -> &'static [u8] {
    match key {
        egui::Key::Enter => b"\r",
        egui::Key::Backspace => b"\x7f",
        egui::Key::Tab => b"\t",
        egui::Key::Escape => b"\x1b",
        egui::Key::ArrowUp => b"\x1b[A",
        egui::Key::ArrowDown => b"\x1b[B",
        egui::Key::ArrowRight => b"\x1b[C",
        egui::Key::ArrowLeft => b"\x1b[D",
        egui::Key::Home => b"\x1b[H",
        egui::Key::End => b"\x1b[F",
        egui::Key::Delete => b"\x1b[3~",
        egui::Key::PageUp => b"\x1b[5~",
        egui::Key::PageDown => b"\x1b[6~",
        _ => b"",
    }
}

fn ctrl_key_byte(key: egui::Key) -> Option<u8> {
    let letter = match key {
        egui::Key::A => b'a',
        egui::Key::B => b'b',
        egui::Key::C => b'c',
        egui::Key::D => b'd',
        egui::Key::E => b'e',
        egui::Key::F => b'f',
        egui::Key::G => b'g',
        egui::Key::H => b'h',
        egui::Key::I => b'i',
        egui::Key::J => b'j',
        egui::Key::K => b'k',
        egui::Key::L => b'l',
        egui::Key::M => b'm',
        egui::Key::N => b'n',
        egui::Key::O => b'o',
        egui::Key::P => b'p',
        egui::Key::Q => b'q',
        egui::Key::R => b'r',
        egui::Key::S => b's',
        egui::Key::T => b't',
        egui::Key::U => b'u',
        egui::Key::V => b'v',
        egui::Key::W => b'w',
        egui::Key::X => b'x',
        egui::Key::Y => b'y',
        egui::Key::Z => b'z',
        _ => return None,
    };
    Some(letter - b'a' + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn selection_text_copies_multiple_rows() {
        let mut parser = vt100::Parser::new(4, 20, 0);
        parser.process(b"hello\r\nworld");
        let selection = TerminalSelection {
            anchor_row: 0,
            anchor_col: 0,
            cursor_row: 1,
            cursor_col: 4,
        };
        assert_eq!(selection_text(parser.screen(), selection), "hello\nworld");
    }

    #[test]
    fn copy_event_copies_the_current_selection() {
        let input = collect_terminal_input(&[egui::Event::Copy], true, true);

        assert!(input.copy_selection);
        assert!(input.bytes.is_empty());
    }

    #[test]
    fn copy_event_without_selection_sends_ctrl_c() {
        let input = collect_terminal_input(&[egui::Event::Copy], false, true);

        assert!(!input.copy_selection);
        assert_eq!(input.bytes, vec![0x03]);
    }

    #[test]
    fn paste_normalizes_windows_and_unix_line_endings() {
        let input = collect_terminal_input(
            &[egui::Event::Paste("one\r\ntwo\nthree".to_owned())],
            false,
            false,
        );

        assert_eq!(input.bytes, b"one\rtwo\rthree");
    }

    #[test]
    fn ctrl_v_requests_the_clipboard_when_no_paste_event_was_emitted() {
        let modifiers = egui::Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let input = collect_terminal_input(&[key_event(egui::Key::V, modifiers)], false, true);

        assert!(input.paste_from_clipboard);
        assert!(input.bytes.is_empty());
    }

    #[test]
    fn ansi_256_colors_use_the_extended_palette() {
        let palette = TerminalPalette::for_theme(crate::domain::TerminalColorTheme::RShellDark);
        assert_eq!(
            terminal_color(vt100::Color::Idx(1), palette, false),
            palette.color(1)
        );
        assert_eq!(
            terminal_color(vt100::Color::Idx(196), palette, false),
            Color32::from_rgb(255, 0, 0)
        );
        assert_eq!(
            terminal_color(vt100::Color::Idx(232), palette, false),
            Color32::from_rgb(8, 8, 8)
        );
        assert_eq!(
            terminal_color(vt100::Color::Idx(255), palette, false),
            Color32::from_rgb(238, 238, 238)
        );
    }

    #[test]
    fn ansi_sgr_colors_reach_screen_cells() {
        let mut parser = vt100::Parser::new(4, 20, 0);
        parser.process(b"\x1b[01;34mblue\x1b[0m plain");
        let screen = parser.screen();
        let blue = screen.cell(0, 0).unwrap();
        assert_eq!(blue.fgcolor(), vt100::Color::Idx(4));
        assert!(blue.bold());
        assert_eq!(screen.cell(0, 5).unwrap().fgcolor(), vt100::Color::Default);
    }
}
