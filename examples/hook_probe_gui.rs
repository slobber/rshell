use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DispatchMessageW, GetMessageW, PostQuitMessage,
    SetForegroundWindow, SetWindowsHookExW, ShowWindow, TranslateMessage, KBDLLHOOKSTRUCT,
    MSG, SW_SHOW, WH_KEYBOARD_LL, WINDOW_EX_STYLE, WINDOW_STYLE, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static ENABLED: AtomicBool = AtomicBool::new(true);

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && ENABLED.load(Ordering::Relaxed) {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let message = wparam.0 as u32;
        if message == WM_KEYDOWN || message == WM_KEYUP || message == WM_SYSKEYDOWN
            || message == WM_SYSKEYUP
        {
            println!("vk={} msg={message:#x}", info.vkCode);
        }
        if info.vkCode == 0x5B {
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn main() {
    let module = unsafe { GetModuleHandleW(None) }.expect("module handle");
    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("hook probe gui"),
            WINDOW_STYLE(0x10CF_0000),
            100,
            100,
            500,
            300,
            None,
            None,
            Some(HINSTANCE(module.0)),
            None,
        )
    }
    .expect("window");
    unsafe {
        let _ = ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
    }
    let hook = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_proc),
            Some(HINSTANCE(module.0)),
            0,
        )
    }
    .expect("install hook");
    println!("hook={:?}", hook.0);
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(15));
        println!("send win+d");
        unsafe {
            keybd_event(0x5B, 0, KEYBD_EVENT_FLAGS(0), 0);
            std::thread::sleep(Duration::from_millis(80));
            keybd_event(0x44, 0, KEYBD_EVENT_FLAGS(0), 0);
            std::thread::sleep(Duration::from_millis(80));
            keybd_event(0x44, 0, KEYBD_EVENT_FLAGS(2), 0);
            std::thread::sleep(Duration::from_millis(80));
            keybd_event(0x5B, 0, KEYBD_EVENT_FLAGS(2), 0);
        }
        std::thread::sleep(Duration::from_secs(2));
        std::process::exit(0);
    });
    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
