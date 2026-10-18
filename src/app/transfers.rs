use crate::domain::{
    LogLevel, SessionTab, TransferDestination, TransferDirection, TransferStatus, TransferTask,
};

use super::{ConsoleTab, ShellCoreApp};

enum TransferRoute {
    Upload,
    Download,
    Remote {
        source_session_id: u64,
        destination_session_id: u64,
    },
}

impl ShellCoreApp {
    pub fn queue_transfer(&mut self, tab_id: u64, direction: TransferDirection) {
        let Some(tab) = self.workspace.find_tab(tab_id).cloned() else {
            return;
        };
        self.queue_transfer_for_tab(&tab, direction);
    }

    pub fn queue_transfer_for_tab(&mut self, tab: &SessionTab, direction: TransferDirection) {
        let names = match direction {
            TransferDirection::Upload => tab.local_selected.clone(),
            TransferDirection::Download => tab.remote_selected.clone(),
            TransferDirection::RemoteToRemote => return,
        };
        self.queue_transfers_for_tab(tab, direction, &names);
    }

    pub fn queue_transfers_for_tab(
        &mut self,
        tab: &SessionTab,
        direction: TransferDirection,
        names: &[String],
    ) {
        if matches!(direction, TransferDirection::RemoteToRemote) {
            return;
        }
        if names.is_empty() {
            self.toast(self.t("select_file_first"));
            return;
        }
        let left_remote = tab.left_host_id.is_some();
        if left_remote && tab.left_session_id.is_none() {
            self.toast("SFTP source channel is not available");
            return;
        }

        let transfers_before = self.transfers.len();
        let mut queued = 0;
        for name in names {
            if self.queue_one_transfer(tab, direction, name) {
                queued += 1;
            }
        }
        if queued == 0 {
            if self.transfers.len() > transfers_before {
                self.toast(self.t("sftp_channel_unavailable"));
            } else {
                self.toast(self.t("select_file_first"));
            }
            return;
        }
        self.console_tab = ConsoleTab::Transfers;
        self.settings.console_collapsed = false;
        self.toast(self.t("transfer_queued"));
    }

    fn queue_one_transfer(
        &mut self,
        tab: &SessionTab,
        direction: TransferDirection,
        selected_name: &str,
    ) -> bool {
        let left_remote = tab.left_host_id.is_some();
        let left_session_id = tab.left_session_id;
        let destination_side = match direction {
            TransferDirection::Upload => TransferDestination::RightRemote,
            TransferDirection::Download if left_remote => TransferDestination::LeftRemote,
            TransferDirection::Download => TransferDestination::Local,
            TransferDirection::RemoteToRemote => return false,
        };
        let (entry, source, destination, size, task_direction, route) = match direction {
            TransferDirection::Upload => {
                let Some(entry) = tab
                    .local_entries
                    .iter()
                    .find(|entry| entry.name == selected_name && !entry.is_dir)
                else {
                    return false;
                };
                (
                    entry.clone(),
                    if left_remote {
                        join_remote(&tab.local_cwd, &entry.name)
                    } else {
                        crate::local_fs::join(&tab.local_cwd, &entry.name)
                    },
                    join_remote(&tab.remote_cwd, &entry.name),
                    entry.size,
                    if left_remote {
                        TransferDirection::RemoteToRemote
                    } else {
                        TransferDirection::Upload
                    },
                    if let Some(source_session_id) = left_session_id {
                        TransferRoute::Remote {
                            source_session_id,
                            destination_session_id: tab.id,
                        }
                    } else {
                        TransferRoute::Upload
                    },
                )
            }
            TransferDirection::Download => {
                let Some(entry) = tab
                    .remote_entries
                    .iter()
                    .find(|entry| entry.name == selected_name && !entry.is_dir)
                else {
                    return false;
                };
                (
                    entry.clone(),
                    join_remote(&tab.remote_cwd, &entry.name),
                    if left_remote {
                        join_remote(&tab.local_cwd, &entry.name)
                    } else {
                        crate::local_fs::join(&tab.local_cwd, &entry.name)
                    },
                    entry.size,
                    if left_remote {
                        TransferDirection::RemoteToRemote
                    } else {
                        TransferDirection::Download
                    },
                    if let Some(destination_session_id) = left_session_id {
                        TransferRoute::Remote {
                            source_session_id: tab.id,
                            destination_session_id,
                        }
                    } else {
                        TransferRoute::Download
                    },
                )
            }
            TransferDirection::RemoteToRemote => return false,
        };
        let id = self.next_transfer_id;
        self.next_transfer_id += 1;
        let task = TransferTask {
            id,
            tab_id: tab.id,
            name: entry.name.clone(),
            direction: task_direction,
            destination_side,
            source: source.clone(),
            destination: destination.clone(),
            total_bytes: size.max(1) as f64,
            transferred_bytes: 0.0,
            speed_bps: 0.0,
            status: TransferStatus::Active,
            logged_done: false,
        };
        self.transfers.push(task);
        let accepted = match route {
            TransferRoute::Upload => self.ssh.upload(tab.id, id, source, destination, size),
            TransferRoute::Download => self.ssh.download(tab.id, id, source, destination, size),
            TransferRoute::Remote {
                source_session_id,
                destination_session_id,
            } => self.ssh.copy_remote_to_remote(
                id,
                source_session_id,
                source,
                destination_session_id,
                destination,
                size,
            ),
        };
        if !accepted {
            if let Some(task) = self.transfers.iter_mut().find(|task| task.id == id) {
                task.status = TransferStatus::Failed;
            }
            self.log(
                LogLevel::Error,
                format!("SFTP channel is unavailable for {}", entry.name),
            );
            return false;
        }
        self.log(
            LogLevel::Transfer,
            format!(
                "[SFTP {}] queued {} ({})",
                match task_direction {
                    TransferDirection::Upload => "Upload",
                    TransferDirection::Download => "Download",
                    TransferDirection::RemoteToRemote => "Remote",
                },
                entry.name,
                crate::domain::format_bytes(size)
            ),
        );
        true
    }
}

fn join_remote(parent: &str, name: &str) -> String {
    if parent.ends_with('/') {
        format!("{parent}{name}")
    } else {
        format!("{parent}/{name}")
    }
}
