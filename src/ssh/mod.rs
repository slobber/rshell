use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize};
use russh::client;
use russh::keys::ssh_key::HashAlg;
use russh::keys::{PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh::{ChannelMsg, ChannelWriteHalf, Disconnect};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileAttributes, OpenFlags};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc as tokio_mpsc;

use crate::domain::{FileEntry, Host, HostOs, LogLevel};
use crate::local_fs;

#[derive(Clone, Debug)]
pub enum SshAuthMaterial {
    Password(String),
    PrivateKey(String),
    Agent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SshMode {
    Terminal { cols: u32, rows: u32 },
    Sftp,
}

#[derive(Debug)]
pub enum SshEvent {
    Connected {
        tab_id: u64,
        host_id: u64,
        latency_ms: u32,
    },
    ConnectFailed {
        tab_id: u64,
        host_id: u64,
        message: String,
    },
    OsDetected {
        host_id: u64,
        os: HostOs,
    },
    HostKey {
        tab_id: u64,
        host_key: String,
        fingerprint: String,
        trusted: bool,
        mismatch: bool,
    },
    TerminalData {
        tab_id: u64,
        data: Vec<u8>,
    },
    TerminalClosed {
        tab_id: u64,
        host_id: u64,
        message: String,
    },
    SftpHome {
        tab_id: u64,
        path: String,
    },
    RemoteEntries {
        tab_id: u64,
        path: String,
        entries: Vec<FileEntry>,
    },
    RemoteError {
        tab_id: u64,
        path: String,
        message: String,
    },
    FileContent {
        tab_id: u64,
        path: String,
        content: String,
        truncated: bool,
    },
    FileSaved {
        tab_id: u64,
        path: String,
    },
    PermissionsChanged {
        tab_id: u64,
        path: String,
        mode: String,
    },
    HashResult {
        tab_id: u64,
        path: String,
        hash: String,
    },
    TransferProgress {
        transfer_id: u64,
        transferred_bytes: f64,
        total_bytes: f64,
        speed_bps: f64,
    },
    TransferDone {
        transfer_id: u64,
        total_bytes: f64,
    },
    TransferFailed {
        transfer_id: u64,
        message: String,
    },
    Notice {
        tab_id: u64,
        level: LogLevel,
        message: String,
    },
}

enum SshCommand {
    Input(Vec<u8>),
    Resize {
        cols: u32,
        rows: u32,
    },
    ListRemote(String),
    ReadRemoteFile(String),
    SaveRemoteFile {
        path: String,
        content: String,
    },
    Upload {
        transfer_id: u64,
        local_path: String,
        remote_path: String,
        total_bytes: u64,
    },
    Download {
        transfer_id: u64,
        remote_path: String,
        local_path: String,
        total_bytes: u64,
    },
    SetPermissions {
        path: String,
        mode: String,
    },
    Md5Remote(String),
    Disconnect,
}

struct SessionEntry {
    host_id: u64,
    tx: tokio_mpsc::UnboundedSender<SshCommand>,
}

pub struct SshManager {
    sessions: HashMap<u64, SessionEntry>,
    sftp_sessions: Arc<Mutex<HashMap<u64, Arc<SftpSession>>>>,
    events: Receiver<SshEvent>,
    event_tx: Sender<SshEvent>,
    known_hosts: HashMap<String, String>,
}

impl SshManager {
    pub fn new(known_hosts: HashMap<String, String>) -> Self {
        let (event_tx, events) = mpsc::channel();
        Self {
            sessions: HashMap::new(),
            sftp_sessions: Arc::new(Mutex::new(HashMap::new())),
            events,
            event_tx,
            known_hosts,
        }
    }

    pub fn known_hosts(&self) -> &HashMap<String, String> {
        &self.known_hosts
    }

    pub fn trust_host_key(&mut self, host_key: String, fingerprint: String) {
        self.known_hosts.insert(host_key, fingerprint);
    }

    pub fn open_terminal(
        &mut self,
        tab_id: u64,
        host: Host,
        auth: SshAuthMaterial,
        cols: u32,
        rows: u32,
    ) -> Result<(), String> {
        self.start_session(
            tab_id,
            host,
            auth,
            SshMode::Terminal {
                cols: cols.max(20),
                rows: rows.max(8),
            },
        )
    }

    pub fn open_sftp(
        &mut self,
        tab_id: u64,
        host: Host,
        auth: SshAuthMaterial,
    ) -> Result<(), String> {
        self.start_session(tab_id, host, auth, SshMode::Sftp)
    }

    pub fn open_local_terminal(
        &mut self,
        tab_id: u64,
        host: Host,
        cols: u32,
        rows: u32,
    ) -> Result<(), String> {
        let host_id = host.id;
        let event_tx = self.event_tx.clone();
        let (command_tx, command_rx) = tokio_mpsc::unbounded_channel();
        let thread_name = format!("rshell-wsl-{tab_id}");
        std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || run_local_shell(tab_id, host, cols, rows, command_rx, event_tx))
            .map_err(|error| format!("Could not start WSL worker: {error}"))?;

        self.sessions.insert(
            tab_id,
            SessionEntry {
                host_id,
                tx: command_tx,
            },
        );
        Ok(())
    }

    fn start_session(
        &mut self,
        tab_id: u64,
        host: Host,
        auth: SshAuthMaterial,
        mode: SshMode,
    ) -> Result<(), String> {
        let host_id = host.id;
        let host_key = host_key_name(&host);
        let expected_host_key = self.known_hosts.get(&host_key).cloned();
        let event_tx = self.event_tx.clone();
        let sftp_sessions = self.sftp_sessions.clone();
        let (command_tx, command_rx) = tokio_mpsc::unbounded_channel();
        let thread_name = format!("rshell-ssh-{tab_id}");
        std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = event_tx.send(SshEvent::ConnectFailed {
                            tab_id,
                            host_id,
                            message: format!("Could not start SSH runtime: {error}"),
                        });
                        return;
                    }
                };
                runtime.block_on(run_worker(WorkerRequest {
                    tab_id,
                    host,
                    host_key,
                    expected_host_key,
                    auth,
                    mode,
                    commands: command_rx,
                    events: event_tx,
                    sftp_sessions,
                }));
            })
            .map_err(|error| format!("Could not start SSH worker: {error}"))?;

        self.sessions.insert(
            tab_id,
            SessionEntry {
                host_id,
                tx: command_tx,
            },
        );
        Ok(())
    }

    pub fn drain_events(&self) -> Vec<SshEvent> {
        self.events.try_iter().collect()
    }

    pub fn send_input(&self, tab_id: u64, data: Vec<u8>) -> bool {
        self.send(tab_id, SshCommand::Input(data))
    }

    pub fn resize(&self, tab_id: u64, cols: u32, rows: u32) -> bool {
        self.send(
            tab_id,
            SshCommand::Resize {
                cols: cols.max(20),
                rows: rows.max(8),
            },
        )
    }

    pub fn list_remote(&self, tab_id: u64, path: String) -> bool {
        self.send(tab_id, SshCommand::ListRemote(path))
    }

    pub fn read_remote_file(&self, tab_id: u64, path: String) -> bool {
        self.send(tab_id, SshCommand::ReadRemoteFile(path))
    }

    pub fn save_remote_file(&self, tab_id: u64, path: String, content: String) -> bool {
        self.send(tab_id, SshCommand::SaveRemoteFile { path, content })
    }

    pub fn upload(
        &self,
        tab_id: u64,
        transfer_id: u64,
        local_path: String,
        remote_path: String,
        total_bytes: u64,
    ) -> bool {
        self.send(
            tab_id,
            SshCommand::Upload {
                transfer_id,
                local_path,
                remote_path,
                total_bytes,
            },
        )
    }

    pub fn download(
        &self,
        tab_id: u64,
        transfer_id: u64,
        remote_path: String,
        local_path: String,
        total_bytes: u64,
    ) -> bool {
        self.send(
            tab_id,
            SshCommand::Download {
                transfer_id,
                remote_path,
                local_path,
                total_bytes,
            },
        )
    }

    pub fn copy_remote_to_remote(
        &self,
        transfer_id: u64,
        source_session_id: u64,
        source_path: String,
        destination_session_id: u64,
        destination_path: String,
        total_bytes: u64,
    ) -> bool {
        let (source, destination) = {
            let Ok(sessions) = self.sftp_sessions.lock() else {
                return false;
            };
            let Some(source) = sessions.get(&source_session_id).cloned() else {
                return false;
            };
            let Some(destination) = sessions.get(&destination_session_id).cloned() else {
                return false;
            };
            (source, destination)
        };
        let events = self.event_tx.clone();
        std::thread::Builder::new()
            .name(format!("rshell-sftp-copy-{transfer_id}"))
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = events.send(SshEvent::TransferFailed {
                            transfer_id,
                            message: format!("Could not start remote copy runtime: {error}"),
                        });
                        return;
                    }
                };
                runtime.block_on(async move {
                    if let Err(error) = copy_between_remote(
                        source,
                        destination,
                        transfer_id,
                        source_path,
                        destination_path,
                        total_bytes,
                        events.clone(),
                    )
                    .await
                    {
                        let _ = events.send(SshEvent::TransferFailed {
                            transfer_id,
                            message: error,
                        });
                    }
                });
            })
            .is_ok()
    }

    pub fn set_remote_permissions(&self, tab_id: u64, path: String, mode: String) -> bool {
        self.send(tab_id, SshCommand::SetPermissions { path, mode })
    }

    pub fn md5_remote(&self, tab_id: u64, path: String) -> bool {
        self.send(tab_id, SshCommand::Md5Remote(path))
    }

    pub fn close_tab(&mut self, tab_id: u64) {
        if let Ok(mut sessions) = self.sftp_sessions.lock() {
            sessions.remove(&tab_id);
        }
        if let Some(session) = self.sessions.remove(&tab_id) {
            let _ = session.tx.send(SshCommand::Disconnect);
        }
    }

    pub fn forget_session(&mut self, tab_id: u64) {
        self.sessions.remove(&tab_id);
        if let Ok(mut sessions) = self.sftp_sessions.lock() {
            sessions.remove(&tab_id);
        }
    }

    pub fn session_count_for_host(&self, host_id: u64) -> usize {
        self.sessions
            .values()
            .filter(|session| session.host_id == host_id)
            .count()
    }

    pub fn session_tab_ids_for_host(&self, host_id: u64) -> Vec<u64> {
        self.sessions
            .iter()
            .filter_map(|(tab_id, session)| (session.host_id == host_id).then_some(*tab_id))
            .collect()
    }

    pub fn has_session(&self, tab_id: u64) -> bool {
        self.sessions.contains_key(&tab_id)
    }

    pub fn disconnect_host(&mut self, host_id: u64) {
        let tab_ids = self
            .sessions
            .iter()
            .filter_map(|(tab_id, session)| (session.host_id == host_id).then_some(*tab_id))
            .collect::<Vec<_>>();
        for tab_id in tab_ids {
            self.close_tab(tab_id);
        }
    }

    pub fn shutdown(&mut self) {
        let tab_ids = self.sessions.keys().copied().collect::<Vec<_>>();
        for tab_id in tab_ids {
            self.close_tab(tab_id);
        }
    }

    fn send(&self, tab_id: u64, command: SshCommand) -> bool {
        self.sessions
            .get(&tab_id)
            .is_some_and(|session| session.tx.send(command).is_ok())
    }
}

struct ClientHandler {
    tab_id: u64,
    host_key: String,
    expected_host_key: Option<String>,
    event_tx: Sender<SshEvent>,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let fingerprint = server_public_key
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string();
        let mismatch = self
            .expected_host_key
            .as_ref()
            .is_some_and(|expected| expected != &fingerprint);
        let _ = self.event_tx.send(SshEvent::HostKey {
            tab_id: self.tab_id,
            host_key: self.host_key.clone(),
            fingerprint,
            trusted: !mismatch,
            mismatch,
        });
        Ok(!mismatch)
    }
}

struct LocalPty {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    reader: Box<dyn Read + Send>,
}

#[cfg(windows)]
fn spawn_wsl_pty(host: &Host, cols: u32, rows: u32) -> Result<LocalPty, String> {
    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: rows.clamp(8, u32::from(u16::MAX)) as u16,
            cols: cols.clamp(20, u32::from(u16::MAX)) as u16,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Could not open WSL PTY: {error}"))?;

    let mut command = CommandBuilder::new("wsl.exe");
    if let Some(distro) = host.wsl_distro.as_deref().filter(|value| !value.is_empty()) {
        command.arg("-d");
        command.arg(distro);
    }
    command.arg("--cd");
    command.arg("~");
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");

    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Could not start WSL shell: {error}"))?;
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("Could not read WSL PTY: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("Could not write to WSL PTY: {error}"))?;

    Ok(LocalPty {
        master: pair.master,
        writer,
        child,
        reader,
    })
}

#[cfg(not(windows))]
fn spawn_wsl_pty(_host: &Host, _cols: u32, _rows: u32) -> Result<LocalPty, String> {
    Err("WSL sessions are only available on Windows".to_owned())
}

fn run_local_shell(
    tab_id: u64,
    host: Host,
    cols: u32,
    rows: u32,
    mut commands: tokio_mpsc::UnboundedReceiver<SshCommand>,
    events: Sender<SshEvent>,
) {
    let host_id = host.id;
    let started = Instant::now();
    let pty = match spawn_wsl_pty(&host, cols, rows) {
        Ok(pty) => pty,
        Err(message) => {
            let _ = events.send(SshEvent::ConnectFailed {
                tab_id,
                host_id,
                message,
            });
            return;
        }
    };
    let LocalPty {
        master,
        mut writer,
        mut child,
        mut reader,
    } = pty;

    let _ = events.send(SshEvent::Connected {
        tab_id,
        host_id,
        latency_ms: started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
    });

    let reader_events = events.clone();
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if reader_events
                        .send(SshEvent::TerminalData {
                            tab_id,
                            data: buffer[..read].to_vec(),
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = reader_events.send(SshEvent::TerminalClosed {
            tab_id,
            host_id,
            message: "WSL shell closed".to_owned(),
        });
    });

    while let Some(command) = commands.blocking_recv() {
        match command {
            SshCommand::Input(data) => {
                if writer
                    .write_all(&data)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    break;
                }
            }
            SshCommand::Resize { cols, rows } => {
                let _ = master.resize(PtySize {
                    rows: rows.clamp(8, u32::from(u16::MAX)) as u16,
                    cols: cols.clamp(20, u32::from(u16::MAX)) as u16,
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
            SshCommand::Disconnect => {
                let _ = child.kill();
                break;
            }
            _ => {}
        }
    }
    let _ = child.kill();
}

struct Worker {
    tab_id: u64,
    host: Host,
    events: Sender<SshEvent>,
    sftp_sessions: Arc<Mutex<HashMap<u64, Arc<SftpSession>>>>,
    session: client::Handle<ClientHandler>,
    terminal_writer: Option<ChannelWriteHalf<client::Msg>>,
    sftp: Option<Arc<SftpSession>>,
    home: String,
}

struct WorkerRequest {
    tab_id: u64,
    host: Host,
    host_key: String,
    expected_host_key: Option<String>,
    auth: SshAuthMaterial,
    mode: SshMode,
    commands: tokio_mpsc::UnboundedReceiver<SshCommand>,
    events: Sender<SshEvent>,
    sftp_sessions: Arc<Mutex<HashMap<u64, Arc<SftpSession>>>>,
}

async fn run_worker(request: WorkerRequest) {
    let WorkerRequest {
        tab_id,
        host,
        host_key,
        expected_host_key,
        auth,
        mode,
        mut commands,
        events,
        sftp_sessions,
    } = request;
    let host_id = host.id;
    let started = Instant::now();
    let handler = ClientHandler {
        tab_id,
        host_key,
        expected_host_key,
        event_tx: events.clone(),
    };
    let config = Arc::new(client::Config {
        client_id: russh::SshId::Standard(std::borrow::Cow::Borrowed("SSH-2.0-RShell_0.1")),
        keepalive_interval: (host.keep_alive > 0)
            .then_some(Duration::from_secs(u64::from(host.keep_alive))),
        keepalive_max: 3,
        nodelay: true,
        ..Default::default()
    });

    let mut session = match client::connect(config, (host.ip.as_str(), host.port), handler).await {
        Ok(session) => session,
        Err(error) => {
            let _ = events.send(SshEvent::ConnectFailed {
                tab_id,
                host_id,
                message: format!("TCP/SSH handshake failed: {error}"),
            });
            return;
        }
    };

    if let Err(error) = authenticate(&mut session, &host, auth).await {
        let _ = events.send(SshEvent::ConnectFailed {
            tab_id,
            host_id,
            message: error,
        });
        return;
    }

    let latency_ms = started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
    let _ = events.send(SshEvent::Connected {
        tab_id,
        host_id,
        latency_ms,
    });

    let mut worker = Worker {
        tab_id,
        host,
        events: events.clone(),
        sftp_sessions,
        session,
        terminal_writer: None,
        sftp: None,
        home: "/".to_owned(),
    };

    let initial = match mode {
        SshMode::Terminal { cols, rows } => worker.open_terminal(cols, rows).await,
        SshMode::Sftp => worker.open_sftp().await,
    };
    if let Err(error) = initial {
        let _ = events.send(SshEvent::ConnectFailed {
            tab_id,
            host_id,
            message: error,
        });
        return;
    }

    // Re-check the ambiguous families as well: Deepin, UOS and Kylin often
    // advertise a debian/ubuntu ID_LIKE, so older detections may have stored
    // them as Debian or Ubuntu before those distributions were recognized.
    let needs_os_detection = matches!(
        worker.host.os,
        HostOs::Unknown | HostOs::Linux | HostOs::Debian | HostOs::Ubuntu
    );
    if needs_os_detection {
        if let Ok(Some(os)) = tokio::time::timeout(Duration::from_secs(6), worker.detect_os()).await
        {
            let _ = events.send(SshEvent::OsDetected { host_id, os });
        }
    }

    while let Some(command) = commands.recv().await {
        match command {
            SshCommand::Input(data) => {
                if let Some(writer) = &worker.terminal_writer {
                    let _ = writer.data_bytes(data).await;
                }
            }
            SshCommand::Resize { cols, rows } => {
                if let Some(writer) = &worker.terminal_writer {
                    let _ = writer
                        .window_change(cols, rows, cols.saturating_mul(8), rows.saturating_mul(17))
                        .await;
                }
            }
            SshCommand::ListRemote(path) => {
                if let Err(error) = worker.list_remote(path).await {
                    let _ = worker.events.send(SshEvent::RemoteError {
                        tab_id,
                        path: ".".to_owned(),
                        message: error,
                    });
                }
            }
            SshCommand::ReadRemoteFile(path) => {
                if let Err(error) = worker.read_remote_file(path).await {
                    let _ = worker.events.send(SshEvent::Notice {
                        tab_id,
                        level: LogLevel::Error,
                        message: error,
                    });
                }
            }
            SshCommand::SaveRemoteFile { path, content } => {
                match worker.save_remote_file(path.clone(), content).await {
                    Ok(()) => {
                        let _ = worker.events.send(SshEvent::FileSaved { tab_id, path });
                    }
                    Err(error) => {
                        let _ = worker.events.send(SshEvent::Notice {
                            tab_id,
                            level: LogLevel::Error,
                            message: error,
                        });
                    }
                }
            }
            SshCommand::Upload {
                transfer_id,
                local_path,
                remote_path,
                total_bytes,
            } => {
                if let Some(sftp) = worker.sftp.clone() {
                    let events = worker.events.clone();
                    tokio::spawn(async move {
                        if let Err(error) = upload_file(
                            sftp,
                            transfer_id,
                            local_path,
                            remote_path,
                            total_bytes,
                            events.clone(),
                        )
                        .await
                        {
                            let _ = events.send(SshEvent::TransferFailed {
                                transfer_id,
                                message: error,
                            });
                        }
                    });
                } else {
                    let _ = worker.events.send(SshEvent::TransferFailed {
                        transfer_id,
                        message: "SFTP channel is not available".to_owned(),
                    });
                }
            }
            SshCommand::Download {
                transfer_id,
                remote_path,
                local_path,
                total_bytes,
            } => {
                if let Some(sftp) = worker.sftp.clone() {
                    let events = worker.events.clone();
                    tokio::spawn(async move {
                        if let Err(error) = download_file(
                            sftp,
                            transfer_id,
                            remote_path,
                            local_path,
                            total_bytes,
                            events.clone(),
                        )
                        .await
                        {
                            let _ = events.send(SshEvent::TransferFailed {
                                transfer_id,
                                message: error,
                            });
                        }
                    });
                } else {
                    let _ = worker.events.send(SshEvent::TransferFailed {
                        transfer_id,
                        message: "SFTP channel is not available".to_owned(),
                    });
                }
            }
            SshCommand::SetPermissions { path, mode } => {
                match worker.set_permissions(path.clone(), &mode).await {
                    Ok(()) => {
                        let _ =
                            worker
                                .events
                                .send(SshEvent::PermissionsChanged { tab_id, path, mode });
                    }
                    Err(error) => {
                        let _ = worker.events.send(SshEvent::Notice {
                            tab_id,
                            level: LogLevel::Error,
                            message: error,
                        });
                    }
                }
            }
            SshCommand::Md5Remote(path) => match worker.md5_remote(&path).await {
                Ok(hash) => {
                    let _ = worker
                        .events
                        .send(SshEvent::HashResult { tab_id, path, hash });
                }
                Err(error) => {
                    let _ = worker.events.send(SshEvent::Notice {
                        tab_id,
                        level: LogLevel::Error,
                        message: error,
                    });
                }
            },
            SshCommand::Disconnect => break,
        }
    }

    let _ = worker
        .session
        .disconnect(Disconnect::ByApplication, "", "en")
        .await;
    let sftp_sessions = worker.sftp_sessions.clone();
    {
        if let Ok(mut sessions) = sftp_sessions.lock() {
            sessions.remove(&tab_id);
        };
    }
}

impl Worker {
    async fn detect_os(&self) -> Option<HostOs> {
        let unix = self
            .exec_output(
                "sh -c 'if [ -r /etc/os-release ]; then cat /etc/os-release; else uname -s 2>/dev/null; fi'",
            )
            .await
            .unwrap_or_default();
        let os = HostOs::from_detection_output(&unix);
        if os != HostOs::Unknown {
            return Some(os);
        }

        let windows = self.exec_output("cmd.exe /c ver").await.unwrap_or_default();
        let os = HostOs::from_detection_output(&windows);
        (os != HostOs::Unknown).then_some(os)
    }

    async fn exec_output(&self, command: &str) -> Result<String, String> {
        let mut channel = self
            .session
            .channel_open_session()
            .await
            .map_err(|error| error.to_string())?;
        channel
            .exec(true, command)
            .await
            .map_err(|error| error.to_string())?;

        let mut output = Vec::new();
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => output.extend_from_slice(&data),
                ChannelMsg::ExtendedData { data, .. } => output.extend_from_slice(&data),
                ChannelMsg::ExitStatus { .. } | ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
            if output.len() >= 64 * 1024 {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&output).into_owned())
    }

    async fn open_terminal(&mut self, cols: u32, rows: u32) -> Result<(), String> {
        let channel = self
            .session
            .channel_open_session()
            .await
            .map_err(|error| format!("Could not open terminal channel: {error}"))?;
        channel
            .request_pty(true, "xterm-256color", cols, rows, 0, 0, &[])
            .await
            .map_err(|error| format!("PTY request failed: {error}"))?;
        channel
            .request_shell(true)
            .await
            .map_err(|error| format!("Remote shell request failed: {error}"))?;

        let (mut reader, writer) = channel.split();
        self.terminal_writer = Some(writer);
        let events = self.events.clone();
        let tab_id = self.tab_id;
        let host_id = self.host.id;
        tokio::spawn(async move {
            let mut close_message = "Remote shell closed".to_owned();
            while let Some(message) = reader.wait().await {
                match message {
                    ChannelMsg::Data { data } => {
                        if events
                            .send(SshEvent::TerminalData {
                                tab_id,
                                data: data.to_vec(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    ChannelMsg::ExtendedData { data, .. } => {
                        if events
                            .send(SshEvent::TerminalData {
                                tab_id,
                                data: data.to_vec(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    ChannelMsg::ExitStatus { exit_status } => {
                        close_message = format!("Remote shell exited with status {exit_status}");
                    }
                    ChannelMsg::ExitSignal { .. } => {
                        close_message = "Remote shell terminated by signal".to_owned();
                    }
                    ChannelMsg::Eof | ChannelMsg::Close => break,
                    _ => {}
                }
            }
            let _ = events.send(SshEvent::TerminalClosed {
                tab_id,
                host_id,
                message: close_message,
            });
        });
        Ok(())
    }

    async fn open_sftp(&mut self) -> Result<(), String> {
        let sftp = self.ensure_sftp().await?;
        let home = sftp
            .canonicalize(".")
            .await
            .map_err(|error| format!("Could not resolve remote home: {error}"))?;
        self.home = home.clone();
        let _ = self.events.send(SshEvent::SftpHome {
            tab_id: self.tab_id,
            path: home.clone(),
        });
        self.list_remote(home).await
    }

    async fn ensure_sftp(&mut self) -> Result<Arc<SftpSession>, String> {
        if let Some(sftp) = &self.sftp {
            return Ok(sftp.clone());
        }
        let channel = self
            .session
            .channel_open_session()
            .await
            .map_err(|error| format!("Could not open SFTP channel: {error}"))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|error| format!("SFTP subsystem request failed: {error}"))?;
        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| format!("SFTP initialization failed: {error}"))?;
        sftp.set_timeout(30);
        let sftp = Arc::new(sftp);
        if let Ok(mut sessions) = self.sftp_sessions.lock() {
            sessions.insert(self.tab_id, sftp.clone());
        }
        self.sftp = Some(sftp.clone());
        Ok(sftp)
    }

    async fn list_remote(&mut self, path: String) -> Result<(), String> {
        let sftp = self.ensure_sftp().await?;
        let path = if path.trim().is_empty() || path == "." {
            self.home.clone()
        } else {
            path
        };
        let entries = sftp
            .read_dir(path.clone())
            .await
            .map_err(|error| format!("Could not list {path}: {error}"))?
            .map(remote_entry)
            .collect::<Vec<_>>();
        let _ = self.events.send(SshEvent::RemoteEntries {
            tab_id: self.tab_id,
            path,
            entries,
        });
        Ok(())
    }

    async fn read_remote_file(&self, path: String) -> Result<(), String> {
        let sftp = self
            .sftp
            .as_ref()
            .ok_or_else(|| "SFTP channel is not available".to_owned())?;
        let bytes = sftp
            .read(path.clone())
            .await
            .map_err(|error| format!("Could not read {path}: {error}"))?;
        let max_bytes = 1024 * 1024;
        if bytes.len() > max_bytes * 8 {
            return Err(format!(
                "File is too large to open in the built-in editor ({} bytes)",
                bytes.len()
            ));
        }
        let truncated = bytes.len() > max_bytes;
        let content = String::from_utf8_lossy(&bytes[..bytes.len().min(max_bytes)]).into_owned();
        let _ = self.events.send(SshEvent::FileContent {
            tab_id: self.tab_id,
            path,
            content,
            truncated,
        });
        Ok(())
    }

    async fn save_remote_file(&self, path: String, content: String) -> Result<(), String> {
        let sftp = self
            .sftp
            .as_ref()
            .ok_or_else(|| "SFTP channel is not available".to_owned())?;
        let mut file = sftp
            .open_with_flags(
                path.clone(),
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
            )
            .await
            .map_err(|error| format!("Could not open {path}: {error}"))?;
        file.write_all(content.as_bytes())
            .await
            .map_err(|error| format!("Could not write {path}: {error}"))?;
        file.close()
            .await
            .map_err(|error| format!("Could not close {path}: {error}"))
    }

    async fn set_permissions(&self, path: String, mode: &str) -> Result<(), String> {
        let sftp = self
            .sftp
            .as_ref()
            .ok_or_else(|| "SFTP channel is not available".to_owned())?;
        let mode = u32::from_str_radix(mode.trim_start_matches("0o"), 8)
            .map_err(|_| format!("Invalid permission mode: {mode}"))?;
        let mut attributes = FileAttributes::empty();
        attributes.permissions = Some(mode);
        sftp.set_metadata(path.clone(), attributes)
            .await
            .map_err(|error| format!("chmod failed for {path}: {error}"))
    }

    async fn md5_remote(&self, path: &str) -> Result<String, String> {
        let sftp = self
            .sftp
            .as_ref()
            .ok_or_else(|| "SFTP channel is not available".to_owned())?;
        let mut file = sftp
            .open(path.to_owned())
            .await
            .map_err(|error| format!("Could not read {path}: {error}"))?;
        use md5::{Digest, Md5};
        let mut hasher = Md5::new();
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .await
                .map_err(|error| format!("Could not hash {path}: {error}"))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        file.close()
            .await
            .map_err(|error| format!("Could not close {path}: {error}"))?;
        Ok(hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }
}

async fn authenticate(
    session: &mut client::Handle<ClientHandler>,
    host: &Host,
    auth: SshAuthMaterial,
) -> Result<(), String> {
    if host.user.trim().is_empty() {
        return Err("Username is required".to_owned());
    }
    let result = match auth {
        SshAuthMaterial::Password(password) => session
            .authenticate_password(host.user.clone(), password)
            .await
            .map_err(|error| format!("Password authentication failed: {error}"))?,
        SshAuthMaterial::PrivateKey(contents) => {
            let key = load_secret_key_from_text(&contents)?;
            let hash = session
                .best_supported_rsa_hash()
                .await
                .map_err(|error| format!("Could not negotiate RSA signature: {error}"))?
                .flatten();
            session
                .authenticate_publickey(
                    host.user.clone(),
                    PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                )
                .await
                .map_err(|error| format!("Public key authentication failed: {error}"))?
        }
        SshAuthMaterial::Agent => authenticate_agent(session, &host.user).await?,
    };
    if result.success() {
        Ok(())
    } else {
        Err(format!("Authentication rejected by {}", host.address()))
    }
}

fn load_secret_key_from_text(contents: &str) -> Result<russh::keys::PrivateKey, String> {
    russh::keys::decode_secret_key(contents, None)
        .map_err(|error| format!("Could not decode private key: {error}"))
}

#[cfg(unix)]
async fn authenticate_agent(
    session: &mut client::Handle<ClientHandler>,
    user: &str,
) -> Result<russh::client::AuthResult, String> {
    use russh::keys::agent::client::AgentClient;
    let mut agent = AgentClient::connect_env()
        .await
        .map_err(|error| format!("Could not connect to SSH agent: {error}"))?;
    authenticate_with_agent(session, user, &mut agent).await
}

#[cfg(windows)]
async fn authenticate_agent(
    session: &mut client::Handle<ClientHandler>,
    user: &str,
) -> Result<russh::client::AuthResult, String> {
    use russh::keys::agent::client::AgentClient;
    let mut agent = AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent")
        .await
        .map_err(|error| format!("Could not connect to OpenSSH agent: {error}"))?;
    authenticate_with_agent(session, user, &mut agent).await
}

async fn authenticate_with_agent<S>(
    session: &mut client::Handle<ClientHandler>,
    user: &str,
    agent: &mut russh::keys::agent::client::AgentClient<S>,
) -> Result<russh::client::AuthResult, String>
where
    S: russh::keys::agent::client::AgentStream + Send + Unpin,
{
    use russh::keys::agent::AgentIdentity;
    let identities = agent
        .request_identities()
        .await
        .map_err(|error| format!("Could not list SSH agent identities: {error}"))?;
    if identities.is_empty() {
        return Err("SSH agent has no identities".to_owned());
    }
    let mut last_failure = None;
    for identity in identities {
        let result = match identity {
            AgentIdentity::PublicKey { key, .. } => session
                .authenticate_publickey_with(user.to_owned(), key, None, agent)
                .await
                .map_err(|error| error.to_string()),
            AgentIdentity::Certificate { certificate, .. } => session
                .authenticate_certificate_with(user.to_owned(), certificate, None, agent)
                .await
                .map_err(|error| error.to_string()),
        };
        match result {
            Ok(result) if result.success() => return Ok(result),
            Ok(result) => last_failure = Some(result),
            Err(error) => return Err(error),
        }
    }
    Ok(last_failure.unwrap_or(russh::client::AuthResult::Failure {
        remaining_methods: russh::MethodSet::empty(),
        partial_success: false,
    }))
}

async fn upload_file(
    sftp: Arc<SftpSession>,
    transfer_id: u64,
    local_path: String,
    remote_path: String,
    total_bytes: u64,
    events: Sender<SshEvent>,
) -> Result<(), String> {
    let mut local = tokio::fs::File::open(local_fs::resolve(&local_path))
        .await
        .map_err(|error| format!("Could not open {local_path}: {error}"))?;
    let mut remote = sftp
        .create(remote_path.clone())
        .await
        .map_err(|error| format!("Could not create {remote_path}: {error}"))?;
    copy_with_progress(&mut local, &mut remote, transfer_id, total_bytes, &events).await?;
    remote
        .close()
        .await
        .map_err(|error| format!("Could not close {remote_path}: {error}"))?;
    let _ = events.send(SshEvent::TransferDone {
        transfer_id,
        total_bytes: total_bytes as f64,
    });
    Ok(())
}

async fn download_file(
    sftp: Arc<SftpSession>,
    transfer_id: u64,
    remote_path: String,
    local_path: String,
    total_bytes: u64,
    events: Sender<SshEvent>,
) -> Result<(), String> {
    let mut remote = sftp
        .open(remote_path.clone())
        .await
        .map_err(|error| format!("Could not open {remote_path}: {error}"))?;
    let resolved = local_fs::resolve(&local_path);
    if let Some(parent) = resolved.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
    }
    let mut local = tokio::fs::File::create(&resolved)
        .await
        .map_err(|error| format!("Could not create {}: {error}", resolved.display()))?;
    copy_with_progress(&mut remote, &mut local, transfer_id, total_bytes, &events).await?;
    local
        .flush()
        .await
        .map_err(|error| format!("Could not flush {}: {error}", resolved.display()))?;
    let _ = events.send(SshEvent::TransferDone {
        transfer_id,
        total_bytes: total_bytes as f64,
    });
    Ok(())
}

async fn copy_between_remote(
    source: Arc<SftpSession>,
    destination: Arc<SftpSession>,
    transfer_id: u64,
    source_path: String,
    destination_path: String,
    total_bytes: u64,
    events: Sender<SshEvent>,
) -> Result<(), String> {
    let mut source_file = source
        .open(source_path.clone())
        .await
        .map_err(|error| format!("Could not open {source_path}: {error}"))?;
    let mut destination_file = destination
        .create(destination_path.clone())
        .await
        .map_err(|error| format!("Could not create {destination_path}: {error}"))?;
    copy_with_progress(
        &mut source_file,
        &mut destination_file,
        transfer_id,
        total_bytes,
        &events,
    )
    .await?;
    destination_file
        .close()
        .await
        .map_err(|error| format!("Could not close {destination_path}: {error}"))?;
    let _ = events.send(SshEvent::TransferDone {
        transfer_id,
        total_bytes: total_bytes as f64,
    });
    Ok(())
}

async fn copy_with_progress<R, W>(
    reader: &mut R,
    writer: &mut W,
    transfer_id: u64,
    total_bytes: u64,
    events: &Sender<SshEvent>,
) -> Result<(), String>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut transferred = 0_u64;
    let started = Instant::now();
    let mut last_report = Instant::now();
    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|error| format!("Transfer read failed: {error}"))?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buffer[..read])
            .await
            .map_err(|error| format!("Transfer write failed: {error}"))?;
        transferred += read as u64;
        if last_report.elapsed() >= Duration::from_millis(120) {
            last_report = Instant::now();
            let elapsed = started.elapsed().as_secs_f64().max(0.001);
            let _ = events.send(SshEvent::TransferProgress {
                transfer_id,
                transferred_bytes: transferred as f64,
                total_bytes: total_bytes.max(transferred) as f64,
                speed_bps: transferred as f64 / elapsed,
            });
        }
    }
    let _ = events.send(SshEvent::TransferProgress {
        transfer_id,
        transferred_bytes: transferred as f64,
        total_bytes: total_bytes.max(transferred) as f64,
        speed_bps: 0.0,
    });
    Ok(())
}

fn remote_entry(entry: russh_sftp::client::fs::DirEntry) -> FileEntry {
    let name = entry.file_name();
    let metadata = entry.metadata();
    let permissions = metadata.permissions();
    let type_prefix = if metadata.is_dir() {
        'd'
    } else if metadata.is_symlink() {
        'l'
    } else {
        '-'
    };
    let owner = match (&metadata.user, &metadata.group) {
        (Some(user), Some(group)) => format!("{user}:{group}"),
        (Some(user), None) => user.clone(),
        (None, Some(group)) => (metadata.uid, Some(group.clone()))
            .0
            .map(|uid| format!("{uid}:{group}"))
            .unwrap_or_else(|| format!(":{}", group)),
        (None, None) => match (metadata.uid, metadata.gid) {
            (Some(uid), Some(gid)) => format!("{uid}:{gid}"),
            (Some(uid), None) => uid.to_string(),
            _ => "-".to_owned(),
        },
    };
    FileEntry {
        archive: is_archive(&name),
        name,
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        modified: metadata
            .modified()
            .ok()
            .map(local_fs::modified_label)
            .unwrap_or_else(|| "--".to_owned()),
        perm: format!("{type_prefix}{permissions}"),
        owner,
        symlink: metadata.is_symlink(),
    }
}

fn is_archive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".zip", ".tar", ".gz", ".tgz", ".bz2", ".xz", ".7z", ".rar"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

pub fn md5_bytes(bytes: &[u8]) -> String {
    use md5::{Digest, Md5};
    let digest = Md5::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn md5_local(path: &str) -> Result<String, String> {
    let resolved = local_fs::resolve(path);
    let mut file = std::fs::File::open(&resolved)
        .map_err(|error| format!("Could not read {}: {error}", resolved.display()))?;
    use md5::{Digest, Md5};
    use std::io::Read;
    let mut hasher = Md5::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not hash {}: {error}", resolved.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

impl Drop for SshManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn host_key_name(host: &Host) -> String {
    format!("{}:{}", host.ip, host.port)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use russh::server::{self, Msg, Session as ServerSession};
    use russh::{Channel, ChannelId};

    use super::*;
    use crate::domain::{AuthKind, ConnectionBackend, HostStatus, KeySource};

    struct EchoServer;

    impl server::Handler for EchoServer {
        type Error = russh::Error;

        async fn auth_password(
            &mut self,
            _user: &str,
            _password: &str,
        ) -> Result<server::Auth, Self::Error> {
            Ok(server::Auth::Accept)
        }

        async fn channel_open_session(
            &mut self,
            _channel: Channel<Msg>,
            reply: server::ChannelOpenHandle,
            _session: &mut ServerSession,
        ) -> Result<(), Self::Error> {
            reply.accept().await;
            Ok(())
        }

        async fn pty_request(
            &mut self,
            channel: ChannelId,
            _term: &str,
            _col_width: u32,
            _row_height: u32,
            _pix_width: u32,
            _pix_height: u32,
            _modes: &[(russh::Pty, u32)],
            session: &mut ServerSession,
        ) -> Result<(), Self::Error> {
            let _ = session.channel_success(channel);
            Ok(())
        }

        async fn shell_request(
            &mut self,
            channel: ChannelId,
            session: &mut ServerSession,
        ) -> Result<(), Self::Error> {
            let _ = session.channel_success(channel);
            session.data(channel, b"welcome to rshell test\r\n".to_vec())?;
            Ok(())
        }

        async fn data(
            &mut self,
            channel: ChannelId,
            data: &[u8],
            session: &mut ServerSession,
        ) -> Result<(), Self::Error> {
            session.data(channel, data.to_vec())?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn terminal_pipeline_connects_and_streams_output() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let config = rudimentary_server_config();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = server::run_stream(config, stream, EchoServer).await;
        });

        let mut manager = SshManager::new(HashMap::new());
        manager
            .open_terminal(
                1,
                test_host(addr),
                SshAuthMaterial::Password("secret".to_owned()),
                80,
                24,
            )
            .unwrap();

        let mut connected = false;
        let mut output = Vec::new();
        for _ in 0..80 {
            tokio::time::sleep(Duration::from_millis(25)).await;
            for event in manager.drain_events() {
                match event {
                    SshEvent::Connected { .. } => connected = true,
                    SshEvent::TerminalData { data, .. } => output.extend(data),
                    SshEvent::ConnectFailed { message, .. } => panic!("{message}"),
                    _ => {}
                }
            }
            if connected && String::from_utf8_lossy(&output).contains("welcome") {
                break;
            }
        }
        assert!(connected, "SSH connection did not complete");
        assert!(
            String::from_utf8_lossy(&output).contains("welcome to rshell test"),
            "terminal output was not forwarded"
        );
        manager.close_tab(1);
    }

    #[derive(Default)]
    struct SftpTestServer {
        clients: Arc<tokio::sync::Mutex<HashMap<ChannelId, Channel<Msg>>>>,
    }

    impl server::Handler for SftpTestServer {
        type Error = russh::Error;

        async fn auth_password(
            &mut self,
            _user: &str,
            _password: &str,
        ) -> Result<server::Auth, Self::Error> {
            Ok(server::Auth::Accept)
        }

        async fn channel_open_session(
            &mut self,
            channel: Channel<Msg>,
            reply: server::ChannelOpenHandle,
            _session: &mut ServerSession,
        ) -> Result<(), Self::Error> {
            self.clients.lock().await.insert(channel.id(), channel);
            reply.accept().await;
            Ok(())
        }

        async fn subsystem_request(
            &mut self,
            channel_id: ChannelId,
            name: &str,
            session: &mut ServerSession,
        ) -> Result<(), Self::Error> {
            if name != "sftp" {
                let _ = session.channel_failure(channel_id);
                return Ok(());
            }
            let Some(channel) = self.clients.lock().await.remove(&channel_id) else {
                let _ = session.channel_failure(channel_id);
                return Ok(());
            };
            let _ = session.channel_success(channel_id);
            russh_sftp::server::run(channel.into_stream(), TestSftpFs::default()).await;
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestSftpFs {
        listed: bool,
    }

    impl russh_sftp::server::Handler for TestSftpFs {
        type Error = russh_sftp::protocol::StatusCode;

        fn unimplemented(&self) -> Self::Error {
            russh_sftp::protocol::StatusCode::OpUnsupported
        }

        async fn init(
            &mut self,
            _version: u32,
            _extensions: HashMap<String, String>,
        ) -> Result<russh_sftp::protocol::Version, Self::Error> {
            Ok(russh_sftp::protocol::Version::new())
        }

        async fn realpath(
            &mut self,
            id: u32,
            _path: String,
        ) -> Result<russh_sftp::protocol::Name, Self::Error> {
            Ok(russh_sftp::protocol::Name {
                id,
                files: vec![russh_sftp::protocol::File::dummy("/")],
            })
        }

        async fn opendir(
            &mut self,
            id: u32,
            path: String,
        ) -> Result<russh_sftp::protocol::Handle, Self::Error> {
            self.listed = false;
            Ok(russh_sftp::protocol::Handle { id, handle: path })
        }

        async fn readdir(
            &mut self,
            id: u32,
            _handle: String,
        ) -> Result<russh_sftp::protocol::Name, Self::Error> {
            if self.listed {
                return Err(russh_sftp::protocol::StatusCode::Eof);
            }
            self.listed = true;
            let mut attrs = FileAttributes::empty();
            attrs.size = Some(12);
            attrs.permissions = Some(0o100644);
            attrs.uid = Some(1000);
            attrs.gid = Some(1000);
            Ok(russh_sftp::protocol::Name {
                id,
                files: vec![russh_sftp::protocol::File::new("hello.txt", attrs)],
            })
        }

        async fn close(
            &mut self,
            id: u32,
            _handle: String,
        ) -> Result<russh_sftp::protocol::Status, Self::Error> {
            Ok(russh_sftp::protocol::Status {
                id,
                status_code: russh_sftp::protocol::StatusCode::Ok,
                error_message: "Ok".to_owned(),
                language_tag: "en-US".to_owned(),
            })
        }
    }

    #[tokio::test]
    async fn sftp_pipeline_lists_remote_directory() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let config = rudimentary_server_config();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = server::run_stream(config, stream, SftpTestServer::default()).await;
        });

        let mut manager = SshManager::new(HashMap::new());
        manager
            .open_sftp(
                2,
                test_host(addr),
                SshAuthMaterial::Password("secret".to_owned()),
            )
            .unwrap();

        let mut connected = false;
        let mut entries = Vec::new();
        for _ in 0..80 {
            tokio::time::sleep(Duration::from_millis(25)).await;
            for event in manager.drain_events() {
                match event {
                    SshEvent::Connected { .. } => connected = true,
                    SshEvent::RemoteEntries {
                        entries: listed, ..
                    } => entries = listed,
                    SshEvent::ConnectFailed { message, .. }
                    | SshEvent::RemoteError { message, .. } => panic!("{message}"),
                    _ => {}
                }
            }
            if connected && !entries.is_empty() {
                break;
            }
        }
        assert!(connected, "SFTP SSH connection did not complete");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "hello.txt");
        assert_eq!(entries[0].size, 12);
        manager.close_tab(2);
    }

    fn rudimentary_server_config() -> Arc<server::Config> {
        Arc::new(server::Config {
            keys: vec![russh::keys::PrivateKey::random(
                &mut rand::rng(),
                russh::keys::Algorithm::Ed25519,
            )
            .unwrap()],
            ..Default::default()
        })
    }

    #[test]
    fn md5_matches_known_vector() {
        assert_eq!(md5_bytes(b"hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    fn test_host(address: SocketAddr) -> Host {
        Host {
            id: 1,
            folder_id: 1,
            name: "test".to_owned(),
            ip: address.ip().to_string(),
            port: address.port(),
            user: "tester".to_owned(),
            auth: AuthKind::Password,
            key_source: KeySource::FileSystem,
            key_id: None,
            secret: "secret".to_owned(),
            gateway: String::new(),
            environment: String::new(),
            keep_alive: 0,
            backend: ConnectionBackend::Ssh,
            os: crate::domain::HostOs::Unknown,
            wsl_distro: None,
            status: HostStatus::Offline,
            latency_ms: None,
            rdp_mode: Default::default(),
            credentials_unavailable: false,
        }
    }
}
