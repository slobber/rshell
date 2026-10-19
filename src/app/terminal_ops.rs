use crate::domain::{find_host, LogLevel, TabKind};

use super::ShellCoreApp;

impl ShellCoreApp {
    pub fn execute_terminal_command(&mut self, tab_id: u64, command: String) {
        self.send_terminal_command(tab_id, command);
    }

    pub fn send_terminal_command(&mut self, tab_id: u64, command: String) {
        let Some(host_id) = self.workspace.find_tab(tab_id).map(|tab| tab.host_id) else {
            return;
        };
        let Some(host) = find_host(&self.hosts, host_id).cloned() else {
            return;
        };
        if !self.connected_hosts.contains(&host_id) {
            self.log(
                LogLevel::Warning,
                format!("Command ignored: {} is disconnected", host.name),
            );
            return;
        }
        let Some(tab) = self.workspace.find_tab_mut(tab_id) else {
            return;
        };
        if tab.kind != TabKind::Terminal {
            return;
        }
        if command.trim() == "clear" {
            tab.terminal.clear();
        }
        tab.input.clear();
        tab.scroll_to_bottom = true;

        let command_label = command.clone();
        let mut payload = command.into_bytes();
        payload.push(b'\r');
        if !self.ssh.send_input(tab_id, payload.clone()) {
            self.log(
                LogLevel::Warning,
                format!("Command ignored: {} is not connected", host.name),
            );
            return;
        }
        self.pending_command_history
            .push((tab_id, command_label.clone()));

        if self.broadcast {
            let target_ids = self
                .workspace
                .root
                .terminal_tab_ids()
                .into_iter()
                .filter(|id| *id != tab_id)
                .collect::<Vec<_>>();
            for other_id in target_ids {
                let Some(other_host_id) =
                    self.workspace.find_tab(other_id).map(|other| other.host_id)
                else {
                    continue;
                };
                if self.connected_hosts.contains(&other_host_id) {
                    self.ssh.send_input(other_id, payload.clone());
                }
            }
        }
        self.log(
            LogLevel::Info,
            format!("Sent to {}: {command_label}", host.name),
        );
    }
}
