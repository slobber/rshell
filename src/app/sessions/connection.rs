use crate::app::ShellCoreApp;
use crate::domain::{
    find_host, find_host_mut, ConnectionBackend, HostStatus, LogLevel, SessionTab, TabKind,
};

impl ShellCoreApp {
    pub fn ensure_connected(&mut self, host_id: u64) {
        if self.connected_hosts.contains(&host_id) {
            return;
        }
        let Some(current_status) = find_host(&self.hosts, host_id).map(|host| host.status) else {
            return;
        };
        if current_status == HostStatus::Connecting {
            return;
        }
        if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
            host.status = HostStatus::Connecting;
            host.latency_ms = None;
        }
        self.active_host_id = Some(host_id);
        if let Some(host) = find_host(&self.hosts, host_id) {
            self.log(
                LogLevel::Info,
                format!("Connecting to {}...", host.address()),
            );
        }
    }

    pub fn disconnect_host(&mut self, host_id: u64) {
        let Some(host) = find_host(&self.hosts, host_id).cloned() else {
            return;
        };
        let terminal_tabs = self.ssh.session_tab_ids_for_host(host_id);
        self.ssh.disconnect_host(host_id);
        self.rdp.disconnect_host(host_id);
        self.connected_hosts.remove(&host_id);
        if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
            host.status = HostStatus::Offline;
            host.latency_ms = None;
        }
        for tab_id in terminal_tabs {
            if let Some(tab) = self.workspace.find_tab_mut(tab_id) {
                if tab.kind == TabKind::Terminal {
                    tab.terminal.ensure_new_line();
                    tab.terminal.write_text("\x1b[33m[Disconnected]\x1b[0m\r\n");
                    tab.scroll_to_bottom = true;
                }
            }
        }
        self.log(
            LogLevel::Warning,
            format!("Disconnected from {}.", host.address()),
        );
        self.toast(format!("Disconnected {}", host.name));
    }

    pub fn reconnect_active(&mut self) {
        let Some(host) = self.active_host_clone() else {
            self.open_quick_connect();
            return;
        };
        self.ssh.disconnect_host(host.id);
        self.rdp.disconnect_host(host.id);
        self.connected_hosts.remove(&host.id);
        if let Some(host) = find_host_mut(&mut self.hosts, host.id) {
            // Let `ensure_connected` report the new attempt once instead of
            // marking the host as connecting before it can log the transition.
            host.status = HostStatus::Offline;
            host.latency_ms = None;
        }
        let tab_ids = self.host_tab_ids(host.id);
        for tab_id in tab_ids {
            self.start_tab_session(tab_id);
        }
    }

    pub fn start_tab_session(&mut self, tab_id: u64) {
        if let Some(tab) = self.workspace.find_tab_mut(tab_id) {
            if tab.kind == TabKind::Sftp {
                tab.remote_loading = true;
                tab.remote_error = None;
                tab.remote_request_path = tab.remote_cwd.clone();
            } else if tab.kind == TabKind::Terminal {
                // A reconnect must never append the new shell banner to the
                // previous prompt line.
                tab.terminal.ensure_new_line();
                tab.scroll_to_bottom = true;
            }
        }
        let Some(tab) = self.workspace.find_tab(tab_id).cloned() else {
            return;
        };
        self.start_session(&tab);
    }

    pub fn start_session(&mut self, tab: &SessionTab) {
        let Some(host) = find_host(&self.hosts, tab.host_id).cloned() else {
            return;
        };
        if host.backend == ConnectionBackend::Rdp {
            if tab.kind != TabKind::Rdp {
                self.toast(self.t("rdp_sftp_unavailable"));
                return;
            }
            self.ensure_connected(host.id);
            let desktop_width = crate::rdp::DEFAULT_DESKTOP_WIDTH;
            let desktop_height = crate::rdp::DEFAULT_DESKTOP_HEIGHT;
            let result = self.rdp.open(tab.id, &host, desktop_width, desktop_height);
            if let Err(error) = result {
                self.log(LogLevel::Error, error);
                if let Some(host) = find_host_mut(&mut self.hosts, host.id) {
                    host.status = HostStatus::Error;
                }
            }
            return;
        }
        if host.backend == ConnectionBackend::LocalWsl {
            if tab.kind != TabKind::Terminal {
                self.toast(self.t("wsl_sftp_unavailable"));
                return;
            }
            self.ensure_connected(host.id);
            let (rows, cols) = tab.terminal.size();
            let result = self.ssh.open_local_terminal(
                tab.id,
                host.clone(),
                u32::from(cols),
                u32::from(rows),
            );
            if let Err(error) = result {
                self.log(LogLevel::Error, error);
                if let Some(host) = find_host_mut(&mut self.hosts, host.id) {
                    host.status = HostStatus::Error;
                }
            }
            return;
        }
        let auth = match self.resolve_ssh_auth(&host) {
            Ok(auth) => auth,
            Err(error) => {
                if let Some(host) = find_host_mut(&mut self.hosts, tab.host_id) {
                    host.status = HostStatus::Error;
                }
                self.log(LogLevel::Error, error.clone());
                tab.terminal
                    .write_text(&format!("Connection failed: {error}\r\n"));
                return;
            }
        };
        self.ensure_connected(host.id);
        let result = match tab.kind {
            TabKind::Terminal => {
                let (rows, cols) = tab.terminal.size();
                self.ssh
                    .open_terminal(tab.id, host.clone(), auth, u32::from(cols), u32::from(rows))
            }
            TabKind::Sftp => self.ssh.open_sftp(tab.id, host.clone(), auth),
            TabKind::Rdp => return,
        };
        if let Err(error) = result {
            self.log(LogLevel::Error, error);
            if let Some(host) = find_host_mut(&mut self.hosts, host.id) {
                host.status = HostStatus::Error;
            }
        }
    }
}
