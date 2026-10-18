#[cfg(windows)]
mod platform {
    use std::io::{Read as _, Seek as _, SeekFrom};
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use ironrdp_client::rdp::{CliprdrBackendFactory, RdpInputSender};
    use ironrdp_cliprdr::backend::{ClipboardMessage, ClipboardMessageProxy, CliprdrBackend};
    use ironrdp_cliprdr::pdu::{
        ClipboardFileAttributes, ClipboardFormat, ClipboardFormatId, ClipboardGeneralCapabilityFlags,
        FileContentsFlags, FileContentsRequest, FileContentsResponse, FileDescriptor, FormatDataRequest,
        FormatDataResponse, LockDataId,
    };
    use ironrdp_cliprdr_native::WinClipboard;
    use ironrdp_core::impl_as_any;
    use windows::core::BOOL;
    use windows::Win32::Foundation::{FALSE, HGLOBAL, HWND, LPARAM, TRUE, WPARAM};
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EnumThreadWindows, GetClassNameW, GetMessageW, PostMessageW,
        PostThreadMessageW, TranslateMessage, MSG, WM_ACTIVATEAPP, WM_QUIT,
    };

    type FileEntry = (FileDescriptor, PathBuf);

    #[derive(Debug, Default)]
    struct FileState {
        ready: bool,
        files: Vec<FileEntry>,
    }

    type SharedFiles = Arc<Mutex<FileState>>;

    #[cfg(debug_assertions)]
    fn probe_log(message: &str) {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("rshell-clipboard.log"))
        {
            let _ = writeln!(file, "{message}");
        }
    }

    fn message_kind(message: &ClipboardMessage) -> String {
        match message {
            ClipboardMessage::SendInitiateCopy(formats) => {
                format!("SendInitiateCopy({} formats)", formats.len())
            }
            ClipboardMessage::SendFormatData(_) => "SendFormatData".to_owned(),
            ClipboardMessage::SendInitiatePaste(_) => "SendInitiatePaste".to_owned(),
            ClipboardMessage::SendFileContentsRequest(_) => "SendFileContentsRequest".to_owned(),
            ClipboardMessage::SendFileContentsResponse(_) => "SendFileContentsResponse".to_owned(),
            ClipboardMessage::SendInitiateFileCopy(files) => {
                format!("SendInitiateFileCopy({} files)", files.len())
            }
            ClipboardMessage::Error(_) => "Error".to_owned(),
        }
    }

    #[derive(Debug, Clone)]
    struct ClipboardProxy {
        input: RdpInputSender,
        files: SharedFiles,
    }

    impl ClipboardProxy {
        fn send(&self, message: ClipboardMessage) {
            #[cfg(debug_assertions)]
            probe_log(&message_kind(&message));
            if self.input.send_clipboard(message).is_err() {
                eprintln!("RDP clipboard channel closed");
            }
        }
    }

    impl ClipboardMessageProxy for ClipboardProxy {
        fn send_clipboard_message(&self, message: ClipboardMessage) {
            #[cfg(debug_assertions)]
            if let ClipboardMessage::SendInitiateCopy(formats) = &message {
                probe_log(&format!(
                    "formats: {:?}",
                    formats
                        .iter()
                        .map(|format| format.id().value())
                        .collect::<Vec<_>>()
                ));
            }
            if let ClipboardMessage::SendInitiateCopy(formats) = &message {
                if formats
                    .iter()
                    .any(|format| format.id() == ClipboardFormatId::CF_HDROP)
                {
                    let entries = read_clipboard_files();
                    if !entries.is_empty() {
                        let (ready, descriptors) = {
                            let mut state = self.files.lock().unwrap_or_else(|e| e.into_inner());
                            state.files = entries;
                            (
                                state.ready,
                                state
                                    .files
                                    .iter()
                                    .map(|(descriptor, _)| descriptor.clone())
                                    .collect::<Vec<_>>(),
                            )
                        };
                        if ready {
                            self.send(ClipboardMessage::SendInitiateFileCopy(descriptors));
                            return;
                        }
                        let filtered = formats
                            .iter()
                            .filter(|format| format.id() != ClipboardFormatId::CF_HDROP)
                            .cloned()
                            .collect::<Vec<_>>();
                        self.send(ClipboardMessage::SendInitiateCopy(filtered));
                        return;
                    }
                }
            }
            self.send(message);
        }
    }

    fn read_clipboard_files() -> Vec<FileEntry> {
        unsafe {
            let mut opened = false;
            for _ in 0..10 {
                if OpenClipboard(None).is_ok() {
                    opened = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            if !opened {
                return Vec::new();
            }
            let Ok(handle) = GetClipboardData(ClipboardFormatId::CF_HDROP.value())
            else {
                let _ = CloseClipboard();
                return Vec::new();
            };
            let hglobal = HGLOBAL(handle.0);
            let pointer = GlobalLock(hglobal);
            if pointer.is_null() {
                let _ = CloseClipboard();
                return Vec::new();
            }
            let size = GlobalSize(hglobal);
            let data = std::slice::from_raw_parts(pointer.cast::<u8>(), size);
            let paths = parse_drop_files(data);
            let _ = GlobalUnlock(hglobal);
            let _ = CloseClipboard();
            paths
                .into_iter()
                .filter_map(|path| file_entry(path))
                .collect()
        }
    }

    fn parse_drop_files(data: &[u8]) -> Vec<PathBuf> {
        if data.len() < 20 {
            return Vec::new();
        }
        let offset = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let wide = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) != 0;
        let mut paths = Vec::new();
        if wide {
            let mut current = Vec::new();
            let mut index = offset;
            while index + 2 <= data.len() {
                let unit = u16::from_le_bytes([data[index], data[index + 1]]);
                index += 2;
                if unit == 0 {
                    if current.is_empty() {
                        break;
                    }
                    paths.push(PathBuf::from(String::from_utf16_lossy(&current)));
                    current.clear();
                } else {
                    current.push(unit);
                }
            }
        } else {
            let mut current = Vec::new();
            for &byte in data.iter().skip(offset) {
                if byte == 0 {
                    if current.is_empty() {
                        break;
                    }
                    paths.push(PathBuf::from(String::from_utf8_lossy(&current).into_owned()));
                    current.clear();
                } else {
                    current.push(byte);
                }
            }
        }
        paths
    }

    fn file_entry(path: PathBuf) -> Option<FileEntry> {
        let metadata = std::fs::metadata(&path).ok()?;
        if metadata.is_dir() {
            return None;
        }
        let name = path.file_name()?.to_string_lossy().into_owned();
        let mut attributes = ClipboardFileAttributes::ARCHIVE;
        if metadata.permissions().readonly() {
            attributes |= ClipboardFileAttributes::READONLY;
        }
        let mut descriptor = FileDescriptor::new(name)
            .with_attributes(attributes)
            .with_file_size(metadata.len());
        if let Some(write_time) = metadata
            .modified()
            .ok()
            .and_then(system_time_to_filetime)
        {
            descriptor = descriptor.with_last_write_time(write_time);
        }
        Some((descriptor, path))
    }

    fn system_time_to_filetime(time: SystemTime) -> Option<u64> {
        let since_epoch = time.duration_since(UNIX_EPOCH).ok()?;
        Some(
            since_epoch.as_secs() * 10_000_000
                + u64::from(since_epoch.subsec_nanos()) / 100
                + 116_444_736_000_000_000,
        )
    }

    fn serve_file(request: &FileContentsRequest, files: &[FileEntry]) -> FileContentsResponse<'static> {
        let Ok(index) = usize::try_from(request.index) else {
            return FileContentsResponse::new_error(request.stream_id);
        };
        let Some((_, path)) = files.get(index) else {
            return FileContentsResponse::new_error(request.stream_id);
        };
        let Ok(metadata) = std::fs::metadata(path) else {
            return FileContentsResponse::new_error(request.stream_id);
        };
        if metadata.is_dir() {
            return FileContentsResponse::new_error(request.stream_id);
        }
        if request.flags.contains(FileContentsFlags::SIZE) {
            return FileContentsResponse::new_size_response(request.stream_id, metadata.len());
        }
        const MAX_CHUNK_BYTES: u32 = 4 * 1024 * 1024;
        let read_len = request.requested_size.min(MAX_CHUNK_BYTES);
        let Ok(mut file) = std::fs::File::open(path) else {
            return FileContentsResponse::new_error(request.stream_id);
        };
        if file.seek(SeekFrom::Start(request.position)).is_err() {
            return FileContentsResponse::new_error(request.stream_id);
        }
        let mut buffer = Vec::new();
        if file.take(u64::from(read_len)).read_to_end(&mut buffer).is_err() {
            return FileContentsResponse::new_error(request.stream_id);
        }
        FileContentsResponse::new_data_response(request.stream_id, buffer)
    }

    #[derive(Debug)]
    struct FileBackend {
        inner: Box<dyn CliprdrBackend>,
        proxy: ClipboardProxy,
        files: SharedFiles,
    }

    impl_as_any!(FileBackend);

    impl CliprdrBackend for FileBackend {
        fn temporary_directory(&self) -> &str {
            self.inner.temporary_directory()
        }

        fn client_capabilities(&self) -> ClipboardGeneralCapabilityFlags {
            self.inner.client_capabilities()
                | ClipboardGeneralCapabilityFlags::STREAM_FILECLIP_ENABLED
        }

        fn on_ready(&mut self) {
            self.inner.on_ready();
            let descriptors = {
                let mut state = self.files.lock().unwrap_or_else(|e| e.into_inner());
                state.ready = true;
                state
                    .files
                    .iter()
                    .map(|(descriptor, _)| descriptor.clone())
                    .collect::<Vec<_>>()
            };
            if !descriptors.is_empty() {
                self.proxy
                    .send(ClipboardMessage::SendInitiateFileCopy(descriptors));
            }
        }

        fn on_request_format_list(&mut self) {
            self.inner.on_request_format_list();
        }

        fn on_format_list_response(&mut self, ok: bool) {
            self.inner.on_format_list_response(ok);
        }

        fn on_process_negotiated_capabilities(
            &mut self,
            capabilities: ClipboardGeneralCapabilityFlags,
        ) {
            self.inner.on_process_negotiated_capabilities(capabilities);
        }

        fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
            self.inner.on_remote_copy(available_formats);
        }

        fn on_format_data_request(&mut self, request: FormatDataRequest) {
            self.inner.on_format_data_request(request);
        }

        fn on_format_data_response(&mut self, response: FormatDataResponse<'_>) {
            self.inner.on_format_data_response(response);
        }

        fn on_file_contents_request(&mut self, request: FileContentsRequest) {
            let response = {
                let state = self.files.lock().unwrap_or_else(|e| e.into_inner());
                serve_file(&request, &state.files)
            };
            self.proxy
                .send(ClipboardMessage::SendFileContentsResponse(response));
        }

        fn on_file_contents_response(&mut self, response: FileContentsResponse<'_>) {
            self.inner.on_file_contents_response(response);
        }

        fn on_lock(&mut self, data_id: LockDataId) {
            self.inner.on_lock(data_id);
        }

        fn on_unlock(&mut self, data_id: LockDataId) {
            self.inner.on_unlock(data_id);
        }

        fn now_ms(&self) -> u64 {
            self.inner.now_ms()
        }
    }

    struct FileFactory {
        inner: Box<dyn CliprdrBackendFactory + Send>,
        proxy: ClipboardProxy,
        files: SharedFiles,
    }

    impl CliprdrBackendFactory for FileFactory {
        fn build_cliprdr_backend(&self) -> Box<dyn CliprdrBackend> {
            Box::new(FileBackend {
                inner: self.inner.build_cliprdr_backend(),
                proxy: self.proxy.clone(),
                files: Arc::clone(&self.files),
            })
        }
    }

    pub struct ClipboardHost {
        thread_id: u32,
    }

    unsafe extern "system" fn find_clipboard_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let target = unsafe { &mut *(lparam.0 as *mut Option<HWND>) };
        let mut class = [0u16; 64];
        let len = unsafe { GetClassNameW(hwnd, &mut class) };
        if len > 0
            && String::from_utf16_lossy(&class[..len as usize]) == "IronRDPClipboardMonitor"
        {
            *target = Some(hwnd);
            return FALSE;
        }
        TRUE
    }

    fn force_inactive_clipboard_window(thread_id: u32) {
        let mut target: Option<HWND> = None;
        unsafe {
            let _ = EnumThreadWindows(
                thread_id,
                Some(find_clipboard_window),
                LPARAM(&mut target as *mut _ as isize),
            );
        }
        if let Some(hwnd) = target {
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_ACTIVATEAPP, WPARAM(0), LPARAM(0));
            }
        }
    }

    impl Drop for ClipboardHost {
        fn drop(&mut self) {
            #[cfg(debug_assertions)]
            probe_log(&format!("clipboard host drop thread={}", self.thread_id));
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }

    pub fn start(
        input: RdpInputSender,
    ) -> Result<(Box<dyn CliprdrBackendFactory + Send>, ClipboardHost), String> {
        let files: SharedFiles = Arc::new(Mutex::new(FileState::default()));
        let proxy = ClipboardProxy {
            input,
            files: Arc::clone(&files),
        };
        let (ready_tx, ready_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("rshell-rdp-clipboard".to_owned())
            .spawn(move || {
                let clipboard = match WinClipboard::new(proxy.clone()) {
                    Ok(clipboard) => clipboard,
                    Err(error) => {
                        #[cfg(debug_assertions)]
                        probe_log(&format!("WinClipboard::new failed: {error}"));
                        let _ = ready_tx.send(Err(format!("RDP clipboard init failed: {error}")));
                        return;
                    }
                };
                let factory: Box<dyn CliprdrBackendFactory + Send> = Box::new(FileFactory {
                    inner: clipboard.backend_factory(),
                    proxy,
                    files,
                });
                let thread_id = unsafe { GetCurrentThreadId() };
                force_inactive_clipboard_window(thread_id);
                #[cfg(debug_assertions)]
                probe_log(&format!("clipboard host started thread={thread_id}"));
                if ready_tx.send(Ok((factory, thread_id))).is_err() {
                    return;
                }
                let mut message = MSG::default();
                #[cfg(debug_assertions)]
                probe_log("clipboard host message loop entered");
                while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
                    unsafe {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                #[cfg(debug_assertions)]
                probe_log("clipboard host message loop exited");
                drop(clipboard);
            })
            .map_err(|error| format!("Could not start RDP clipboard thread: {error}"))?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok((factory, thread_id))) => Ok((factory, ClipboardHost { thread_id })),
            Ok(Err(error)) => Err(error),
            Err(_) => Err("RDP clipboard thread did not initialize".to_owned()),
        }
    }
}

#[cfg(windows)]
pub use platform::{start, ClipboardHost};

#[cfg(not(windows))]
#[derive(Default)]
pub struct ClipboardHost;
