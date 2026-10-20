use eframe::egui;
use std::path::Path;

pub fn copy_text(ctx: &egui::Context, text: String) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if clipboard.set_text(text.clone()).is_ok() {
            return;
        }
    }
    ctx.copy_text(text);
}

pub fn clipboard_text() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

#[cfg(windows)]
pub fn copy_file_to_clipboard(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use std::time::Duration;
    use windows::Win32::Foundation::{GlobalFree, HANDLE};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
    };

    const CF_HDROP: u32 = 15;

    #[repr(C)]
    struct DropFiles {
        p_files: u32,
        x: i32,
        y: i32,
        non_client: i32,
        wide: i32,
    }

    if !path.is_file() {
        return false;
    }
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut encoded = path.as_os_str().encode_wide().collect::<Vec<_>>();
    encoded.push(0);
    encoded.push(0);
    let header_size = std::mem::size_of::<DropFiles>();
    let total_size = header_size + encoded.len() * 2;

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
            return false;
        }

        let mut copied = false;
        if EmptyClipboard().is_ok() {
            if let Ok(handle) = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total_size) {
                let pointer = GlobalLock(handle);
                if !pointer.is_null() {
                    let header = DropFiles {
                        p_files: header_size as u32,
                        x: 0,
                        y: 0,
                        non_client: 0,
                        wide: 1,
                    };
                    std::ptr::copy_nonoverlapping(
                        (&header as *const DropFiles).cast::<u8>(),
                        pointer.cast::<u8>(),
                        header_size,
                    );
                    std::ptr::copy_nonoverlapping(
                        encoded.as_ptr().cast::<u8>(),
                        pointer.cast::<u8>().add(header_size),
                        encoded.len() * 2,
                    );
                    let _ = GlobalUnlock(handle);
                    copied = SetClipboardData(CF_HDROP, Some(HANDLE(handle.0))).is_ok();
                }
                if !copied {
                    let _ = GlobalFree(Some(handle));
                }
            }
        }
        let _ = CloseClipboard();
        copied
    }
}

#[cfg(not(windows))]
pub fn copy_file_to_clipboard(_path: &Path) -> bool {
    false
}
