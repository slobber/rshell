#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturedKey {
    pub scan_code: u8,
    pub extended: bool,
    pub release: bool,
}

#[cfg(windows)]
mod platform {
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
    use std::sync::Mutex;

    use windows::core::w;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, MapVirtualKeyW, MAPVK_VK_TO_VSC_EX, VK_CONTROL, VK_ESCAPE, VK_LWIN,
        VK_MENU, VK_RWIN, VK_SNAPSHOT, VK_TAB,
    };
    use windows::Win32::UI::Input::{RegisterRawInputDevices, RAWINPUTDEVICE, RIDEV_REMOVE};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, CreateWindowExW, DispatchMessageW, GetMessageW, SetWindowsHookExW,
        TranslateMessage, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MSG, WH_KEYBOARD_LL, WINDOW_EX_STYLE,
        WINDOW_STYLE, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    use super::CapturedKey;

    static ENABLED: AtomicBool = AtomicBool::new(false);
    static WIN_DOWN: AtomicBool = AtomicBool::new(false);
    static HOOK: AtomicIsize = AtomicIsize::new(0);
    static HOOK_STARTED: AtomicBool = AtomicBool::new(false);
    static QUEUE: Mutex<Vec<CapturedKey>> = Mutex::new(Vec::new());

    fn pressed(key: u16) -> bool {
        unsafe { GetAsyncKeyState(i32::from(key)) as u16 & 0x8000 != 0 }
    }

    fn mapped_scan_code(vk: u32) -> (u8, bool) {
        let mapped = unsafe { MapVirtualKeyW(vk, MAPVK_VK_TO_VSC_EX) };
        let scan_code = (mapped & 0xff) as u8;
        let extended = matches!(mapped >> 8, 0xe0 | 0xe1);
        (scan_code, extended)
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && ENABLED.load(Ordering::Relaxed) {
            let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            let message = wparam.0 as u32;
            let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
            let up = message == WM_KEYUP || message == WM_SYSKEYUP;
            if down || up {
                let vk = info.vkCode;
                let swallow = if vk == u32::from(VK_LWIN.0) || vk == u32::from(VK_RWIN.0) {
                    WIN_DOWN.store(down, Ordering::Relaxed);
                    true
                } else if vk == u32::from(VK_SNAPSHOT.0)
                    || (vk == u32::from(VK_TAB.0) && pressed(VK_MENU.0))
                    || (vk == u32::from(VK_ESCAPE.0)
                        && (pressed(VK_MENU.0) || pressed(VK_CONTROL.0)))
                {
                    true
                } else {
                    WIN_DOWN.load(Ordering::Relaxed)
                };
                if swallow {
                    let (mapped_scan, mapped_extended) = mapped_scan_code(vk);
                    if let Ok(mut queue) = QUEUE.lock() {
                        queue.push(CapturedKey {
                            scan_code: if mapped_scan == 0 {
                                info.scanCode as u8
                            } else {
                                mapped_scan
                            },
                            extended: mapped_extended || info.flags.contains(LLKHF_EXTENDED),
                            release: up,
                        });
                        if queue.len() > 64 {
                            queue.remove(0);
                        }
                    }
                    return LRESULT(1);
                }
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    fn install_hook() -> bool {
        let module = unsafe { GetModuleHandleW(w!("user32.dll")) }.unwrap_or_default();
        match unsafe {
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                Some(HINSTANCE(module.0)),
                0,
            )
        } {
            Ok(hook) => {
                HOOK.store(hook.0 as isize, Ordering::SeqCst);
                true
            }
            Err(error) => {
                eprintln!("keyboard capture hook install failed: {error}");
                false
            }
        }
    }

    fn start_hook_thread() {
        if HOOK_STARTED.swap(true, Ordering::SeqCst) {
            return;
        }
        let started = std::thread::Builder::new()
            .name("rshell-keyboard-hook".to_owned())
            .spawn(|| {
                let _window = unsafe {
                    CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        w!("STATIC"),
                        w!("RShell keyboard hook"),
                        WINDOW_STYLE(0),
                        0,
                        0,
                        0,
                        0,
                        None,
                        None,
                        None,
                        None,
                    )
                };
                if !install_hook() {
                    HOOK_STARTED.store(false, Ordering::SeqCst);
                    return;
                }
                let mut message = MSG::default();
                while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
                    unsafe {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
            });
        if started.is_err() {
            HOOK_STARTED.store(false, Ordering::SeqCst);
        }
    }

    fn remove_raw_keyboard() {
        let devices = [RAWINPUTDEVICE {
            usUsagePage: 1,
            usUsage: 6,
            dwFlags: RIDEV_REMOVE,
            hwndTarget: HWND::default(),
        }];
        if let Err(error) = unsafe {
            RegisterRawInputDevices(&devices, std::mem::size_of::<RAWINPUTDEVICE>() as u32)
        } {
            eprintln!("keyboard raw input removal failed: {error}");
        }
    }

    pub fn set_enabled(enabled: bool) {
        let was_enabled = ENABLED.swap(enabled, Ordering::SeqCst);
        if enabled {
            // Winit registers raw keyboard input for device events. With that
            // registration active Windows routes Win-key downs past low-level
            // hooks, so only key-up events reach the RDP surface.
            if !was_enabled {
                remove_raw_keyboard();
            }
            start_hook_thread();
        } else {
            WIN_DOWN.store(false, Ordering::SeqCst);
            if let Ok(mut queue) = QUEUE.lock() {
                queue.clear();
            }
        }
    }

    pub fn drain() -> Vec<CapturedKey> {
        QUEUE
            .lock()
            .map(|mut queue| std::mem::take(&mut *queue))
            .unwrap_or_default()
    }

    pub fn is_enabled() -> bool {
        ENABLED.load(Ordering::SeqCst) && HOOK.load(Ordering::SeqCst) != 0
    }
}

#[cfg(windows)]
pub use platform::{drain, is_enabled, set_enabled};

#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool) {}

#[cfg(not(windows))]
pub fn drain() -> Vec<CapturedKey> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn is_enabled() -> bool {
    false
}
