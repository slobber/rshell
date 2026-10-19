use eframe::egui::{self, Align2, Color32, Event, FontId, Rect, RichText, Sense, Ui, Vec2};
use ironrdp_pdu::input::fast_path::{FastPathInputEvent, KeyboardFlags};
use ironrdp_pdu::input::mouse::{MousePdu, PointerFlags};
use smallvec::SmallVec;

use crate::app::ShellCoreApp;
use crate::domain::SessionTab;
use crate::theme::{Palette, MIN_UI_FONT_SIZE};

pub fn draw(app: &mut ShellCoreApp, ui: &mut Ui, tab: &SessionTab, palette: Palette) {
    let window_focused = ui.input(|input| input.focused);
    crate::rdp::keyboard_capture::set_enabled(window_focused && app.rdp.error(tab.id).is_none());
    for key in crate::rdp::keyboard_capture::drain() {
        let mut flags = KeyboardFlags::empty();
        if key.extended {
            flags |= KeyboardFlags::EXTENDED;
        }
        if key.release {
            flags |= KeyboardFlags::RELEASE;
        }
        app.rdp.send_input(
            tab.id,
            SmallVec::from_slice(&[FastPathInputEvent::KeyboardEvent(flags, key.scan_code)]),
        );
    }

    let Some(frame) = app.rdp.frame(tab.id) else {
        let message = app
            .rdp
            .error(tab.id)
            .map(str::to_owned)
            .unwrap_or_else(|| app.t("rdp_connecting").to_owned());
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new(message).size(MIN_UI_FONT_SIZE).color(
                if app.rdp.error(tab.id).is_some() {
                    palette.red
                } else {
                    palette.muted
                },
            ));
        });
        return;
    };

    let outer = ui.available_rect_before_wrap();
    let pixels_per_point = ui.ctx().pixels_per_point();
    let native_pixels_per_point = ui.ctx().input(|input| {
        input
            .viewport()
            .native_pixels_per_point
            .unwrap_or(pixels_per_point)
    });
    let target_width = (outer.width() * pixels_per_point)
        .round()
        .clamp(640.0, 8192.0) as u16;
    let target_height = (outer.height() * pixels_per_point)
        .round()
        .clamp(480.0, 8192.0) as u16;
    let scale_factor =
        ((native_pixels_per_point * 100.0 / 25.0).round() * 25.0).clamp(100.0, 500.0) as u32;
    app.rdp
        .resize_to_view(tab.id, target_width, target_height, scale_factor);
    ui.painter().rect_filled(outer, 0.0, Color32::BLACK);
    let scale = (outer.width() / f32::from(frame.width))
        .min(outer.height() / f32::from(frame.height))
        .max(0.01);
    let display_size = Vec2::new(
        f32::from(frame.width) * scale,
        f32::from(frame.height) * scale,
    );
    let display = Rect::from_center_size(outer.center(), display_size);
    let response = ui.interact(
        display,
        ui.scope_id().with(("rdp_surface", tab.id)),
        Sense::click_and_drag(),
    );
    let focus_requested = app.pending_focus_tab == Some(tab.id);
    if focus_requested || response.clicked() {
        response.request_focus();
    }
    if focus_requested {
        app.pending_focus_tab = None;
    }

    let Some(texture) = app.rdp.texture(ui.ctx(), tab.id) else {
        return;
    };
    ui.painter().image(
        texture.id(),
        display,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );

    let remote_point = |pos: egui::Pos2| {
        let x = ((pos.x - display.left()) / display.width()).clamp(0.0, 1.0);
        let y = ((pos.y - display.top()) / display.height()).clamp(0.0, 1.0);
        (
            (x * f32::from(frame.width))
                .round()
                .min(f32::from(frame.width.saturating_sub(1))) as u16,
            (y * f32::from(frame.height))
                .round()
                .min(f32::from(frame.height.saturating_sub(1))) as u16,
        )
    };
    let mouse_at = |pos: egui::Pos2, flags: PointerFlags| {
        let (x, y) = remote_point(pos);
        app.rdp.send_input(
            tab.id,
            SmallVec::from_slice(&[FastPathInputEvent::MouseEvent(MousePdu {
                flags,
                number_of_wheel_rotation_units: 0,
                x_position: x,
                y_position: y,
            })]),
        )
    };

    if let Some(pos) = response.hover_pos() {
        mouse_at(pos, PointerFlags::MOVE);
    }
    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            mouse_at(pos, PointerFlags::LEFT_BUTTON | PointerFlags::DOWN);
        }
    }
    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            mouse_at(pos, PointerFlags::LEFT_BUTTON | PointerFlags::MOVE);
        }
    }
    if response.drag_stopped() {
        if let Some(pos) = response.interact_pointer_pos() {
            mouse_at(pos, PointerFlags::LEFT_BUTTON);
        }
    }
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            mouse_at(pos, PointerFlags::LEFT_BUTTON | PointerFlags::DOWN);
            mouse_at(pos, PointerFlags::LEFT_BUTTON);
        }
    }
    if response.secondary_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            mouse_at(pos, PointerFlags::RIGHT_BUTTON | PointerFlags::DOWN);
            mouse_at(pos, PointerFlags::RIGHT_BUTTON);
        }
    }
    if response.middle_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            mouse_at(
                pos,
                PointerFlags::MIDDLE_BUTTON_OR_WHEEL | PointerFlags::DOWN,
            );
            mouse_at(pos, PointerFlags::MIDDLE_BUTTON_OR_WHEEL);
        }
    }

    if response.hovered() {
        let wheel_events = ui.ctx().input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::MouseWheel { unit, delta, .. } if delta.y.abs() > 0.001 => {
                        Some((*unit, delta.y))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
        if let Some(pos) = response.hover_pos() {
            for (unit, delta_y) in wheel_events {
                let units = wheel_rotation_units(unit, delta_y);
                if units == 0 {
                    continue;
                }
                let (x, y) = remote_point(pos);
                app.rdp.send_input(
                    tab.id,
                    SmallVec::from_slice(&[FastPathInputEvent::MouseEvent(MousePdu {
                        flags: PointerFlags::VERTICAL_WHEEL,
                        number_of_wheel_rotation_units: units,
                        x_position: x,
                        y_position: y,
                    })]),
                );
            }
        }
    }

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
        let events = ui.ctx().input(|input| input.events.clone());
        let modifiers = ui.ctx().input(|input| input.modifiers);
        for event in events {
            match event {
                Event::Text(text) if !modifiers.ctrl && !modifiers.alt => {
                    for character in text.chars() {
                        if character.is_control() {
                            continue;
                        }
                        let code = character as u32;
                        if code <= u32::from(u16::MAX) {
                            send_unicode(app, tab.id, code as u16);
                        }
                    }
                }
                Event::Paste(_) if modifiers.ctrl => {
                    send_shortcut(
                        app,
                        tab.id,
                        egui::Modifiers {
                            ctrl: true,
                            ..Default::default()
                        },
                        0x2f,
                    );
                }
                Event::Paste(text) if !modifiers.ctrl && !modifiers.alt => {
                    for character in text.chars() {
                        if character.is_control() {
                            continue;
                        }
                        let code = character as u32;
                        if code <= u32::from(u16::MAX) {
                            send_unicode(app, tab.id, code as u16);
                        }
                    }
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if modifiers.ctrl || modifiers.alt {
                        if let Some(code) = shortcut_scancode(key) {
                            send_shortcut(app, tab.id, modifiers, code);
                        }
                    } else if let Some(code) = special_scancode(key) {
                        send_key(app, tab.id, code, is_extended_key(key), false);
                    }
                }
                _ => {}
            }
        }
    }

    let capture = if crate::rdp::keyboard_capture::is_enabled() {
        format!("  ·  {}", app.t("keyboard_capture"))
    } else {
        String::new()
    };
    let status = format!(
        "RDP  {}×{}  ·  {}{capture}",
        frame.width,
        frame.height,
        app.t("rdp_protocol_hint")
    );
    ui.painter().text(
        egui::pos2(outer.left() + 8.0, outer.bottom() - 8.0),
        Align2::LEFT_BOTTOM,
        status,
        FontId::monospace(crate::theme::STATUS_FONT_SIZE),
        palette.muted,
    );
}

fn wheel_rotation_units(unit: egui::MouseWheelUnit, delta: f32) -> i16 {
    let units = match unit {
        egui::MouseWheelUnit::Line | egui::MouseWheelUnit::Page => (delta * 100.0).round(),
        egui::MouseWheelUnit::Point => delta.round(),
    } as i16;
    units.clamp(
        *MousePdu::WHEEL_ROTATION_RANGE.start(),
        *MousePdu::WHEEL_ROTATION_RANGE.end(),
    )
}

fn send_unicode(app: &mut ShellCoreApp, tab_id: u64, code: u16) {
    let press = FastPathInputEvent::UnicodeKeyboardEvent(KeyboardFlags::empty(), code);
    let release = FastPathInputEvent::UnicodeKeyboardEvent(KeyboardFlags::RELEASE, code);
    app.rdp
        .send_input(tab_id, SmallVec::from_buf([press, release]));
}

fn send_shortcut(app: &mut ShellCoreApp, tab_id: u64, modifiers: egui::Modifiers, key_code: u8) {
    let mut events = SmallVec::new();
    if modifiers.ctrl {
        events.push(FastPathInputEvent::KeyboardEvent(
            KeyboardFlags::empty(),
            0x1d,
        ));
    }
    if modifiers.alt {
        events.push(FastPathInputEvent::KeyboardEvent(
            KeyboardFlags::empty(),
            0x38,
        ));
    }
    if modifiers.shift {
        events.push(FastPathInputEvent::KeyboardEvent(
            KeyboardFlags::empty(),
            0x2a,
        ));
    }
    events.push(FastPathInputEvent::KeyboardEvent(
        KeyboardFlags::empty(),
        key_code,
    ));
    events.push(FastPathInputEvent::KeyboardEvent(
        KeyboardFlags::RELEASE,
        key_code,
    ));
    if modifiers.shift {
        events.push(FastPathInputEvent::KeyboardEvent(
            KeyboardFlags::RELEASE,
            0x2a,
        ));
    }
    if modifiers.alt {
        events.push(FastPathInputEvent::KeyboardEvent(
            KeyboardFlags::RELEASE,
            0x38,
        ));
    }
    if modifiers.ctrl {
        events.push(FastPathInputEvent::KeyboardEvent(
            KeyboardFlags::RELEASE,
            0x1d,
        ));
    }
    app.rdp.send_input(tab_id, events);
}

fn send_key(app: &mut ShellCoreApp, tab_id: u64, code: u8, extended: bool, release: bool) {
    let mut flags = KeyboardFlags::empty();
    if extended {
        flags |= KeyboardFlags::EXTENDED;
    }
    if release {
        flags |= KeyboardFlags::RELEASE;
    }
    app.rdp.send_input(
        tab_id,
        SmallVec::from_slice(&[FastPathInputEvent::KeyboardEvent(flags, code)]),
    );
    if !release {
        let mut release_flags = KeyboardFlags::RELEASE;
        if extended {
            release_flags |= KeyboardFlags::EXTENDED;
        }
        app.rdp.send_input(
            tab_id,
            SmallVec::from_slice(&[FastPathInputEvent::KeyboardEvent(release_flags, code)]),
        );
    }
}

fn shortcut_scancode(key: egui::Key) -> Option<u8> {
    use egui::Key;
    Some(match key {
        Key::A => 0x1e,
        Key::B => 0x30,
        Key::C => 0x2e,
        Key::D => 0x20,
        Key::E => 0x12,
        Key::F => 0x21,
        Key::G => 0x22,
        Key::H => 0x23,
        Key::I => 0x17,
        Key::J => 0x24,
        Key::K => 0x25,
        Key::L => 0x26,
        Key::M => 0x32,
        Key::N => 0x31,
        Key::O => 0x18,
        Key::P => 0x19,
        Key::Q => 0x10,
        Key::R => 0x13,
        Key::S => 0x1f,
        Key::T => 0x14,
        Key::U => 0x16,
        Key::V => 0x2f,
        Key::W => 0x11,
        Key::X => 0x2d,
        Key::Y => 0x15,
        Key::Z => 0x2c,
        Key::Num0 => 0x0b,
        Key::Num1 => 0x02,
        Key::Num2 => 0x03,
        Key::Num3 => 0x04,
        Key::Num4 => 0x05,
        Key::Num5 => 0x06,
        Key::Num6 => 0x07,
        Key::Num7 => 0x08,
        Key::Num8 => 0x09,
        Key::Num9 => 0x0a,
        Key::Minus => 0x0c,
        Key::Plus => 0x0d,
        Key::Equals => 0x0d,
        Key::OpenBracket => 0x1a,
        Key::CloseBracket => 0x1b,
        Key::Backslash => 0x2b,
        Key::Semicolon => 0x27,
        Key::Quote => 0x28,
        Key::Comma => 0x33,
        Key::Period => 0x34,
        Key::Slash => 0x35,
        Key::Backtick => 0x29,
        _ => return special_scancode(key),
    })
}

fn special_scancode(key: egui::Key) -> Option<u8> {
    use egui::Key;
    Some(match key {
        Key::Enter => 0x1c,
        Key::Escape => 0x01,
        Key::Backspace => 0x0e,
        Key::Tab => 0x0f,
        Key::Space => 0x39,
        Key::Delete => 0x53,
        Key::Insert => 0x52,
        Key::Home => 0x47,
        Key::End => 0x4f,
        Key::PageUp => 0x49,
        Key::PageDown => 0x51,
        Key::ArrowUp => 0x48,
        Key::ArrowDown => 0x50,
        Key::ArrowLeft => 0x4b,
        Key::ArrowRight => 0x4d,
        Key::F1 => 0x3b,
        Key::F2 => 0x3c,
        Key::F3 => 0x3d,
        Key::F4 => 0x3e,
        Key::F5 => 0x3f,
        Key::F6 => 0x40,
        Key::F7 => 0x41,
        Key::F8 => 0x42,
        Key::F9 => 0x43,
        Key::F10 => 0x44,
        Key::F11 => 0x57,
        Key::F12 => 0x58,
        _ => return None,
    })
}

fn is_extended_key(key: egui::Key) -> bool {
    use egui::Key;
    matches!(
        key,
        Key::Insert
            | Key::Delete
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::ArrowLeft
            | Key::ArrowRight
    )
}

#[cfg(test)]
mod tests {
    use super::wheel_rotation_units;
    use eframe::egui::MouseWheelUnit;

    #[test]
    fn maps_wheel_lines_to_rdp_rotation_units() {
        assert_eq!(wheel_rotation_units(MouseWheelUnit::Line, 1.0), 100);
        assert_eq!(wheel_rotation_units(MouseWheelUnit::Line, -1.0), -100);
        assert_eq!(wheel_rotation_units(MouseWheelUnit::Point, 3.0), 3);
        assert_eq!(wheel_rotation_units(MouseWheelUnit::Line, 4.0), 255);
    }
}
