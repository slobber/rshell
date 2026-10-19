use eframe::egui;

use crate::domain::{find_host, find_host_mut, HostStatus, LogLevel, RdpConnectionMode};
use crate::rdp::RdpEvent;

use super::ShellCoreApp;

impl ShellCoreApp {
    pub(super) fn poll_rdp_events(&mut self, ctx: &egui::Context) {
        let events = self.rdp.drain_events();
        if events.is_empty() {
            return;
        }

        for event in events {
            match event {
                RdpEvent::Connected { tab_id, host_id } => {
                    self.connected_hosts.insert(host_id);
                    if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
                        host.status = HostStatus::Online;
                        host.latency_ms = Some(0);
                    }
                    let name = self
                        .hosts
                        .iter()
                        .flat_map(flatten_visible_hosts)
                        .find(|host| host.id == host_id)
                        .map(|host| host.address())
                        .unwrap_or_else(|| format!("host #{host_id}"));
                    if let Some(mode) = self.rdp.connection_mode(tab_id) {
                        let mut changed = false;
                        if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
                            if host.rdp_mode != mode {
                                host.rdp_mode = mode;
                                changed = true;
                            }
                        }
                        if changed {
                            self.persist_hosts();
                            self.log(
                                LogLevel::Info,
                                format!("RDP mode saved for {name}: {}.", mode.as_str()),
                            );
                        }
                    }
                    self.log(LogLevel::Success, format!("RDP connected to {name}."));
                    let _ = tab_id;
                }
                RdpEvent::Frame { tab_id, frame } => {
                    self.rdp.store_frame(tab_id, frame);
                }
                RdpEvent::Failed {
                    tab_id,
                    host_id,
                    message,
                } => {
                    if self.retry_rdp_compat(tab_id, host_id, &message) {
                        continue;
                    }
                    self.rdp.set_error(tab_id, message.clone());
                    if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
                        host.status = HostStatus::Error;
                        host.latency_ms = None;
                    }
                    self.log(LogLevel::Error, message);
                }
                RdpEvent::Closed {
                    tab_id,
                    host_id,
                    message,
                } => {
                    if self.retry_rdp_compat(tab_id, host_id, &message) {
                        continue;
                    }
                    self.rdp.forget_session(tab_id);
                    if self.rdp.session_count_for_host(host_id) == 0
                        && self.ssh.session_count_for_host(host_id) == 0
                    {
                        self.connected_hosts.remove(&host_id);
                        if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
                            host.status = HostStatus::Offline;
                            host.latency_ms = None;
                        }
                    }
                    self.log(LogLevel::Warning, message);
                }
            }
        }
        ctx.request_repaint();
    }

    fn retry_rdp_compat(&mut self, tab_id: u64, host_id: u64, message: &str) -> bool {
        let Some(host) = find_host(&self.hosts, host_id).cloned() else {
            return false;
        };
        let used_mode = self.rdp.connection_mode(tab_id).unwrap_or(host.rdp_mode);
        if self.rdp_compat_retried.contains(&tab_id) || !should_retry_rdp_compat(used_mode, message)
        {
            return false;
        }
        let Some((width, height)) = self.rdp.last_size(tab_id) else {
            return false;
        };
        self.rdp_compat_retried.insert(tab_id);
        self.rdp.forget_session(tab_id);
        match self.rdp.open_compat(tab_id, &host, width, height) {
            Ok(()) => {
                self.log(
                    LogLevel::Info,
                    "RDP KRDP mode failed; retrying with Windows RDP compatibility mode."
                        .to_owned(),
                );
                true
            }
            Err(error) => {
                self.log(LogLevel::Error, error);
                false
            }
        }
    }
}

fn should_retry_rdp_compat(mode: RdpConnectionMode, message: &str) -> bool {
    if mode == RdpConnectionMode::Compatibility || is_rdp_auth_failure(message) {
        return false;
    }
    !message.contains("RDP session ended") || is_rdp_compat_symptom(message)
}

fn is_rdp_auth_failure(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("invalid username or password")
        || message.contains("logon failed")
        || message.contains("authentication")
}

fn is_rdp_compat_symptom(message: &str) -> bool {
    message.contains("rfx progressive decode failed")
        || message.contains("GraphicsPipelineClient")
        || message.contains("negotiation")
        || message.contains("capability")
        || message.contains("decode_send_data_indication")
        || message.contains("received disconnect provider ultimatum")
}

fn flatten_visible_hosts(folder: &crate::domain::HostFolder) -> Vec<&crate::domain::Host> {
    let mut hosts = folder.hosts.iter().collect::<Vec<_>>();
    for child in &folder.children {
        hosts.extend(flatten_visible_hosts(child));
    }
    hosts
}

#[cfg(test)]
mod tests {
    use super::{should_retry_rdp_compat, RdpConnectionMode};

    #[test]
    fn retries_krdp_failures_with_windows_rdp() {
        assert!(should_retry_rdp_compat(
            RdpConnectionMode::GraphicsPipeline,
            "RDP connection failed: negotiation failed"
        ));
    }

    #[test]
    fn does_not_retry_after_compatibility_mode_was_used() {
        assert!(!should_retry_rdp_compat(
            RdpConnectionMode::Compatibility,
            "RDP connection failed: negotiation failed"
        ));
    }

    #[test]
    fn does_not_retry_authentication_failures() {
        assert!(!should_retry_rdp_compat(
            RdpConnectionMode::GraphicsPipeline,
            "RDP logon failed: invalid username or password."
        ));
    }

    #[test]
    fn does_not_retry_graceful_disconnect() {
        assert!(!should_retry_rdp_compat(
            RdpConnectionMode::GraphicsPipeline,
            "RDP session ended: UserInitiated"
        ));
    }

    #[test]
    fn retries_egfx_failure_even_after_connect() {
        assert!(should_retry_rdp_compat(
            RdpConnectionMode::GraphicsPipeline,
            "RDP session ended: rfx progressive decode failed"
        ));
    }
}
