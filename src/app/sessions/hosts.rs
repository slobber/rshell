use crate::app::{FolderDraft, HostDraft, ShellCoreApp};
use crate::domain::{
    find_folder, find_folder_mut, find_host, find_host_mut, next_host_id, ConnectionBackend, Host,
    HostFolder, HostStatus, KeySource, LogLevel,
};

impl ShellCoreApp {
    pub fn open_edit_host(&mut self, host_id: u64) {
        let Some(host) = find_host(&self.hosts, host_id).cloned() else {
            return;
        };
        self.active_host_id = Some(host_id);
        self.host_dialog = Some(HostDraft {
            editing_id: Some(host.id),
            title: self.t("edit_connection").to_owned(),
            name: host.name,
            backend: host.backend,
            wsl_distro: host.wsl_distro.unwrap_or_default(),
            ip: host.ip,
            port: host.port.to_string(),
            user: host.user,
            auth: host.auth,
            key_source: host.key_source,
            key_id: host.key_id,
            secret: host.secret,
            folder_id: Some(host.folder_id),
            gateway: host.gateway,
            environment: host.environment,
            keep_alive: host.keep_alive.to_string(),
            quick: false,
            connect_after_save: false,
        });
    }

    pub fn add_host(&mut self, draft: HostDraft) {
        let name = draft.name.trim();
        let ip = draft.ip.trim();
        let is_wsl = draft.backend == ConnectionBackend::LocalWsl;
        let is_rdp = draft.backend == ConnectionBackend::Rdp;
        if name.is_empty() || (!is_wsl && ip.is_empty()) {
            self.toast("Host name and IP are required");
            return;
        }
        let port = if is_wsl {
            0
        } else if is_rdp {
            draft.port.parse::<u16>().unwrap_or(3389)
        } else {
            draft.port.parse::<u16>().unwrap_or(22)
        };
        let user = draft.user.trim().to_owned();
        let folder_id = match draft.folder_id {
            Some(id) if find_folder(&self.hosts, id).is_some() => id,
            _ => {
                let id = max_folder_id(&self.hosts) + 1;
                let mut folder = HostFolder::new(id, "Sessions");
                folder.sort_order = self.hosts.len() as i64;
                self.hosts.push(folder);
                id
            }
        };
        let id = next_host_id(&self.hosts);
        let host = Host {
            id,
            folder_id,
            name: name.to_owned(),
            ip: ip.to_owned(),
            port,
            user: if is_wsl { String::new() } else { user.clone() },
            auth: if is_rdp {
                crate::domain::AuthKind::Password
            } else {
                draft.auth
            },
            key_source: draft.key_source,
            key_id: if is_wsl || is_rdp { None } else { draft.key_id },
            secret: if is_wsl {
                String::new()
            } else if is_rdp {
                draft.secret
            } else if (draft.auth == crate::domain::AuthKind::Key
                && draft.key_source == KeySource::Managed)
                || draft.auth == crate::domain::AuthKind::Agent
            {
                String::new()
            } else {
                draft.secret
            },
            gateway: if is_wsl {
                "direct".to_owned()
            } else {
                draft.gateway
            },
            environment: draft.environment,
            keep_alive: if is_wsl {
                0
            } else {
                draft.keep_alive.parse::<u32>().unwrap_or(30)
            },
            backend: draft.backend,
            os: crate::domain::HostOs::Unknown,
            wsl_distro: if is_wsl {
                Some(draft.wsl_distro.trim().to_owned()).filter(|value| !value.is_empty())
            } else {
                None
            },
            status: HostStatus::Offline,
            latency_ms: None,
            credentials_unavailable: false,
        };
        if let Some(folder) = find_folder_mut(&mut self.hosts, folder_id) {
            folder.hosts.push(host);
        }
        self.active_host_id = Some(id);
        self.persist_hosts();
        let address = if is_wsl {
            format!("wsl:{}", draft.wsl_distro.trim())
        } else if is_rdp {
            format!("rdp:{user}@{ip}:{port}")
        } else {
            format!("{ip}:{port}")
        };
        self.log(LogLevel::Info, format!("Host added: {name} ({address})"));
        self.toast(self.t("host_added"));
        if draft.connect_after_save {
            self.ensure_connected(id);
            self.new_terminal_tab_for_host(id);
        }
    }

    pub fn save_host(&mut self, draft: HostDraft) {
        if draft.editing_id.is_some() {
            self.update_host(draft);
        } else {
            self.add_host(draft);
        }
    }

    fn update_host(&mut self, draft: HostDraft) {
        let Some(host_id) = draft.editing_id else {
            return;
        };
        let Some(existing) = find_host(&self.hosts, host_id).cloned() else {
            self.toast("Host no longer exists");
            return;
        };
        let name = draft.name.trim().to_owned();
        let ip = draft.ip.trim().to_owned();
        let is_wsl = draft.backend == ConnectionBackend::LocalWsl;
        let is_rdp = draft.backend == ConnectionBackend::Rdp;
        if name.is_empty() || (!is_wsl && ip.is_empty()) {
            self.toast("Host name and IP are required");
            return;
        }
        let folder_id = draft
            .folder_id
            .filter(|id| find_folder(&self.hosts, *id).is_some())
            .unwrap_or(existing.folder_id);
        let port = if is_wsl {
            0
        } else if is_rdp {
            draft.port.parse::<u16>().unwrap_or(existing.port)
        } else {
            draft.port.parse::<u16>().unwrap_or(existing.port)
        };
        let user = draft.user.trim().to_owned();
        let keep_alive = if is_wsl {
            0
        } else {
            draft
                .keep_alive
                .parse::<u32>()
                .unwrap_or(existing.keep_alive)
        };
        let secret = if is_wsl {
            String::new()
        } else if is_rdp {
            draft.secret
        } else if (draft.auth == crate::domain::AuthKind::Key
            && draft.key_source == crate::domain::KeySource::Managed)
            || draft.auth == crate::domain::AuthKind::Agent
        {
            String::new()
        } else {
            draft.secret
        };
        let was_connected = self.connected_hosts.contains(&host_id)
            || self.ssh.session_count_for_host(host_id) > 0
            || self.rdp.session_count_for_host(host_id) > 0;
        let updated = Host {
            id: host_id,
            folder_id,
            name: name.clone(),
            ip: ip.clone(),
            port,
            user: if is_wsl { String::new() } else { user.clone() },
            auth: if is_rdp {
                crate::domain::AuthKind::Password
            } else {
                draft.auth
            },
            key_source: draft.key_source,
            key_id: if is_wsl || is_rdp { None } else { draft.key_id },
            secret,
            gateway: if is_wsl {
                "direct".to_owned()
            } else {
                draft.gateway
            },
            environment: draft.environment,
            keep_alive,
            backend: draft.backend,
            os: existing.os,
            wsl_distro: if is_wsl {
                Some(draft.wsl_distro.trim().to_owned()).filter(|value| !value.is_empty())
            } else {
                None
            },
            status: if was_connected {
                HostStatus::Online
            } else {
                HostStatus::Offline
            },
            latency_ms: if was_connected {
                existing.latency_ms
            } else {
                None
            },
            credentials_unavailable: false,
        };
        let connection_changed = existing.ip != updated.ip
            || existing.port != updated.port
            || existing.user != updated.user
            || existing.auth != updated.auth
            || existing.key_source != updated.key_source
            || existing.key_id != updated.key_id
            || existing.secret != updated.secret
            || existing.gateway != updated.gateway
            || existing.keep_alive != updated.keep_alive
            || existing.backend != updated.backend
            || existing.wsl_distro != updated.wsl_distro;

        update_tab_titles(&mut self.workspace.root, host_id, &existing.name, &name);
        if existing.folder_id == folder_id {
            if let Some(host) = find_host_mut(&mut self.hosts, host_id) {
                *host = updated;
            }
        } else if take_host(&mut self.hosts, host_id).is_some() {
            if let Some(folder) = find_folder_mut(&mut self.hosts, folder_id) {
                folder.hosts.push(updated);
            }
        }
        self.active_host_id = Some(host_id);
        self.persist_hosts();
        let address = if is_wsl {
            format!("wsl:{}", draft.wsl_distro.trim())
        } else if is_rdp {
            format!("rdp:{user}@{ip}:{port}")
        } else {
            format!("{ip}:{port}")
        };
        self.log(LogLevel::Info, format!("Host updated: {name} ({address})"));
        self.toast(self.t("connection_updated"));

        if was_connected && connection_changed {
            self.ssh.disconnect_host(host_id);
            self.rdp.disconnect_host(host_id);
            self.connected_hosts.remove(&host_id);
            let tab_ids = self.host_tab_ids(host_id);
            for tab_id in tab_ids {
                self.start_tab_session(tab_id);
            }
        }
    }

    pub fn add_folder(&mut self, draft: FolderDraft) {
        if draft.name.trim().is_empty() {
            return;
        }
        let id = max_folder_id(&self.hosts) + 1;
        let mut folder = HostFolder::new(id, draft.name.trim());
        folder.subtitle = draft.subtitle.trim().to_owned();
        folder.parent_id = draft.parent_id;
        folder.sort_order = 0;
        if let Some(parent_id) = draft.parent_id {
            if let Some(parent) = find_folder_mut(&mut self.hosts, parent_id) {
                parent.children.push(folder);
            }
        } else {
            self.hosts.push(folder);
        }
        self.persist_hosts();
        self.toast(self.t("folder_added"));
    }

    pub fn open_rename_folder(&mut self, folder_id: u64) {
        let Some(folder) = find_folder(&self.hosts, folder_id) else {
            return;
        };
        self.folder_dialog = Some(FolderDraft {
            editing_id: Some(folder.id),
            name: folder.title.clone(),
            subtitle: folder.subtitle.clone(),
            parent_id: folder.parent_id,
        });
    }

    pub fn rename_folder(&mut self, draft: FolderDraft) {
        let Some(folder_id) = draft.editing_id else {
            return;
        };
        if draft.name.trim().is_empty() {
            return;
        }
        let Some(folder) = find_folder_mut(&mut self.hosts, folder_id) else {
            return;
        };
        folder.title = draft.name.trim().to_owned();
        folder.subtitle = draft.subtitle.trim().to_owned();
        self.persist_hosts();
        self.toast(self.t("folder_renamed"));
    }

    pub fn delete_folder(&mut self, folder_id: u64) {
        let Some(folder) = find_folder(&self.hosts, folder_id).cloned() else {
            return;
        };
        let mut host_ids = Vec::new();
        collect_folder_host_ids(&folder, &mut host_ids);

        for host_id in &host_ids {
            for tab_id in self.host_tab_ids(*host_id) {
                self.close_tab(tab_id);
            }
            self.ssh.disconnect_host(*host_id);
            self.rdp.disconnect_host(*host_id);
            self.connected_hosts.remove(host_id);
        }

        if take_folder(&mut self.hosts, folder_id).is_none() {
            return;
        }
        if self
            .active_host_id
            .is_some_and(|host_id| host_ids.contains(&host_id))
        {
            self.active_host_id = first_host_id(&self.hosts);
        }

        self.persist_hosts();
        self.toast(self.t("folder_deleted"));
        self.log(
            LogLevel::Info,
            format!(
                "Deleted folder: {} ({} hosts)",
                folder.title,
                host_ids.len()
            ),
        );
    }

    pub fn move_tree_item(&mut self, item: crate::app::TreeItem, target_folder_id: u64) {
        if find_folder(&self.hosts, target_folder_id).is_none() {
            return;
        }
        let moved = match item {
            crate::app::TreeItem::Host(host_id) => {
                let Some(host) = find_host(&self.hosts, host_id) else {
                    return;
                };
                if host.folder_id == target_folder_id {
                    return;
                }
                if let Some(mut host) = take_host(&mut self.hosts, host_id) {
                    host.folder_id = target_folder_id;
                    if let Some(folder) = find_folder_mut(&mut self.hosts, target_folder_id) {
                        folder.hosts.push(host);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            crate::app::TreeItem::Folder(folder_id) => {
                if folder_id == target_folder_id {
                    return;
                }
                let Some(folder) = find_folder(&self.hosts, folder_id) else {
                    return;
                };
                if find_folder(&folder.children, target_folder_id).is_some() {
                    self.toast(self.t("cannot_move_folder"));
                    return;
                }
                if let Some(mut folder) = take_folder(&mut self.hosts, folder_id) {
                    folder.parent_id = Some(target_folder_id);
                    if let Some(target) = find_folder_mut(&mut self.hosts, target_folder_id) {
                        target.children.push(folder);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        };
        if moved {
            self.persist_hosts();
            self.toast(self.t("item_moved"));
        }
    }
}

fn take_host(folders: &mut Vec<HostFolder>, host_id: u64) -> Option<Host> {
    for folder in folders.iter_mut() {
        if let Some(index) = folder.hosts.iter().position(|host| host.id == host_id) {
            return Some(folder.hosts.remove(index));
        }
        if let Some(host) = take_host(&mut folder.children, host_id) {
            return Some(host);
        }
    }
    None
}

fn take_folder(folders: &mut Vec<HostFolder>, folder_id: u64) -> Option<HostFolder> {
    if let Some(index) = folders.iter().position(|folder| folder.id == folder_id) {
        return Some(folders.remove(index));
    }
    for folder in folders.iter_mut() {
        if let Some(found) = take_folder(&mut folder.children, folder_id) {
            return Some(found);
        }
    }
    None
}

fn update_tab_titles(
    node: &mut crate::domain::SplitNode,
    host_id: u64,
    old_name: &str,
    new_name: &str,
) {
    match node {
        crate::domain::SplitNode::Leaf(pane) => {
            for tab in &mut pane.tabs {
                if tab.host_id == host_id && tab.title == old_name {
                    tab.title = new_name.to_owned();
                }
            }
        }
        crate::domain::SplitNode::Split { first, second, .. } => {
            update_tab_titles(first, host_id, old_name, new_name);
            update_tab_titles(second, host_id, old_name, new_name);
        }
    }
}

fn max_folder_id(folders: &[HostFolder]) -> u64 {
    folders
        .iter()
        .map(|folder| folder.id.max(max_folder_id(&folder.children)))
        .max()
        .unwrap_or(100)
}

fn collect_folder_host_ids(folder: &HostFolder, output: &mut Vec<u64>) {
    output.extend(folder.hosts.iter().map(|host| host.id));
    for child in &folder.children {
        collect_folder_host_ids(child, output);
    }
}

fn first_host_id(folders: &[HostFolder]) -> Option<u64> {
    for folder in folders {
        if let Some(host) = folder.hosts.first() {
            return Some(host.id);
        }
        if let Some(host_id) = first_host_id(&folder.children) {
            return Some(host_id);
        }
    }
    None
}
