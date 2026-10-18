use std::time::Duration;

use eframe::egui;
use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS};

struct Probe {
    initialized: bool,
}

impl eframe::App for Probe {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.initialized {
            self.initialized = true;
            rshell::rdp::keyboard_capture::disable_raw_keyboard();
        }
        rshell::rdp::keyboard_capture::set_enabled(true);
        ui.label("hook probe eframe");
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

fn main() -> eframe::Result<()> {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(5));
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
    eframe::run_native(
        "hook probe eframe",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(Probe { initialized: false }))),
    )
}
