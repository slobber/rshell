use eframe::egui;

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
