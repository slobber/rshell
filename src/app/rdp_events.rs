use eframe::egui;

use crate::domain::{find_host_mut, HostStatus, LogLevel};
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
}

fn flatten_visible_hosts(folder: &crate::domain::HostFolder) -> Vec<&crate::domain::Host> {
    let mut hosts = folder.hosts.iter().collect::<Vec<_>>();
    for child in &folder.children {
        hosts.extend(flatten_visible_hosts(child));
    }
    hosts
}
