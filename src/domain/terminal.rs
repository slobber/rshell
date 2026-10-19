use std::sync::{Arc, Mutex};

use crate::domain::FileEntry;

use super::host::Host;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabKind {
    Terminal,
    Sftp,
    Rdp,
}

#[derive(Clone, Copy, Debug)]
pub struct SftpColumnWidths {
    pub perm: f32,
    pub owner: f32,
    pub size: f32,
    pub date: f32,
}

impl Default for SftpColumnWidths {
    fn default() -> Self {
        Self {
            perm: 90.0,
            owner: 72.0,
            size: 62.0,
            date: 140.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SessionTab {
    pub id: u64,
    pub session_uid: String,
    pub kind: TabKind,
    pub host_id: u64,
    pub title: String,
    pub input: String,
    pub last_command: String,
    pub cwd: String,
    pub local_cwd: String,
    pub remote_cwd: String,
    pub left_host_id: Option<u64>,
    pub left_session_id: Option<u64>,
    pub left_loading: bool,
    pub left_request_path: String,
    pub local_path_input: String,
    pub remote_path_input: String,
    pub local_selected: Vec<String>,
    pub remote_selected: Vec<String>,
    pub local_anchor: Option<usize>,
    pub remote_anchor: Option<usize>,
    pub local_columns: SftpColumnWidths,
    pub remote_columns: SftpColumnWidths,
    pub sync_browse: bool,
    pub scroll_to_bottom: bool,
    pub local_entries: Vec<FileEntry>,
    pub remote_entries: Vec<FileEntry>,
    pub remote_loading: bool,
    pub local_error: Option<String>,
    pub remote_error: Option<String>,
    pub remote_request_path: String,
    pub terminal: TerminalBuffer,
    pub terminal_selection: Option<TerminalSelection>,
}

impl SessionTab {
    pub fn terminal(id: u64, host: &Host) -> Self {
        Self {
            id,
            session_uid: new_session_uid(),
            kind: TabKind::Terminal,
            host_id: host.id,
            title: host.name.clone(),
            input: String::new(),
            last_command: String::new(),
            cwd: "~".to_owned(),
            local_cwd: "~".to_owned(),
            remote_cwd: "/".to_owned(),
            left_host_id: None,
            left_session_id: None,
            left_loading: false,
            left_request_path: String::new(),
            local_path_input: "~".to_owned(),
            remote_path_input: "/".to_owned(),
            local_selected: Vec::new(),
            remote_selected: Vec::new(),
            local_anchor: None,
            remote_anchor: None,
            local_columns: SftpColumnWidths::default(),
            remote_columns: SftpColumnWidths::default(),
            sync_browse: false,
            scroll_to_bottom: true,
            local_entries: Vec::new(),
            remote_entries: Vec::new(),
            remote_loading: false,
            local_error: None,
            remote_error: None,
            remote_request_path: String::new(),
            terminal: TerminalBuffer::new(36, 120),
            terminal_selection: None,
        }
    }

    pub fn sftp(id: u64, host: &Host) -> Self {
        Self {
            id,
            session_uid: new_session_uid(),
            kind: TabKind::Sftp,
            host_id: host.id,
            title: host.name.clone(),
            input: String::new(),
            last_command: String::new(),
            cwd: "~".to_owned(),
            local_cwd: crate::local_fs::display_path(&crate::local_fs::home_dir()),
            remote_cwd: "/".to_owned(),
            left_host_id: None,
            left_session_id: None,
            left_loading: false,
            left_request_path: String::new(),
            local_path_input: crate::local_fs::display_path(&crate::local_fs::home_dir()),
            remote_path_input: String::new(),
            local_selected: Vec::new(),
            remote_selected: Vec::new(),
            local_anchor: None,
            remote_anchor: None,
            local_columns: SftpColumnWidths::default(),
            remote_columns: SftpColumnWidths::default(),
            sync_browse: false,
            scroll_to_bottom: false,
            local_entries: Vec::new(),
            remote_entries: Vec::new(),
            remote_loading: false,
            local_error: None,
            remote_error: None,
            remote_request_path: String::new(),
            terminal: TerminalBuffer::new(36, 120),
            terminal_selection: None,
        }
    }

    pub fn rdp(id: u64, host: &Host) -> Self {
        let mut tab = Self::terminal(id, host);
        tab.kind = TabKind::Rdp;
        tab
    }
}

fn new_session_uid() -> String {
    let mut bytes = [0_u8; 16];
    if getrandom::fill(&mut bytes).is_ok() {
        return bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{}-{nanos}", std::process::id())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalSelection {
    pub anchor_row: u16,
    pub anchor_col: u16,
    pub cursor_row: u16,
    pub cursor_col: u16,
}

impl TerminalSelection {
    pub fn ordered(self) -> ((u16, u16), (u16, u16)) {
        let anchor = (self.anchor_row, self.anchor_col);
        let cursor = (self.cursor_row, self.cursor_col);
        if anchor <= cursor {
            (anchor, cursor)
        } else {
            (cursor, anchor)
        }
    }
}

#[derive(Clone)]
pub struct TerminalBuffer {
    parser: Arc<Mutex<vt100::Parser>>,
}

impl std::fmt::Debug for TerminalBuffer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TerminalBuffer")
            .field("size", &self.size())
            .finish()
    }
}

impl TerminalBuffer {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 4_000))),
        }
    }

    pub fn process(&self, bytes: &[u8]) {
        if let Ok(mut parser) = self.parser.lock() {
            parser.process(bytes);
        }
    }

    pub fn write_text(&self, text: &str) {
        if text.contains("\r\n") {
            self.process(text.as_bytes());
        } else {
            self.process(text.replace('\n', "\r\n").as_bytes());
        }
    }

    /// Starts subsequent output on a fresh line when the previous session
    /// stopped mid-line, for example right after a shell prompt.
    pub fn ensure_new_line(&self) {
        let column = self
            .with_screen(|screen| screen.cursor_position().1)
            .unwrap_or(0);
        if column > 0 {
            self.process(b"\r\n");
        }
    }

    pub fn size(&self) -> (u16, u16) {
        self.parser
            .lock()
            .map(|parser| parser.screen().size())
            .unwrap_or((36, 120))
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        if let Ok(mut parser) = self.parser.lock() {
            parser.screen_mut().set_size(rows.max(8), cols.max(20));
        }
    }

    pub fn with_screen<R>(&self, read: impl FnOnce(&vt100::Screen) -> R) -> Option<R> {
        self.parser.lock().ok().map(|parser| read(parser.screen()))
    }

    pub fn with_screen_mut<R>(&self, write: impl FnOnce(&mut vt100::Screen) -> R) -> Option<R> {
        self.parser
            .lock()
            .ok()
            .map(|mut parser| write(parser.screen_mut()))
    }

    pub fn clear(&self) {
        if let Ok(mut parser) = self.parser.lock() {
            let (rows, cols) = parser.screen().size();
            *parser = vt100::Parser::new(rows, cols, 4_000);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalBuffer;

    #[test]
    fn ensure_new_line_moves_output_below_a_prompt() {
        let buffer = TerminalBuffer::new(10, 40);
        buffer.write_text("slobber@host:~$ ");
        buffer.ensure_new_line();
        assert_eq!(
            buffer.with_screen(|screen| screen.cursor_position()),
            Some((1, 0))
        );

        buffer.ensure_new_line();
        assert_eq!(
            buffer.with_screen(|screen| screen.cursor_position()),
            Some((1, 0)),
            "an empty current line must not produce extra blank lines"
        );
    }
}
