use std::collections::HashSet;
use std::time::Instant;

use crate::config::AppConfig;
use crate::domain::{
    AppSettings, Axis, ConnectionBackend, LogEntry, LogLevel, RememberDuration, ThemeMode,
    Workspace,
};
use crate::fonts::FontCatalog;
use crate::rdp::RdpManager;
use crate::security::{initialize_or_unlock, VaultOpen};
use crate::ssh::SshManager;
use crate::storage::SecurityRecord;
use crate::theme::{apply_ui_font_size, install_theme};

use super::persistence::apply_settings_map;
use super::{AppLock, ConsoleTab, FontCache, ShellCoreApp, SidebarTab};

impl ShellCoreApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: AppConfig) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let fonts = FontCatalog::load();
        let mut settings = AppSettings::default();
        let mut storage_error = config.warning.clone();
        let mut persistence = None;

        if let Err(error) = config.ensure_directories() {
            storage_error = Some(format!("Could not create RShell config directory: {error}"));
        } else {
            match crate::storage::Persistence::open(&config.database_path) {
                Ok(storage) => {
                    if let Ok(values) = storage.load_settings() {
                        apply_settings_map(&mut settings, &values);
                    }
                    persistence = Some(storage);
                }
                Err(error) => storage_error = Some(error),
            }
        }

        let known_hosts = persistence
            .as_ref()
            .and_then(|storage| storage.load_known_host_keys().ok())
            .unwrap_or_default();

        if settings.ui_font.is_empty() {
            settings.ui_font = fonts.default_ui_font();
        }
        if settings.terminal_font.is_empty() {
            settings.terminal_font = fonts.default_mono_font();
        }
        let _ = fonts.apply(&cc.egui_ctx, &settings.ui_font, &settings.terminal_font);
        install_theme(&cc.egui_ctx, settings.theme);
        apply_ui_font_size(&cc.egui_ctx, settings.ui_font_size);

        let mut app = Self {
            config,
            storage: None,
            storage_error,
            security_record: SecurityRecord::default(),
            cipher: None,
            lock_state: AppLock::Unlocked,
            applied_fonts: FontCache {
                ui_font: settings.ui_font.clone(),
                terminal_font: settings.terminal_font.clone(),
            },
            settings,
            fonts,
            runtime_theme: ThemeMode::Dark,
            hosts: Vec::new(),
            active_host_id: None,
            active_tree_item: None,
            tree_rename: None,
            connected_hosts: HashSet::new(),
            rdp_compat_retried: HashSet::new(),
            workspace: Workspace::new(),
            next_tab_id: 1,
            next_aux_session_id: 1_000_000,
            sidebar_tab: SidebarTab::Hosts,
            console_tab: ConsoleTab::Logs,
            host_search: String::new(),
            host_search_open: false,
            logs: Vec::new(),
            log_selection: None,
            transfers: Vec::new(),
            tunnels: Vec::new(),
            next_transfer_id: 1,
            ssh: SshManager::new(known_hosts),
            rdp: RdpManager::new(),
            broadcast: false,
            toast: None,
            last_tick: Instant::now(),
            copy_feedback: None,
            config_copy_feedback: None,
            drag: None,
            drop_target: None,
            pending_split: None,
            pending_focus_tab: None,
            rdp_fullscreen_tab: None,
            maximized_pane: None,
            host_dialog: None,
            folder_dialog: None,
            folder_delete: None,
            host_delete: None,
            command_history: None,
            pending_command_history: Vec::new(),
            settings_open: false,
            keyring_open: false,
            snippets_open: false,
            about_open: false,
            command_palette_open: false,
            command_palette_query: String::new(),
            editor_dialog: None,
            chmod_dialog: None,
            md5_dialog: None,
            unlock_password: String::new(),
            unlock_remember: RememberDuration::None,
            unlock_error: None,
            reset_confirmation: false,
            security_current: String::new(),
            security_new: String::new(),
            security_confirm: String::new(),
            security_remember: RememberDuration::None,
            security_notice: None,
            saved_keys: Vec::new(),
            key_name: String::new(),
            key_file_path: String::new(),
            keyring_error: None,
            font_filter_ui: String::new(),
            font_filter_mono: String::new(),
            window_maximized: false,
        };

        if let Some(mut storage) = persistence {
            match storage.load_security() {
                Ok(mut record) => {
                    match initialize_or_unlock(&mut record, &storage.path) {
                        Ok(VaultOpen::Unlocked(cipher)) => {
                            app.cipher = Some(cipher.clone());
                            app.lock_state = AppLock::Unlocked;
                            match storage.load_host_tree(&cipher) {
                                Ok(hosts) => {
                                    app.hosts = hosts;
                                    app.saved_keys = storage.list_saved_keys().unwrap_or_default();
                                }
                                Err(error) => app.storage_error = Some(error),
                            }
                        }
                        Ok(VaultOpen::Locked) => app.lock_state = AppLock::Locked,
                        Ok(VaultOpen::CredentialsLost) => app.lock_state = AppLock::CredentialsLost,
                        Err(error) => {
                            app.storage_error = Some(error);
                            app.lock_state = AppLock::Unlocked;
                        }
                    }
                    let _ = storage.save_security(&record);
                    app.security_record = record;
                }
                Err(error) => app.storage_error = Some(error),
            }
            app.storage = Some(storage);
        }

        app.runtime_theme = app.settings.theme;
        app.logs.push(LogEntry::now(
            LogLevel::Info,
            format!(
                "RShell started. Database: {}",
                app.config.database_path.display()
            ),
        ));
        if let Some(warning) = &app.storage_error {
            app.logs
                .push(LogEntry::now(LogLevel::Error, warning.clone()));
        }
        if cfg!(debug_assertions) {
            if let Ok(scene) = std::env::var("RSHELL_DEBUG_SCENE") {
                app.apply_debug_scene(&scene);
            }
        }
        app
    }

    fn apply_debug_scene(&mut self, scene: &str) {
        let first_host_id = self
            .hosts
            .iter()
            .flat_map(|folder| folder.hosts.iter())
            .next()
            .map(|host| host.id);
        let host_id = if scene == "rdp" {
            self.hosts
                .iter()
                .flat_map(|folder| folder.hosts.iter())
                .find(|host| host.backend == ConnectionBackend::Rdp && host.ip != "127.0.0.1")
                .or_else(|| {
                    self.hosts
                        .iter()
                        .flat_map(|folder| folder.hosts.iter())
                        .find(|host| host.backend == ConnectionBackend::Rdp)
                })
                .map(|host| host.id)
                .or(first_host_id)
        } else {
            first_host_id
        };
        let Some(host_id) = host_id else {
            return;
        };
        self.active_host_id = Some(host_id);
        match scene {
            "terminal" => self.new_terminal_tab_for_host(host_id),
            "sftp" => {
                self.new_terminal_tab_for_host(host_id);
                self.new_sftp_tab_for_host(host_id);
            }
            "split" => {
                self.new_terminal_tab_for_host(host_id);
                self.split_active(Axis::Horizontal);
                self.new_sftp_tab_for_host(host_id);
            }
            "rdp" => self.new_rdp_tab_for_host(host_id),
            "dialog" => self.open_new_session(),
            "settings" => self.settings_open = true,
            "keyring" => self.keyring_open = true,
            _ => {}
        }
    }
}
