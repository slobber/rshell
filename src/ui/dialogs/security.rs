use super::*;
use crate::ui::text_input::TextInput;

pub fn lock_screen(app: &mut ShellCoreApp, ui: &mut Ui) {
    let palette = Palette::for_mode(app.settings.theme);
    let lang = app.settings.language;
    egui::CentralPanel::default()
        .frame(Frame::new().fill(palette.canvas).inner_margin(Margin::ZERO))
        .show(ui, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.add_space((ui.available_height() * 0.13).clamp(56.0, 150.0));
                Frame::new()
                    .fill(palette.panel_alt)
                    .stroke(Stroke::new(1.0, palette.border_strong))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::same(24))
                    .show(ui, |ui| {
                        ui.set_width(420.0);
                        ui.vertical_centered(|ui| {
                            crate::icons::image(ui, Icon::ShieldLock, 42.0, palette.accent);
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(tr(lang, "lock_title"))
                                    .size(20.0)
                                    .strong()
                                    .color(palette.text),
                            );
                            ui.label(
                                RichText::new(tr(lang, "lock_subtitle"))
                                    .size(13.0)
                                    .color(palette.muted),
                            );
                            ui.add_space(18.0);
                            ui.label(
                                RichText::new(app.config.database_path.display().to_string())
                                    .size(13.0)
                                    .monospace()
                                    .color(palette.muted),
                            );
                            ui.add_space(8.0);
                        });

                        if app.lock_state == AppLock::CredentialsLost {
                            ui.colored_label(
                                palette.amber,
                                RichText::new(tr(lang, "keyring_unavailable")).size(13.0),
                            );
                            ui.label(
                                RichText::new(tr(lang, "reset_warning"))
                                    .size(13.0)
                                    .color(palette.text_soft),
                            );
                            ui.add_space(10.0);
                            if primary_button(ui, palette, tr(lang, "reset_button")).clicked() {
                                app.reset_lost_credentials();
                            }
                            return;
                        }

                        let response = TextInput::singleline(&mut app.unlock_password)
                            .desired_size([ui.available_width(), 34.0])
                            .password(true)
                            .hint_text(tr(lang, "unlock"))
                            .font(FontId::proportional(13.0))
                            .show(ui, palette, lang);
                        if response.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter))
                        {
                            app.unlock();
                        }
                        response.request_focus();
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(tr(lang, "remember"))
                                    .size(13.0)
                                    .color(palette.text_soft),
                            );
                            remember_combo(ui, palette, lang, &mut app.unlock_remember);
                        });
                        if let Some(error) = &app.unlock_error {
                            ui.add_space(5.0);
                            ui.colored_label(palette.red, error);
                        }
                        ui.add_space(12.0);
                        if ui
                            .add_sized(
                                [ui.available_width(), 34.0],
                                egui::Button::new(
                                    RichText::new(format!("{}  {}", tr(lang, "unlock"), "Enter"))
                                        .size(13.0)
                                        .strong()
                                        .color(palette.on_accent),
                                )
                                .fill(palette.accent)
                                .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            app.unlock();
                        }
                        ui.add_space(8.0);
                        ui.vertical_centered(|ui| {
                            let forgot = ui.add(
                                egui::Button::new(
                                    RichText::new(tr(lang, "forgot_password"))
                                        .size(13.0)
                                        .color(palette.muted),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                            );
                            if forgot.clicked() {
                                app.reset_confirmation = !app.reset_confirmation;
                            }
                        });
                        if app.reset_confirmation {
                            Frame::new()
                                .fill(palette.panel)
                                .stroke(Stroke::new(1.0, palette.amber))
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::same(8))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(tr(lang, "reset_warning"))
                                            .size(13.0)
                                            .color(palette.amber),
                                    );
                                    if ui
                                        .button(
                                            RichText::new(tr(lang, "reset_button"))
                                                .color(palette.red),
                                        )
                                        .clicked()
                                    {
                                        app.reset_lost_credentials();
                                    }
                                });
                        }
                    });
                ui.add_space(16.0);
                ui.label(
                    RichText::new("RShell · Rust + egui · local-first encrypted configuration")
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                );
            });
        });
}
