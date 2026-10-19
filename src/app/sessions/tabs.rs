use crate::app::{DropTarget, HostDraft, ShellCoreApp, TabDrag};
use crate::domain::{find_host, Axis, ConnectionBackend, LogLevel, SessionTab, TabKind, Workspace};

impl ShellCoreApp {
    pub fn open_new_session(&mut self) {
        let draft = HostDraft {
            folder_id: self.hosts.first().map(|folder| folder.id),
            key_id: self.saved_keys.first().map(|key| key.id),
            title: self.t("new_session_title").to_owned(),
            ..Default::default()
        };
        self.host_dialog = Some(draft);
    }

    pub fn open_quick_connect(&mut self) {
        let draft = HostDraft {
            title: self.t("quick_connect").to_owned(),
            folder_id: self.hosts.first().map(|folder| folder.id),
            key_id: self.saved_keys.first().map(|key| key.id),
            quick: true,
            connect_after_save: true,
            ..Default::default()
        };
        self.host_dialog = Some(draft);
    }

    pub fn new_terminal_tab(&mut self) {
        let Some(host) = self.active_host_clone() else {
            self.open_new_session();
            return;
        };
        self.new_terminal_tab_for_host(host.id);
    }

    pub fn open_or_focus_host(&mut self, host_id: u64) {
        let Some(host) = find_host(&self.hosts, host_id).cloned() else {
            return;
        };
        let kind = match host.backend {
            ConnectionBackend::Rdp => TabKind::Rdp,
            ConnectionBackend::Ssh | ConnectionBackend::LocalWsl => TabKind::Terminal,
        };
        let active_matches = self
            .workspace
            .active_tab()
            .is_some_and(|tab| tab.host_id == host_id && tab.kind == kind);
        if active_matches {
            self.active_host_id = Some(host_id);
            self.new_terminal_tab_for_host(host_id);
            return;
        }
        if let Some(tab_id) = latest_host_tab(&self.workspace, host_id, kind) {
            self.activate_tab(tab_id);
        } else {
            self.active_host_id = Some(host_id);
            self.new_terminal_tab_for_host(host_id);
        }
    }

    pub fn new_sftp_tab(&mut self) {
        let Some(host) = self.active_host_clone() else {
            self.open_new_session();
            return;
        };
        if host.backend != ConnectionBackend::Ssh {
            let message = if host.backend == ConnectionBackend::Rdp {
                self.t("rdp_sftp_unavailable")
            } else {
                self.t("wsl_sftp_unavailable")
            };
            self.toast(message);
            return;
        }
        if let Some(pane) = self.workspace.focused() {
            if let Some(existing) = pane
                .tabs
                .iter()
                .find(|tab| tab.kind == TabKind::Sftp && tab.host_id == host.id)
            {
                let id = existing.id;
                self.activate_tab(id);
                return;
            }
        }
        let id = self.allocate_tab_id();
        let mut tab = SessionTab::sftp(id, &host);
        tab.remote_cwd.clear();
        tab.local_cwd = crate::local_fs::display_path(&crate::local_fs::home_dir());
        refresh_local_directory(&mut tab);
        tab.remote_loading = true;
        tab.remote_request_path = tab.remote_cwd.clone();
        self.add_tab_to_focused(tab);
        self.start_tab_session(id);
        self.log(
            LogLevel::Info,
            format!("SFTP channel opened for {}", host.address()),
        );
    }

    pub fn open_or_focus_sftp(&mut self) {
        self.new_sftp_tab();
    }

    fn add_tab_to_focused(&mut self, tab: SessionTab) {
        if self.workspace.focused_mut().is_none() {
            self.workspace = Workspace::new();
        }
        if let Some(pane) = self.workspace.focused_mut() {
            pane.add_tab(tab);
        }
    }

    pub fn allocate_tab_id(&mut self) -> u64 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        id
    }

    pub fn allocate_aux_session_id(&mut self) -> u64 {
        let id = self.next_aux_session_id;
        self.next_aux_session_id += 1;
        id
    }

    pub fn close_tab(&mut self, tab_id: u64) {
        self.rdp.close_tab(tab_id);
        self.rdp_compat_retried.remove(&tab_id);
        self.ssh.close_tab(tab_id);
        if self
            .command_history
            .as_ref()
            .is_some_and(|dialog| dialog.tab_id == tab_id)
        {
            self.command_history = None;
        }
        if self.rdp_fullscreen_tab == Some(tab_id) {
            self.rdp_fullscreen_tab = None;
        }
        if self.pending_focus_tab == Some(tab_id) {
            self.pending_focus_tab = None;
        }
        if let Some(tab) = self.workspace.remove_tab(tab_id) {
            if let Some(source_session_id) = tab.left_session_id {
                self.ssh.close_tab(source_session_id);
            }
            self.log(LogLevel::Info, format!("Closed tab: {}", tab.title));
        }
    }

    pub fn close_focused_tab(&mut self) {
        if let Some(tab) = self.workspace.active_tab() {
            let id = tab.id;
            self.close_tab(id);
        }
    }

    pub fn split_active(&mut self, axis: Axis) {
        let duplicate = self
            .workspace
            .active_tab()
            .filter(|tab| tab.kind == TabKind::Terminal)
            .cloned()
            .and_then(|tab| {
                let host = find_host(&self.hosts, tab.host_id)?;
                let id = self.next_tab_id;
                self.next_tab_id += 1;
                let mut fresh = SessionTab::terminal(id, host);
                fresh.title = tab.title;
                self.pending_focus_tab = Some(id);
                Some(fresh)
            });
        let new_tab_id = duplicate.as_ref().map(|tab| tab.id);
        self.workspace.split_active(axis, duplicate);
        self.maximized_pane = None;
        if let Some(tab_id) = new_tab_id {
            self.start_tab_session(tab_id);
        }
    }

    pub fn move_tab(&mut self, drag: &TabDrag, target: DropTarget) {
        self.workspace
            .move_tab(drag.tab_id, target.pane_id, target.zone);
    }

    pub fn activate_tab(&mut self, tab_id: u64) {
        if let Some(pane_id) = self.workspace.pane_for_tab(tab_id) {
            self.workspace.focused_pane = pane_id;
            if let Some(host_id) = self.workspace.find_tab(tab_id).map(|tab| tab.host_id) {
                self.active_host_id = Some(host_id);
            }
            if let Some(pane) = self.workspace.pane_mut(pane_id) {
                if let Some(index) = pane.tabs.iter().position(|tab| tab.id == tab_id) {
                    pane.active = index;
                }
            }
        }
    }

    pub fn new_terminal_tab_for_host(&mut self, host_id: u64) {
        if find_host(&self.hosts, host_id).is_none() {
            return;
        }
        self.active_host_id = Some(host_id);
        if find_host(&self.hosts, host_id)
            .is_some_and(|host| host.backend == ConnectionBackend::Rdp)
        {
            self.new_rdp_tab_for_host(host_id);
            return;
        }
        let id = self.allocate_tab_id();
        let Some(host) = find_host(&self.hosts, host_id) else {
            return;
        };
        let tab = SessionTab::terminal(id, host);
        self.add_tab_to_focused(tab);
        self.pending_focus_tab = Some(id);
        self.start_tab_session(id);
    }

    pub fn set_sftp_left_source(&mut self, tab: &mut SessionTab, source_host_id: Option<u64>) {
        if let Some(session_id) = tab.left_session_id.take() {
            self.ssh.close_tab(session_id);
        }
        tab.left_host_id = None;
        tab.left_loading = false;
        tab.local_error = None;
        tab.local_entries.clear();
        tab.local_selected.clear();
        tab.local_anchor = None;
        tab.left_request_path.clear();

        let Some(host_id) = source_host_id else {
            tab.local_cwd = crate::local_fs::display_path(&crate::local_fs::home_dir());
            tab.local_path_input = tab.local_cwd.clone();
            refresh_local_directory(tab);
            return;
        };

        let Some(host) = find_host(&self.hosts, host_id).cloned() else {
            self.toast(self.t("select_file_first"));
            return;
        };
        if host.backend != ConnectionBackend::Ssh {
            self.toast(self.t("sftp_ssh_only"));
            tab.local_cwd = crate::local_fs::display_path(&crate::local_fs::home_dir());
            tab.local_path_input = tab.local_cwd.clone();
            refresh_local_directory(tab);
            return;
        }
        let auth = match self.resolve_ssh_auth(&host) {
            Ok(auth) => auth,
            Err(error) => {
                tab.local_error = Some(error.clone());
                self.log(LogLevel::Error, error);
                return;
            }
        };

        let session_id = self.allocate_aux_session_id();
        tab.left_host_id = Some(host.id);
        tab.left_session_id = Some(session_id);
        tab.local_cwd = "~".to_owned();
        tab.local_path_input = "~".to_owned();
        tab.left_request_path = "~".to_owned();
        tab.left_loading = true;
        self.log(
            LogLevel::Info,
            format!("Opening SFTP source connection to {}...", host.address()),
        );
        if let Err(error) = self.ssh.open_sftp(session_id, host.clone(), auth) {
            tab.left_host_id = None;
            tab.left_session_id = None;
            tab.left_loading = false;
            tab.local_error = Some(error.clone());
            self.log(LogLevel::Error, error);
        }
    }

    pub fn set_sftp_right_host(&mut self, tab: &mut SessionTab, host_id: u64) {
        if tab.host_id == host_id {
            return;
        }
        let Some(host) = find_host(&self.hosts, host_id).cloned() else {
            return;
        };
        if host.backend != ConnectionBackend::Ssh {
            self.toast(self.t("sftp_ssh_only"));
            return;
        }
        let auth = match self.resolve_ssh_auth(&host) {
            Ok(auth) => auth,
            Err(error) => {
                tab.remote_error = Some(error.clone());
                self.log(LogLevel::Error, error);
                return;
            }
        };
        self.ssh.close_tab(tab.id);
        tab.host_id = host.id;
        tab.title = host.name.clone();
        tab.remote_cwd = "~".to_owned();
        tab.remote_path_input = "~".to_owned();
        tab.remote_request_path = "~".to_owned();
        tab.remote_entries.clear();
        tab.remote_selected.clear();
        tab.remote_anchor = None;
        tab.remote_loading = true;
        tab.remote_error = None;
        self.active_host_id = Some(host.id);
        self.log(
            LogLevel::Info,
            format!(
                "Opening SFTP destination connection to {}...",
                host.address()
            ),
        );
        if let Err(error) = self.ssh.open_sftp(tab.id, host.clone(), auth) {
            tab.remote_loading = false;
            tab.remote_error = Some(error.clone());
            self.log(LogLevel::Error, error);
        }
    }

    pub fn new_rdp_tab_for_host(&mut self, host_id: u64) {
        if find_host(&self.hosts, host_id).is_none() {
            return;
        }
        self.active_host_id = Some(host_id);
        let id = self.allocate_tab_id();
        let Some(host) = find_host(&self.hosts, host_id) else {
            return;
        };
        let tab = SessionTab::rdp(id, host);
        self.add_tab_to_focused(tab);
        self.pending_focus_tab = Some(id);
        self.start_tab_session(id);
    }

    pub fn new_sftp_tab_for_host(&mut self, host_id: u64) {
        self.active_host_id = Some(host_id);
        self.new_sftp_tab();
    }

    pub(super) fn host_tab_ids(&self, host_id: u64) -> Vec<u64> {
        fn visit(node: &crate::domain::SplitNode, host_id: u64, output: &mut Vec<u64>) {
            match node {
                crate::domain::SplitNode::Leaf(pane) => {
                    output.extend(
                        pane.tabs
                            .iter()
                            .filter(|tab| tab.host_id == host_id)
                            .map(|tab| tab.id),
                    );
                }
                crate::domain::SplitNode::Split { first, second, .. } => {
                    visit(first, host_id, output);
                    visit(second, host_id, output);
                }
            }
        }
        let mut output = Vec::new();
        visit(&self.workspace.root, host_id, &mut output);
        output
    }
}

fn latest_host_tab(workspace: &Workspace, host_id: u64, kind: TabKind) -> Option<u64> {
    workspace
        .all_tabs()
        .into_iter()
        .filter(|tab| tab.host_id == host_id && tab.kind == kind)
        .max_by_key(|tab| tab.id)
        .map(|tab| tab.id)
}

pub fn refresh_local_directory(tab: &mut SessionTab) {
    match crate::local_fs::list(&tab.local_cwd) {
        Ok((path, entries)) => {
            tab.local_cwd = path;
            tab.local_path_input = tab.local_cwd.clone();
            tab.local_entries = entries;
            tab.local_error = None;
        }
        Err(error) => {
            tab.local_entries.clear();
            tab.local_error = Some(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AuthKind, Host, HostOs, HostStatus, KeySource};

    fn host(id: u64, backend: ConnectionBackend, ip: &str) -> Host {
        Host {
            id,
            folder_id: 1,
            name: format!("host-{id}"),
            ip: ip.to_owned(),
            port: if backend == ConnectionBackend::Rdp {
                3389
            } else {
                22
            },
            user: "user".to_owned(),
            auth: AuthKind::Password,
            key_source: KeySource::FileSystem,
            key_id: None,
            secret: "password".to_owned(),
            gateway: "direct".to_owned(),
            environment: "TEST".to_owned(),
            keep_alive: 30,
            backend,
            os: if backend == ConnectionBackend::Rdp {
                HostOs::Unknown
            } else {
                HostOs::Linux
            },
            wsl_distro: None,
            status: HostStatus::Offline,
            latency_ms: None,
            rdp_mode: Default::default(),
            credentials_unavailable: false,
        }
    }

    #[test]
    fn latest_host_tab_matches_host_and_protocol() {
        let ssh = host(1, ConnectionBackend::Ssh, "10.0.0.1");
        let rdp = host(2, ConnectionBackend::Rdp, "10.0.0.1");
        let mut workspace = Workspace::new();
        let pane = workspace.focused_mut().unwrap();
        pane.add_tab(SessionTab::terminal(10, &ssh));
        pane.add_tab(SessionTab::sftp(20, &ssh));
        pane.add_tab(SessionTab::rdp(30, &rdp));
        pane.add_tab(SessionTab::terminal(40, &ssh));

        assert_eq!(latest_host_tab(&workspace, 1, TabKind::Terminal), Some(40));
        assert_eq!(latest_host_tab(&workspace, 1, TabKind::Sftp), Some(20));
        assert_eq!(latest_host_tab(&workspace, 2, TabKind::Rdp), Some(30));
        assert_eq!(latest_host_tab(&workspace, 2, TabKind::Terminal), None);
    }
}
