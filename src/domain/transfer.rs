#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
    Transfer,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub time: String,
    pub level: LogLevel,
    pub message: String,
}

impl LogEntry {
    pub fn now(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            time: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            level,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,
    Download,
    RemoteToRemote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferDestination {
    Local,
    LeftRemote,
    RightRemote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferStatus {
    Active,
    Done,
    Failed,
}

#[derive(Clone, Debug)]
pub struct TransferTask {
    pub id: u64,
    pub tab_id: u64,
    pub name: String,
    pub direction: TransferDirection,
    pub destination_side: TransferDestination,
    pub source: String,
    pub destination: String,
    pub total_bytes: f64,
    pub transferred_bytes: f64,
    pub speed_bps: f64,
    pub status: TransferStatus,
    pub logged_done: bool,
}

impl TransferTask {
    pub fn progress(&self) -> f32 {
        if self.total_bytes <= 0.0 {
            0.0
        } else {
            (self.transferred_bytes / self.total_bytes).clamp(0.0, 1.0) as f32
        }
    }

    pub fn eta(&self) -> String {
        if self.status == TransferStatus::Done {
            "done".to_owned()
        } else if self.speed_bps <= 1.0 {
            "--".to_owned()
        } else {
            format!(
                "{}s",
                ((self.total_bytes - self.transferred_bytes).max(0.0) / self.speed_bps).ceil()
                    as u64
            )
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TunnelState {
    Online,
    Reconnecting,
    Stopped,
}

#[derive(Clone, Debug)]
pub struct Tunnel {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub local: String,
    pub remote: String,
    pub state: TunnelState,
    pub traffic: String,
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
    pub perm: String,
    pub owner: String,
    pub archive: bool,
    pub symlink: bool,
}

impl FileEntry {
    pub fn size_label(&self) -> String {
        if self.is_dir {
            "<DIR>".to_owned()
        } else {
            format_bytes(self.size)
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let value = bytes as f64;
    if value >= GB {
        format!("{:.1} GB", value / GB)
    } else if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.1} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}
