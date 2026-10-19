mod bootstrap;
mod command_history;
mod file_actions;
mod key_ops;
mod persistence;
mod queries;
mod rdp_events;
mod runtime;
mod security_flow;
pub(crate) mod sessions;
mod ssh_events;
mod state;
mod terminal_ops;
mod transfers;

use std::collections::HashSet;
use std::time::Instant;

use crate::config::AppConfig;
use crate::domain::{
    AppSettings, Axis, HostFolder, LogEntry, RememberDuration, ThemeMode, TransferTask, Tunnel,
    Workspace,
};
use crate::fonts::FontCatalog;
use crate::rdp::RdpManager;
use crate::security::DataCipher;
use crate::ssh::SshManager;
use crate::storage::{Persistence, SavedKeySummary, SecurityRecord};
use crate::ui;

pub use state::{
    AppLock, ChmodDialog, CommandHistoryDialog, ConsoleTab, DropTarget, EditorDialog, FolderDraft,
    FontCache, HostDraft, HostTreeDrag, LogSelection, Md5Dialog, SecurityAction, SidebarTab,
    TabDrag, TreeItem, TreeRename,
};

pub struct ShellCoreApp {
    pub config: AppConfig,
    pub storage: Option<Persistence>,
    pub storage_error: Option<String>,
    pub security_record: SecurityRecord,
    pub cipher: Option<DataCipher>,
    pub lock_state: AppLock,
    pub settings: AppSettings,
    pub fonts: FontCatalog,
    pub applied_fonts: FontCache,
    pub runtime_theme: ThemeMode,
    pub hosts: Vec<HostFolder>,
    pub active_host_id: Option<u64>,
    pub active_tree_item: Option<TreeItem>,
    pub tree_rename: Option<TreeRename>,
    pub connected_hosts: HashSet<u64>,
    pub rdp_compat_retried: HashSet<u64>,
    pub workspace: Workspace,
    pub next_tab_id: u64,
    pub next_aux_session_id: u64,
    pub sidebar_tab: SidebarTab,
    pub console_tab: ConsoleTab,
    pub host_search: String,
    pub host_search_open: bool,
    pub logs: Vec<LogEntry>,
    pub log_selection: Option<LogSelection>,
    pub transfers: Vec<TransferTask>,
    pub tunnels: Vec<Tunnel>,
    pub next_transfer_id: u64,
    pub ssh: SshManager,
    pub rdp: RdpManager,
    pub broadcast: bool,
    pub toast: Option<(String, Instant)>,
    pub last_tick: Instant,
    pub copy_feedback: Option<(u64, Instant)>,
    pub drag: Option<TabDrag>,
    pub drop_target: Option<DropTarget>,
    pub pending_split: Option<Axis>,
    pub pending_focus_tab: Option<u64>,
    pub rdp_fullscreen_tab: Option<u64>,
    pub maximized_pane: Option<u64>,
    pub host_dialog: Option<HostDraft>,
    pub folder_dialog: Option<FolderDraft>,
    pub folder_delete: Option<u64>,
    pub host_delete: Option<u64>,
    pub command_history: Option<CommandHistoryDialog>,
    pub pending_command_history: Vec<(u64, String)>,
    pub settings_open: bool,
    pub keyring_open: bool,
    pub snippets_open: bool,
    pub about_open: bool,
    pub command_palette_open: bool,
    pub command_palette_query: String,
    pub editor_dialog: Option<EditorDialog>,
    pub chmod_dialog: Option<ChmodDialog>,
    pub md5_dialog: Option<Md5Dialog>,
    pub unlock_password: String,
    pub unlock_remember: RememberDuration,
    pub unlock_error: Option<String>,
    pub reset_confirmation: bool,
    pub security_current: String,
    pub security_new: String,
    pub security_confirm: String,
    pub security_remember: RememberDuration,
    pub security_notice: Option<(bool, String)>,
    pub saved_keys: Vec<SavedKeySummary>,
    pub key_name: String,
    pub key_file_path: String,
    pub keyring_error: Option<String>,
    pub font_filter_ui: String,
    pub font_filter_mono: String,
    pub window_maximized: bool,
}

impl eframe::App for ShellCoreApp {
    fn logic(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);
        if self.lock_state == AppLock::Unlocked {
            self.handle_shortcuts(ctx);
        }
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(tab_id) = self.rdp_fullscreen_tab {
            let tab = self.workspace.find_tab(tab_id).cloned();
            if !tab
                .as_ref()
                .is_some_and(|tab| tab.kind == crate::domain::TabKind::Rdp)
            {
                self.exit_rdp_fullscreen(&ctx);
            } else if let Some(tab) = tab {
                let palette = crate::theme::Palette::for_mode(self.settings.theme);
                let exit_label = format!("{}  Esc", self.t("exit_fullscreen"));
                let mut exit = false;
                eframe::egui::CentralPanel::default()
                    .frame(
                        eframe::egui::Frame::new()
                            .fill(eframe::egui::Color32::BLACK)
                            .inner_margin(eframe::egui::Margin::ZERO),
                    )
                    .show(ui, |ui| {
                        ui::rdp::draw(self, ui, &tab, palette);
                        eframe::egui::Area::new(eframe::egui::Id::unique("rdp_fullscreen_exit"))
                            .anchor(eframe::egui::Align2::RIGHT_TOP, [-14.0, 14.0])
                            .order(eframe::egui::Order::Foreground)
                            .show(&ctx, |ui| {
                                eframe::egui::Frame::new()
                                    .fill(palette.panel)
                                    .stroke(eframe::egui::Stroke::new(1.0, palette.border_strong))
                                    .corner_radius(eframe::egui::CornerRadius::same(5))
                                    .inner_margin(eframe::egui::Margin::symmetric(8, 4))
                                    .show(ui, |ui| {
                                        exit = ui
                                            .button(
                                                eframe::egui::RichText::new(exit_label)
                                                    .size(crate::theme::MIN_UI_FONT_SIZE),
                                            )
                                            .clicked();
                                    });
                            });
                    });
                if exit {
                    self.exit_rdp_fullscreen(&ctx);
                }
                return;
            }
        }
        ui::shell::title_bar(self, ui);
        if self.lock_state != AppLock::Unlocked {
            ui::dialogs::lock_screen(self, ui);
            ui::dialogs::toast(self, &ctx);
            return;
        }
        ui::shell::quick_ribbon(self, ui);
        ui::shell::status_bar(self, ui);
        ui::sidebar::draw(self, ui);
        ui::console::draw(self, ui);
        ui::workspace::draw(self, ui);
        self.flush_command_history();
        ui::dialogs::draw_modals(self, &ctx);
        ui::shell::resize_handles(self, &ctx);
        ui::dialogs::drag_ghost(self, &ctx);
        ui::dialogs::toast(self, &ctx);

        if ctx.input(|input| input.pointer.any_released()) {
            if let (Some(drag), Some(target)) = (self.drag.clone(), self.drop_target) {
                self.move_tab(&drag, target);
            }
            self.drag = None;
            self.drop_target = None;
        }
        if let Some(axis) = self.pending_split.take() {
            self.split_active(axis);
        }
    }
}
