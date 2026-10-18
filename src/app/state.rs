use crate::domain::{AuthKind, ConnectionBackend, DropZone, KeySource};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarTab {
    Hosts,
    Tunnels,
    Queue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleTab {
    Logs,
    Transfers,
    Tunnels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogSelection {
    pub anchor_row: usize,
    pub cursor_row: usize,
}

impl LogSelection {
    pub fn ordered(self) -> (usize, usize) {
        if self.anchor_row <= self.cursor_row {
            (self.anchor_row, self.cursor_row)
        } else {
            (self.cursor_row, self.anchor_row)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLock {
    Unlocked,
    Locked,
    CredentialsLost,
}

#[derive(Clone, Debug)]
pub struct HostDraft {
    pub editing_id: Option<u64>,
    pub title: String,
    pub name: String,
    pub backend: ConnectionBackend,
    pub wsl_distro: String,
    pub ip: String,
    pub port: String,
    pub user: String,
    pub auth: AuthKind,
    pub key_source: KeySource,
    pub key_id: Option<u64>,
    pub secret: String,
    pub folder_id: Option<u64>,
    pub gateway: String,
    pub environment: String,
    pub keep_alive: String,
    pub quick: bool,
    pub connect_after_save: bool,
}

impl Default for HostDraft {
    fn default() -> Self {
        Self {
            editing_id: None,
            title: String::new(),
            name: String::new(),
            backend: ConnectionBackend::Ssh,
            wsl_distro: String::new(),
            ip: String::new(),
            port: "22".to_owned(),
            user: "root".to_owned(),
            auth: AuthKind::Key,
            key_source: KeySource::FileSystem,
            key_id: None,
            secret: "~/.ssh/id_ed25519".to_owned(),
            folder_id: None,
            gateway: "direct".to_owned(),
            environment: "LOCAL".to_owned(),
            keep_alive: "30".to_owned(),
            quick: false,
            connect_after_save: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FolderDraft {
    pub editing_id: Option<u64>,
    pub name: String,
    pub subtitle: String,
    pub parent_id: Option<u64>,
}

impl Default for FolderDraft {
    fn default() -> Self {
        Self {
            editing_id: None,
            name: String::new(),
            subtitle: String::new(),
            parent_id: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EditorDialog {
    pub path: String,
    pub remote: bool,
    pub tab_id: Option<u64>,
    pub content: String,
    pub truncated: bool,
}

#[derive(Clone, Debug)]
pub struct ChmodDialog {
    pub path: String,
    pub mode: String,
    pub tab_id: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Md5Dialog {
    pub path: String,
    pub hash: String,
}

#[derive(Clone, Debug)]
pub struct TabDrag {
    pub tab_id: u64,
    pub source_pane: u64,
    pub title: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeItem {
    Host(u64),
    Folder(u64),
}

#[derive(Clone, Debug)]
pub struct HostTreeDrag {
    pub item: TreeItem,
    pub title: String,
}

#[derive(Clone, Copy, Debug)]
pub struct DropTarget {
    pub pane_id: u64,
    pub zone: DropZone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityAction {
    Set,
    Change,
    Remove,
}

#[derive(Clone, Debug)]
pub struct FontCache {
    pub ui_font: String,
    pub terminal_font: String,
}
