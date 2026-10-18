use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use ironrdp_client::config::{ClipboardType, ConfigBuilder, Destination};
use ironrdp_client::output_channel::output_channel;
use ironrdp_client::rdp::{RdpClient, RdpInputEvent, RdpInputSender, RdpOutputEvent};
use ironrdp_pdu::input::fast_path::FastPathInputEvent;
use smallvec::SmallVec;

use crate::domain::Host;

pub mod clipboard;
pub mod keyboard_capture;

pub const DEFAULT_DESKTOP_WIDTH: u16 = 1280;
pub const DEFAULT_DESKTOP_HEIGHT: u16 = 800;

#[derive(Clone, Debug)]
pub struct RdpFrame {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u32>,
}

#[derive(Clone, Debug)]
pub enum RdpEvent {
    Connected {
        tab_id: u64,
        host_id: u64,
    },
    Frame {
        tab_id: u64,
        frame: Arc<RdpFrame>,
    },
    Failed {
        tab_id: u64,
        host_id: u64,
        message: String,
    },
    Closed {
        tab_id: u64,
        host_id: u64,
        message: String,
    },
}

struct RdpSession {
    host_id: u64,
    input: RdpInputSender,
    frame: Option<Arc<RdpFrame>>,
    texture: Option<TextureHandle>,
    texture_version: u64,
    frame_version: u64,
    requested_size: Option<(u16, u16)>,
    error: Option<String>,
    #[cfg(windows)]
    _clipboard_host: Option<clipboard::ClipboardHost>,
}

pub struct RdpManager {
    sessions: HashMap<u64, RdpSession>,
    errors: HashMap<u64, String>,
    events: Receiver<RdpEvent>,
    event_tx: Sender<RdpEvent>,
}

impl RdpManager {
    pub fn new() -> Self {
        let (event_tx, events) = mpsc::channel();
        Self {
            sessions: HashMap::new(),
            errors: HashMap::new(),
            events,
            event_tx,
        }
    }

    pub fn open(
        &mut self,
        tab_id: u64,
        host: &Host,
        desktop_width: u16,
        desktop_height: u16,
    ) -> Result<(), String> {
        self.open_with(tab_id, host, desktop_width, desktop_height, true)
    }

    pub fn open_compat(
        &mut self,
        tab_id: u64,
        host: &Host,
        desktop_width: u16,
        desktop_height: u16,
    ) -> Result<(), String> {
        self.open_with(tab_id, host, desktop_width, desktop_height, false)
    }

    fn open_with(
        &mut self,
        tab_id: u64,
        host: &Host,
        desktop_width: u16,
        desktop_height: u16,
        graphics_pipeline: bool,
    ) -> Result<(), String> {
        let destination = Destination::from_parts(
            host.ip.clone(),
            if host.port == 0 { 3389 } else { host.port },
        );
        let mut builder = ConfigBuilder::new()
            .with_destination(destination)
            .with_client_build(0)
            .with_client_dir(std::env::temp_dir().display().to_string())
            .with_client_name("RShell")
            .with_platform(ironrdp_pdu::rdp::capability_sets::MajorPlatformType::WINDOWS)
            .with_username(host.user.clone())
            .with_password(host.secret.clone())
            .with_desktop_width(desktop_width.max(640))
            .with_desktop_height(desktop_height.max(480))
            .with_desktop_scale_factor(100)
            .with_credssp(true)
            .with_tls(true)
            .with_server_pointer(true)
            .with_pointer_software_rendering(true)
            .with_compression(true)
            .with_clipboard(ClipboardType::Enable)
            .with_rdpdr(true)
            .with_autologon(true)
            // KRDP (KDE Plasma) closes the session during capability exchange
            // when the client does not advertise the Graphics Pipeline extension.
            .with_graphics_pipeline(false);
        let domain = host.gateway.trim();
        if !domain.is_empty() && !domain.eq_ignore_ascii_case("direct") {
            builder = builder.with_domain(domain);
        }
        let config = match builder.build() {
            Ok(config) => config,
            Err(error) => {
                let message = format!("RDP configuration error: {error}");
                self.set_error(tab_id, message.clone());
                return Err(message);
            }
        };
        let (output_tx, mut output_rx) = output_channel(64);
        let client = RdpClient::new(config, output_tx);
        #[cfg(windows)]
        let client = attach_drive_redirection(client);
        let input = client.input_sender();
        #[cfg(windows)]
        let (client, clipboard_host) = match clipboard::start(input.clone()) {
            Ok((factory, host)) => (client.with_cliprdr_backend_factory(factory), Some(host)),
            Err(error) => {
                eprintln!("{error}");
                (client, None)
            }
        };
        let host_id = host.id;
        let event_tx = self.event_tx.clone();
        std::thread::Builder::new()
            .name(format!("rshell-rdp-{tab_id}"))
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = event_tx.send(RdpEvent::Failed {
                            tab_id,
                            host_id,
                            message: format!("Could not start RDP runtime: {error}"),
                        });
                        return;
                    }
                };
                runtime.block_on(async move {
                    let run = client.run();
                    tokio::pin!(run);
                    loop {
                        tokio::select! {
                            _ = &mut run => break,
                            event = output_rx.recv() => {
                                match event {
                                    Some(event) => {
                                        if !forward_output(&event_tx, event, tab_id, host_id) {
                                            return;
                                        }
                                    }
                                    None => break,
                                }
                            }
                        }
                    }
                    while let Some(event) = output_rx.recv().await {
                        if !forward_output(&event_tx, event, tab_id, host_id) {
                            break;
                        }
                    }
                });
            })
            .map_err(|error| format!("Could not start RDP worker: {error}"))?;

        self.sessions.insert(
            tab_id,
            RdpSession {
                host_id,
                input,
                frame: None,
                texture: None,
                texture_version: 0,
                frame_version: 0,
                requested_size: Some((desktop_width.max(640), desktop_height.max(480))),
                error: None,
                #[cfg(windows)]
                _clipboard_host: clipboard_host,
            },
        );
        Ok(())
    }

    pub fn drain_events(&self) -> Vec<RdpEvent> {
        self.events.try_iter().collect()
    }

    pub fn frame(&self, tab_id: u64) -> Option<Arc<RdpFrame>> {
        self.sessions
            .get(&tab_id)
            .and_then(|session| session.frame.clone())
    }

    pub fn texture(&mut self, ctx: &egui::Context, tab_id: u64) -> Option<TextureHandle> {
        let session = self.sessions.get_mut(&tab_id)?;
        let frame = session.frame.clone()?;
        if session.texture.is_none() || session.texture_version != session.frame_version {
            let pixels = frame
                .pixels
                .iter()
                .map(|pixel| {
                    Color32::from_rgb(
                        ((pixel >> 16) & 0xff) as u8,
                        ((pixel >> 8) & 0xff) as u8,
                        (pixel & 0xff) as u8,
                    )
                })
                .collect();
            let image = ColorImage::new(
                [usize::from(frame.width), usize::from(frame.height)],
                pixels,
            );
            if let Some(texture) = &mut session.texture {
                texture.set(image, TextureOptions::NEAREST);
            } else {
                session.texture =
                    Some(ctx.load_texture(format!("rdp-{tab_id}"), image, TextureOptions::NEAREST));
            }
            session.texture_version = session.frame_version;
        }
        session.texture.clone()
    }

    pub fn store_frame(&mut self, tab_id: u64, frame: Arc<RdpFrame>) {
        self.errors.remove(&tab_id);
        if let Some(session) = self.sessions.get_mut(&tab_id) {
            session.frame = Some(frame);
            session.frame_version = session.frame_version.wrapping_add(1);
            session.error = None;
        }
    }

    pub fn set_error(&mut self, tab_id: u64, error: String) {
        self.errors.insert(tab_id, error.clone());
        if let Some(session) = self.sessions.get_mut(&tab_id) {
            session.error = Some(error);
        }
    }

    pub fn error(&self, tab_id: u64) -> Option<&str> {
        self.errors.get(&tab_id).map(String::as_str).or_else(|| {
            self.sessions
                .get(&tab_id)
                .and_then(|session| session.error.as_deref())
        })
    }

    pub fn send_input(&self, tab_id: u64, events: SmallVec<[FastPathInputEvent; 2]>) -> bool {
        self.sessions.get(&tab_id).is_some_and(|session| {
            session
                .input
                .try_send(RdpInputEvent::FastPath(events))
                .is_ok()
        })
    }

    pub fn resize_to_view(
        &mut self,
        tab_id: u64,
        width: u16,
        height: u16,
        scale_factor: u32,
    ) -> bool {
        let Some(session) = self.sessions.get_mut(&tab_id) else {
            return false;
        };
        let width = width.max(200) & !1;
        let height = height.max(200);
        if let Some((requested_width, requested_height)) = session.requested_size {
            if requested_width.abs_diff(width) <= 4 && requested_height.abs_diff(height) <= 4 {
                return false;
            }
        }
        let sent = session
            .input
            .try_send(RdpInputEvent::Resize {
                width,
                height,
                scale_factor,
                physical_size: None,
            })
            .is_ok();
        if sent {
            session.requested_size = Some((width, height));
        }
        sent
    }

    pub fn last_size(&self, tab_id: u64) -> Option<(u16, u16)> {
        self.sessions
            .get(&tab_id)
            .and_then(|session| session.requested_size)
    }

    pub fn close_tab(&mut self, tab_id: u64) {
        self.errors.remove(&tab_id);
        if let Some(session) = self.sessions.remove(&tab_id) {
            session.input.request_close();
        }
    }

    pub fn forget_session(&mut self, tab_id: u64) {
        self.errors.remove(&tab_id);
        self.sessions.remove(&tab_id);
    }

    pub fn session_count_for_host(&self, host_id: u64) -> usize {
        self.sessions
            .values()
            .filter(|session| session.host_id == host_id)
            .count()
    }

    pub fn has_sessions(&self) -> bool {
        self.sessions
            .values()
            .any(|session| session.error.is_none())
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
}

#[cfg(windows)]
fn attach_drive_redirection(client: RdpClient) -> RdpClient {
    let drives = fixed_volume_roots()
        .into_iter()
        .enumerate()
        .filter_map(|(index, (letter, root))| {
            match ironrdp_rdpdr_native::RedirectedDrive::new(
                index as u32 + 1,
                format!("RShell {letter}"),
                root,
                false,
            ) {
                Ok(drive) => Some(drive),
                Err(error) => {
                    eprintln!("Could not share local drive {letter}: {error}");
                    None
                }
            }
        })
        .collect::<Vec<_>>();
    if drives.is_empty() {
        return client;
    }
    let factory = match ironrdp_rdpdr_native::WindowsRdpdrBackendFactory::from_drives(drives) {
        Ok(factory) => factory,
        Err(error) => {
            eprintln!("Could not configure RDP drive redirection: {error}");
            return client;
        }
    };
    client.with_rdpdr_backend_factory(Box::new(factory))
}

#[cfg(windows)]
fn fixed_volume_roots() -> Vec<(char, String)> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    use windows::Win32::System::WindowsProgramming::DRIVE_FIXED;

    let mut roots = Vec::new();
    for letter in b'C'..=b'Z' {
        let letter = char::from(letter);
        let root = format!("{letter}:\\");
        let wide = std::ffi::OsStr::new(&root)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let kind = unsafe { GetDriveTypeW(windows::core::PCWSTR::from_raw(wide.as_ptr())) };
        if kind == DRIVE_FIXED && std::fs::read_dir(&root).is_ok() {
            roots.push((letter, root));
        }
    }
    roots
}

fn forward_output(
    event_tx: &Sender<RdpEvent>,
    event: RdpOutputEvent,
    tab_id: u64,
    host_id: u64,
) -> bool {
    let event = match event {
        RdpOutputEvent::Connected => RdpEvent::Connected { tab_id, host_id },
        RdpOutputEvent::Image {
            buffer,
            width,
            height,
        } => RdpEvent::Frame {
            tab_id,
            frame: Arc::new(RdpFrame {
                width: width.get(),
                height: height.get(),
                pixels: buffer,
            }),
        },
        RdpOutputEvent::ConnectionFailure(error) => RdpEvent::Failed {
            tab_id,
            host_id,
            message: describe_connection_failure(&error),
        },
        RdpOutputEvent::Terminated(result) => RdpEvent::Closed {
            tab_id,
            host_id,
            message: match result {
                Ok(reason) => format!("RDP session ended: {reason:?}"),
                Err(error) => format!("RDP session error: {}", describe_error(&error)),
            },
        },
        _ => return true,
    };
    event_tx.send(event).is_ok()
}

fn describe_error(error: &dyn std::error::Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

fn describe_connection_failure(error: &impl std::fmt::Debug) -> String {
    let detail = format!("{error:?}");
    if detail.contains("0xc000006d") {
        "RDP logon failed: invalid username or password.".to_owned()
    } else if detail.contains("received disconnect provider ultimatum")
        || detail.contains("decode_send_data_indication")
    {
        "RDP server closed the connection during negotiation. This is often a protocol/capability mismatch (KRDP requires Graphics Pipeline support), not necessarily a wrong password.".to_owned()
    } else {
        format!("RDP connection failed: {error:?}")
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::fixed_volume_roots;

    #[test]
    fn enumerates_accessible_fixed_volumes_for_redirection() {
        let roots = fixed_volume_roots();
        assert!(
            !roots.is_empty(),
            "a Windows host should expose at least one accessible fixed volume"
        );
        assert!(roots.iter().all(|(_, root)| root.ends_with(":\\")));
    }
}
