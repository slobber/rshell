use eframe::egui::{self, ViewportCommand};

use crate::domain::{
    find_host_mut, ConnectionBackend, HostStatus, LogLevel, TabKind, TransferDestination,
    TransferStatus,
};
use crate::ssh::SshEvent;

use super::ShellCoreApp;

impl ShellCoreApp {
    pub(super) fn poll_ssh_events(&mut self, ctx: &egui::Context) {
        let events = self.ssh.drain_events();
        if events.is_empty() {
            return;
        }

        for event in events {
            match event {
                SshEvent::HostKey {
                    host_key,
                    fingerprint,
                    trusted,
                    mismatch,
                    ..
                } => {
                    if mismatch {
                        self.log(
                            LogLevel::Error,
                            format!(
                                "SSH host key mismatch for {host_key}; connection rejected ({fingerprint})."
                            ),
                        );
                    } else if !trusted {
                        if let Some(storage) = &mut self.storage {
                            if let Err(error) = storage.save_known_host_key(&host_key, &fingerprint)
                            {
                                self.log(LogLevel::Error, error);
                            }
                        }
                        self.ssh
                            .trust_host_key(host_key.clone(), fingerprint.clone());
                        self.log(
                            LogLevel::Warning,
                            format!("Trusted new SSH host key for {host_key}: {fingerprint}"),
                        );
                    } else {
                        self.log(
                            LogLevel::Info,
                            format!("SSH host key verified for {host_key}: {fingerprint}"),
                        );
                    }
                }
                SshEvent::Connected {
                    host_id,
                    latency_ms,
                    ..
                } => {
                    self.connected_hosts.insert(host_id);
                    let host_info = self
                        .hosts
                        .iter()
                        .flat_map(flatten_visible_hosts)
                        .find(|host| host.id == host_id)
                        .map(|host| (host.address(), host.backend));
                    let address = host_info
                        .as_ref()
                        .map(|(address, _)| address.clone())
                        .unwrap_or_else(|| format!("host #{host_id}"));
                    if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
                        host.status = HostStatus::Online;
                        host.latency_ms = Some(latency_ms);
                    }
                    let message = match host_info.map(|(_, backend)| backend) {
                        Some(ConnectionBackend::LocalWsl) => {
                            format!("WSL shell started: {address} in {latency_ms} ms.")
                        }
                        _ => format!("SSH authenticated to {address} in {latency_ms} ms."),
                    };
                    self.log(LogLevel::Success, message);
                }
                SshEvent::ConnectFailed {
                    tab_id,
                    host_id,
                    message,
                } => {
                    self.ssh.forget_session(tab_id);
                    if let Some((tab, left)) = self.workspace.find_tab_for_sftp_session_mut(tab_id)
                    {
                        if left {
                            tab.left_loading = false;
                            tab.left_session_id = None;
                            tab.local_error = Some(message.clone());
                        }
                    }
                    let other_sessions = self.ssh.session_count_for_host(host_id) > 0;
                    if !other_sessions {
                        self.connected_hosts.remove(&host_id);
                    }
                    let address = self
                        .hosts
                        .iter()
                        .flat_map(flatten_visible_hosts)
                        .find(|host| host.id == host_id)
                        .map(|host| host.address())
                        .unwrap_or_else(|| format!("host #{host_id}"));
                    if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
                        host.status = if other_sessions {
                            HostStatus::Online
                        } else {
                            HostStatus::Error
                        };
                        host.latency_ms = None;
                    }
                    self.log(
                        LogLevel::Error,
                        format!("Connection to {address} failed: {message}"),
                    );
                    if let Some(tab) = self.workspace.find_tab_mut(tab_id) {
                        if tab.kind == TabKind::Terminal {
                            tab.terminal
                                .write_text(&format!("\r\nConnection failed: {message}\r\n"));
                        }
                    }
                    self.toast(format!("SSH connection failed: {message}"));
                }
                SshEvent::OsDetected { host_id, os } => {
                    if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
                        if host.os != os {
                            host.os = os;
                            self.persist_hosts();
                        }
                    }
                }
                SshEvent::TerminalData { tab_id, data } => {
                    let mut cursor_report = None;
                    if let Some(tab) = self.workspace.find_tab_mut(tab_id) {
                        if tab.kind == TabKind::Terminal {
                            tab.terminal.process(&data);
                            tab.scroll_to_bottom = true;
                            if requests_cursor_position(&data) {
                                let (row, col) = tab
                                    .terminal
                                    .with_screen(|screen| screen.cursor_position())
                                    .unwrap_or((0, 0));
                                cursor_report =
                                    Some(format!("\x1b[{};{}R", row + 1, col + 1).into_bytes());
                            }
                        }
                    }
                    if let Some(report) = cursor_report {
                        self.ssh.send_input(tab_id, report);
                    }
                }
                SshEvent::TerminalClosed {
                    tab_id,
                    host_id,
                    message,
                } => {
                    let session_was_active = self.ssh.has_session(tab_id);
                    self.ssh.forget_session(tab_id);
                    if session_was_active {
                        if let Some(tab) = self.workspace.find_tab_mut(tab_id) {
                            tab.terminal
                                .write_text(&format!("\r\n\x1b[33m[SSH] {message}\x1b[0m\r\n"));
                        }
                    }
                    let name = self
                        .hosts
                        .iter()
                        .flat_map(flatten_visible_hosts)
                        .find(|host| host.id == host_id)
                        .map(|host| host.name.clone())
                        .unwrap_or_else(|| format!("host #{host_id}"));
                    self.log(LogLevel::Warning, format!("{name}: {message}"));
                    let other_sessions = self.ssh.session_count_for_host(host_id) > 0;
                    if !other_sessions {
                        self.connected_hosts.remove(&host_id);
                        if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
                            host.status = HostStatus::Offline;
                            host.latency_ms = None;
                        }
                    }
                }
                SshEvent::SftpHome { tab_id, path } => {
                    if let Some((tab, left)) = self.workspace.find_tab_for_sftp_session_mut(tab_id)
                    {
                        if left {
                            tab.local_cwd = path.clone();
                            tab.local_path_input = path.clone();
                            tab.left_request_path = path;
                            tab.left_loading = true;
                        } else {
                            tab.remote_cwd = path.clone();
                            tab.remote_path_input = path.clone();
                            tab.remote_request_path = path;
                            tab.remote_loading = true;
                        }
                    }
                }
                SshEvent::RemoteEntries {
                    tab_id,
                    path,
                    mut entries,
                } => {
                    entries.sort_by(|left, right| {
                        right
                            .is_dir
                            .cmp(&left.is_dir)
                            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                    });
                    if let Some((tab, left)) = self.workspace.find_tab_for_sftp_session_mut(tab_id)
                    {
                        if left {
                            tab.local_cwd = path.clone();
                            tab.local_path_input = path;
                            tab.local_entries = entries;
                            tab.left_loading = false;
                            tab.local_error = None;
                            tab.local_selected.clear();
                            tab.local_anchor = None;
                        } else {
                            tab.remote_cwd = path.clone();
                            tab.remote_path_input = path;
                            tab.remote_entries = entries;
                            tab.remote_loading = false;
                            tab.remote_error = None;
                            tab.remote_selected.clear();
                            tab.remote_anchor = None;
                        }
                    }
                }
                SshEvent::RemoteError {
                    tab_id,
                    path,
                    message,
                } => {
                    if let Some((tab, left)) = self.workspace.find_tab_for_sftp_session_mut(tab_id)
                    {
                        if left {
                            tab.left_loading = false;
                            tab.local_error = Some(message.clone());
                            tab.left_request_path = path;
                        } else {
                            tab.remote_loading = false;
                            tab.remote_error = Some(message.clone());
                            tab.remote_request_path = path;
                        }
                    }
                    self.log(LogLevel::Error, message);
                }
                SshEvent::SftpChanged {
                    tab_id,
                    directory,
                    message,
                } => {
                    let mut refresh = None;
                    if let Some((tab, left)) = self.workspace.find_tab_for_sftp_session_mut(tab_id)
                    {
                        if left {
                            if let Some(session_id) = tab.left_session_id {
                                tab.left_loading = true;
                                tab.local_error = None;
                                tab.left_request_path = directory.clone();
                                tab.local_selected.clear();
                                tab.local_anchor = None;
                                refresh = Some((session_id, directory));
                            }
                        } else {
                            tab.remote_loading = true;
                            tab.remote_error = None;
                            tab.remote_request_path = directory.clone();
                            tab.remote_selected.clear();
                            tab.remote_anchor = None;
                            refresh = Some((tab.id, directory));
                        }
                    }
                    self.log(LogLevel::Success, message.clone());
                    self.toast(message);
                    if let Some((session_id, path)) = refresh {
                        self.ssh.list_remote(session_id, path);
                    }
                }
                SshEvent::FileContent {
                    tab_id,
                    path,
                    content,
                    truncated,
                } => {
                    if let Some(dialog) = &mut self.editor_dialog {
                        if dialog.tab_id == Some(tab_id) && dialog.path == path {
                            dialog.content = content;
                            dialog.truncated = truncated;
                        }
                    }
                }
                SshEvent::FileSaved { path, .. } => {
                    self.log(LogLevel::Success, format!("Saved remote file {path}."));
                    self.toast(self.t("saved"));
                }
                SshEvent::PermissionsChanged { tab_id, path, mode } => {
                    self.log(LogLevel::Success, format!("chmod {mode} applied to {path}"));
                    let mut refresh = None;
                    if let Some((tab, left)) = self.workspace.find_tab_for_sftp_session_mut(tab_id)
                    {
                        refresh = Some(if left {
                            tab.local_cwd.clone()
                        } else {
                            tab.remote_cwd.clone()
                        });
                    }
                    if let Some(path) = refresh {
                        self.ssh.list_remote(tab_id, path);
                    }
                }
                SshEvent::HashResult { path, hashes, .. } => {
                    if let Some(dialog) = &mut self.hash_dialog {
                        if dialog.path == path {
                            dialog.hashes = Some(hashes);
                        }
                    }
                }
                SshEvent::TransferProgress {
                    transfer_id,
                    transferred_bytes,
                    total_bytes,
                    speed_bps,
                } => {
                    if let Some(transfer) = self
                        .transfers
                        .iter_mut()
                        .find(|transfer| transfer.id == transfer_id)
                    {
                        transfer.status = TransferStatus::Active;
                        transfer.transferred_bytes = transferred_bytes;
                        transfer.total_bytes = total_bytes.max(1.0);
                        transfer.speed_bps = speed_bps;
                    }
                }
                SshEvent::TransferDone {
                    transfer_id,
                    total_bytes,
                } => {
                    let mut completed = None;
                    let mut refresh = None;
                    if let Some(transfer) = self
                        .transfers
                        .iter_mut()
                        .find(|transfer| transfer.id == transfer_id)
                    {
                        transfer.status = TransferStatus::Done;
                        transfer.transferred_bytes = total_bytes.max(transfer.transferred_bytes);
                        transfer.total_bytes = total_bytes.max(1.0);
                        transfer.speed_bps = 0.0;
                        transfer.logged_done = true;
                        completed = Some(transfer.name.clone());
                        refresh = Some((transfer.tab_id, transfer.destination_side));
                    }
                    if let Some(name) = completed {
                        self.log(LogLevel::Success, format!("[SFTP] '{name}' completed."));
                        if self.settings.taskbar_attention {
                            ctx.send_viewport_cmd(ViewportCommand::RequestUserAttention(
                                egui::UserAttentionType::Informational,
                            ));
                        }
                    }
                    if let Some((tab_id, destination)) = refresh {
                        match destination {
                            TransferDestination::Local => {
                                if let Some(tab) = self.workspace.find_tab_mut(tab_id) {
                                    crate::app::sessions::refresh_local_directory(tab);
                                }
                            }
                            TransferDestination::LeftRemote => {
                                let request = self.workspace.find_tab_mut(tab_id).and_then(|tab| {
                                    tab.left_session_id.map(|session_id| {
                                        tab.left_loading = true;
                                        tab.local_error = None;
                                        (session_id, tab.local_cwd.clone())
                                    })
                                });
                                if let Some((session_id, path)) = request {
                                    self.ssh.list_remote(session_id, path);
                                }
                            }
                            TransferDestination::RightRemote => {
                                let request = self.workspace.find_tab_mut(tab_id).map(|tab| {
                                    tab.remote_loading = true;
                                    tab.remote_error = None;
                                    (tab.id, tab.remote_cwd.clone())
                                });
                                if let Some((session_id, path)) = request {
                                    self.ssh.list_remote(session_id, path);
                                }
                            }
                        }
                    }
                }
                SshEvent::TransferFailed {
                    transfer_id,
                    message,
                } => {
                    let mut failed_name = None;
                    if let Some(transfer) = self
                        .transfers
                        .iter_mut()
                        .find(|transfer| transfer.id == transfer_id)
                    {
                        transfer.status = TransferStatus::Failed;
                        transfer.speed_bps = 0.0;
                        failed_name = Some(transfer.name.clone());
                    }
                    if let Some(name) = failed_name {
                        self.log(LogLevel::Error, format!("[SFTP] {name} failed: {message}"));
                    } else {
                        self.log(LogLevel::Error, message);
                    }
                }
                SshEvent::Notice { level, message, .. } => self.log(level, message),
            }
        }
        ctx.request_repaint();
    }
}

fn flatten_visible_hosts(folder: &crate::domain::HostFolder) -> Vec<&crate::domain::Host> {
    let mut hosts = folder.hosts.iter().collect::<Vec<_>>();
    for child in &folder.children {
        hosts.extend(flatten_visible_hosts(child));
    }
    hosts
}

fn requests_cursor_position(data: &[u8]) -> bool {
    data.windows(4).any(|window| window == b"\x1b[6n")
}
