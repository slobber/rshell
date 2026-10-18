use std::time::{Duration, Instant};

use eframe::egui::{self, Key, Modifiers, ViewportCommand};

use crate::domain::{Axis, TransferStatus};

use super::ShellCoreApp;

impl ShellCoreApp {
    pub(super) fn tick(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.last_tick = now;
        let rdp_active = self
            .workspace
            .active_tab()
            .is_some_and(|tab| tab.kind == crate::domain::TabKind::Rdp);
        if !rdp_active {
            crate::rdp::keyboard_capture::set_enabled(false);
        }
        self.poll_ssh_events(ctx);
        self.poll_rdp_events(ctx);

        if self
            .toast
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(3))
        {
            self.toast = None;
        }

        let active_transfers = self
            .transfers
            .iter()
            .any(|transfer| transfer.status == TransferStatus::Active);
        ctx.request_repaint_after(if active_transfers || self.rdp.has_sessions() {
            Duration::from_millis(80)
        } else {
            Duration::from_millis(500)
        });
    }

    pub(super) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.rdp_fullscreen_tab.is_some()
            && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.exit_rdp_fullscreen(ctx);
            return;
        }
        if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F11)) {
            if self.rdp_fullscreen_tab.is_some() {
                self.exit_rdp_fullscreen(ctx);
            } else if let Some(tab_id) = self
                .workspace
                .active_tab()
                .filter(|tab| tab.kind == crate::domain::TabKind::Rdp)
                .map(|tab| tab.id)
            {
                self.enter_rdp_fullscreen(tab_id, ctx);
            }
            return;
        }
        if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
            self.settings_open = false;
            self.keyring_open = false;
            self.snippets_open = false;
            self.about_open = false;
            self.command_palette_open = false;
            self.host_dialog = None;
            self.folder_dialog = None;
            self.folder_delete = None;
            self.editor_dialog = None;
            self.chmod_dialog = None;
            self.md5_dialog = None;
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            if ctx.input_mut(|input| {
                input.consume_key(
                    Modifiers {
                        command: true,
                        shift: true,
                        ..Default::default()
                    },
                    Key::P,
                )
            }) {
                self.command_palette_open = true;
                self.command_palette_query.clear();
            }
            return;
        }
        let command = Modifiers::COMMAND;
        let shifted = Modifiers {
            command: true,
            shift: true,
            ..Default::default()
        };
        if ctx.input_mut(|input| input.consume_key(command, Key::N)) {
            self.open_new_session();
        } else if ctx.input_mut(|input| input.consume_key(command, Key::T)) {
            self.new_terminal_tab();
        } else if ctx.input_mut(|input| input.consume_key(shifted, Key::S)) {
            self.new_sftp_tab();
        } else if ctx.input_mut(|input| input.consume_key(command, Key::W)) {
            self.close_focused_tab();
        } else if ctx.input_mut(|input| input.consume_key(command, Key::B)) {
            self.broadcast = !self.broadcast;
        } else if ctx.input_mut(|input| input.consume_key(command, Key::D)) {
            self.split_active(Axis::Horizontal);
        } else if ctx.input_mut(|input| input.consume_key(shifted, Key::D)) {
            self.split_active(Axis::Vertical);
        } else if ctx.input_mut(|input| input.consume_key(command, Key::P)) {
            self.command_palette_open = true;
            self.command_palette_query.clear();
        } else if ctx.input_mut(|input| input.consume_key(command, Key::Comma)) {
            self.settings_open = true;
        } else if ctx.input_mut(|input| input.consume_key(command, Key::Backtick)) {
            self.settings.console_collapsed = !self.settings.console_collapsed;
            self.persist_settings();
        } else if ctx.input_mut(|input| input.consume_key(shifted, Key::L)) {
            self.toggle_theme(ctx);
        } else if ctx.input_mut(|input| input.consume_key(command, Key::Q)) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        } else if ctx.input_mut(|input| input.consume_key(command, Key::Plus)) {
            self.settings.terminal_zoom = (self.settings.terminal_zoom + 0.1).min(1.6);
            self.persist_settings();
        } else if ctx.input_mut(|input| input.consume_key(command, Key::Minus)) {
            self.settings.terminal_zoom = (self.settings.terminal_zoom - 0.1).max(0.75);
            self.persist_settings();
        }
    }
}
