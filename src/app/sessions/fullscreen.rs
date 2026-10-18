use crate::app::ShellCoreApp;

impl ShellCoreApp {
    pub fn enter_rdp_fullscreen(&mut self, tab_id: u64, ctx: &eframe::egui::Context) {
        self.rdp_fullscreen_tab = Some(tab_id);
        self.pending_focus_tab = Some(tab_id);
        self.window_maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
        if self.window_maximized {
            // winit on Windows keeps the maximized work-area geometry when
            // fullscreen is requested directly from a maximized window.
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Maximized(false));
        }
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Fullscreen(true));
        ctx.request_repaint();
    }

    pub fn exit_rdp_fullscreen(&mut self, ctx: &eframe::egui::Context) {
        if self.rdp_fullscreen_tab.take().is_some() {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Fullscreen(false));
            if self.window_maximized {
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Maximized(true));
            }
            self.window_maximized = false;
            ctx.request_repaint();
        }
    }
}
