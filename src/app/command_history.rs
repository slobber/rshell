use crate::app::{CommandHistoryDialog, ShellCoreApp};
use crate::domain::{find_host, TabKind};
use crate::storage::CommandHistoryRecord;

impl ShellCoreApp {
    pub fn open_command_history(&mut self, tab_id: u64) {
        let Some(tab) = self.workspace.find_tab(tab_id) else {
            return;
        };
        let tab = tab.clone();
        self.open_command_history_for_tab(&tab);
    }

    pub fn open_command_history_for_tab(&mut self, tab: &crate::domain::SessionTab) {
        if tab.kind != TabKind::Terminal {
            return;
        }
        let Some(host) = find_host(&self.hosts, tab.host_id) else {
            return;
        };
        self.command_history = Some(CommandHistoryDialog {
            tab_id: tab.id,
            query: tab.last_command.clone(),
            entries: self.load_command_history_entries(),
            current_session_uid: tab.session_uid.clone(),
            host_ip: host.ip.clone(),
            user: host.user.clone(),
            cwd: tab.cwd.clone(),
            same_path_only: true,
            selections: std::collections::HashMap::new(),
        });
    }

    pub fn flush_command_history(&mut self) {
        let pending = std::mem::take(&mut self.pending_command_history);
        for (tab_id, command) in pending {
            self.record_command_history(tab_id, command);
        }
    }

    pub fn insert_history_command(&mut self, tab_id: u64, command: String) -> bool {
        let Some(tab) = self.workspace.find_tab(tab_id) else {
            return false;
        };
        if tab.kind != TabKind::Terminal {
            return false;
        }
        if !self.ssh.send_input(tab_id, command.into_bytes()) {
            return false;
        }
        self.command_history = None;
        self.pending_focus_tab = Some(tab_id);
        true
    }

    fn record_command_history(&mut self, tab_id: u64, command: String) {
        let command = command.trim().to_owned();
        if command.is_empty() {
            return;
        }
        let Some(tab) = self.workspace.find_tab(tab_id) else {
            return;
        };
        if tab.kind != TabKind::Terminal {
            return;
        }
        let Some(host) = find_host(&self.hosts, tab.host_id) else {
            return;
        };
        let record = CommandHistoryRecord {
            session_uid: tab.session_uid.clone(),
            host_ip: host.ip.clone(),
            user: host.user.clone(),
            cwd: tab.cwd.clone(),
            command: command.clone(),
        };
        if let (Some(storage), Some(cipher)) = (&mut self.storage, &self.cipher) {
            if let Err(error) =
                storage.append_command_history(cipher, &record, self.settings.command_history_limit)
            {
                self.storage_error = Some(error);
            }
        }
        let next_cwd = next_cwd_after_command(&record.cwd, &command);
        if let Some(tab) = self.workspace.find_tab_mut(tab_id) {
            tab.last_command = command;
            tab.cwd = next_cwd;
        }
        if self
            .command_history
            .as_ref()
            .is_some_and(|dialog| dialog.tab_id == tab_id)
        {
            let entries = self.load_command_history_entries();
            if let Some(dialog) = &mut self.command_history {
                dialog.entries = entries;
            }
        }
    }

    pub(crate) fn load_command_history_entries(&self) -> Vec<crate::storage::CommandHistoryEntry> {
        let (Some(storage), Some(cipher)) = (&self.storage, &self.cipher) else {
            return Vec::new();
        };
        storage.load_command_history(cipher).unwrap_or_default()
    }

    pub fn delete_command_history_session(&mut self, session_uid: &str) -> Result<(), String> {
        let Some(storage) = &mut self.storage else {
            return Ok(());
        };
        storage.delete_command_history_session(session_uid)
    }
}

fn next_cwd_after_command(current: &str, command: &str) -> String {
    let command = command.trim();
    let mut parts = command.splitn(2, char::is_whitespace);
    if parts.next() != Some("cd") {
        return current.to_owned();
    }
    let target = parts.next().unwrap_or_default().trim();
    let target = target
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            target
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(target);
    match target {
        "" | "~" => "~".to_owned(),
        "-" => current.to_owned(),
        value if value.starts_with("~/") => {
            format!("~/{}", normalize_remote_path(&value[2..]))
        }
        value if value.starts_with('/') => normalize_remote_path(value),
        value if current.starts_with('/') => normalize_remote_path(&format!("{current}/{value}")),
        value if current.starts_with("~/") => {
            format!(
                "~/{}",
                normalize_remote_path(&format!("{}/{value}", &current[2..]))
            )
        }
        value => value.to_owned(),
    }
}

fn normalize_remote_path(path: &str) -> String {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    let joined = parts.join("/");
    if path.starts_with('/') {
        format!("/{joined}")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::{next_cwd_after_command, normalize_remote_path};

    #[test]
    fn tracks_remote_cwd_from_cd_commands() {
        assert_eq!(next_cwd_after_command("~", "cd /"), "/");
        assert_eq!(next_cwd_after_command("/", "cd srv/app"), "/srv/app");
        assert_eq!(next_cwd_after_command("/srv/app", "cd .."), "/srv");
        assert_eq!(next_cwd_after_command("/srv", " ls -la"), "/srv");
        assert_eq!(next_cwd_after_command("/srv", "cd \"..\""), "/");
    }

    #[test]
    fn normalizes_remote_paths() {
        assert_eq!(normalize_remote_path("/a/./b/../c/"), "/a/c");
        assert_eq!(normalize_remote_path("a/b/../../c"), "c");
    }
}
