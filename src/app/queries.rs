use std::collections::HashSet;
use std::time::Instant;

use eframe::egui;

use crate::domain::{
    find_host, ConnectionBackend, Host, HostFolder, LogEntry, LogLevel, SessionTab,
};
use crate::i18n::tr;
use crate::theme::install_theme;

use super::{FontCache, ShellCoreApp};

impl ShellCoreApp {
    pub fn t(&self, key: &'static str) -> &'static str {
        tr(self.settings.language, key)
    }

    pub fn active_host(&self) -> Option<&Host> {
        self.active_host_id
            .and_then(|id| find_host(&self.hosts, id))
    }

    pub fn active_host_clone(&self) -> Option<Host> {
        self.active_host().cloned()
    }

    pub fn has_hosts(&self) -> bool {
        self.hosts.iter().any(|folder| folder.host_count() > 0)
    }

    pub fn connected_ssh_hosts(&self) -> Vec<(u64, String)> {
        fn visit(
            folders: &[HostFolder],
            connected: &HashSet<u64>,
            output: &mut Vec<(u64, String)>,
        ) {
            for folder in folders {
                for host in &folder.hosts {
                    if host.backend == ConnectionBackend::Ssh && connected.contains(&host.id) {
                        output.push((host.id, host.name.clone()));
                    }
                }
                visit(&folder.children, connected, output);
            }
        }
        let mut hosts = Vec::new();
        visit(&self.hosts, &self.connected_hosts, &mut hosts);
        hosts
    }

    pub fn focused_tab(&self) -> Option<&SessionTab> {
        self.workspace.active_tab()
    }

    pub fn focused_tab_mut(&mut self) -> Option<&mut SessionTab> {
        self.workspace.active_tab_mut()
    }

    pub fn toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }

    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.logs.push(LogEntry::now(level, message));
        if self.logs.len() > 800 {
            self.logs.remove(0);
            self.log_selection = None;
        }
    }

    pub fn apply_theme(&mut self, ctx: &egui::Context) {
        install_theme(ctx, self.settings.theme);
    }

    pub fn apply_fonts(&mut self, ctx: &egui::Context) -> Result<(), String> {
        self.fonts
            .apply(ctx, &self.settings.ui_font, &self.settings.terminal_font)?;
        self.applied_fonts = FontCache {
            ui_font: self.settings.ui_font.clone(),
            terminal_font: self.settings.terminal_font.clone(),
        };
        Ok(())
    }

    pub fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.settings.theme = self.settings.theme.toggled();
        self.runtime_theme = self.settings.theme;
        install_theme(ctx, self.settings.theme);
        self.persist_settings();
    }
}
