use super::*;
use crate::ui::components::icon_button;
use crate::ui::text_input::TextInput;

pub(super) fn settings_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let mut fonts_changed = false;
    let mut theme_action = None;
    let mut language_changed = false;
    let mut ui_font_size_changed = false;
    let mut terminal_theme_changed = false;
    let mut terminal_settings_changed = false;
    let mut security_action = None;
    let mut lock_now = false;

    let (close_requested, _) = dialog_window(
        ctx,
        "settings",
        tr(lang, "settings_title"),
        Icon::Settings,
        690.0,
        Some(720.0),
        true,
        palette,
        |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                section_label(ui, palette, tr(lang, "appearance"));
                ui.add_space(5.0);
                Frame::new()
                    .fill(palette.panel)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if theme_choice(
                                ui,
                                palette,
                                tr(lang, "theme_dark"),
                                app.settings.theme == ThemeMode::Dark,
                            ) {
                                theme_action = Some(ThemeMode::Dark);
                            }
                            if theme_choice(
                                ui,
                                palette,
                                tr(lang, "theme_light"),
                                app.settings.theme == ThemeMode::Light,
                            ) {
                                theme_action = Some(ThemeMode::Light);
                            }
                            ui.add_space(15.0);
                            ui.label(
                                RichText::new(tr(lang, "language"))
                                    .size(13.0)
                                    .color(palette.text_soft),
                            );
                            ComboBox::from_id_salt("settings_language")
                                .icon(combo_icon)
                                .selected_text(app.settings.language.label())
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            app.settings.language == Language::ZhCn,
                                            "简体中文",
                                        )
                                        .clicked()
                                    {
                                        app.settings.language = Language::ZhCn;
                                        language_changed = true;
                                    }
                                    if ui
                                        .selectable_label(
                                            app.settings.language == Language::EnUs,
                                            "English",
                                        )
                                        .clicked()
                                    {
                                        app.settings.language = Language::EnUs;
                                        language_changed = true;
                                    }
                                });
                        });
                        ui.add_space(7.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().slider_width = 220.0;
                            ui.label(
                                RichText::new(tr(lang, "ui_font_size"))
                                    .size(13.0)
                                    .color(palette.text_soft),
                            );
                            let mut font_size = app.settings.ui_font_size;
                            if ui
                                .add(
                                    egui::Slider::new(
                                        &mut font_size,
                                        crate::theme::UI_FONT_SIZE_MIN
                                            ..=crate::theme::UI_FONT_SIZE_MAX,
                                    )
                                    .step_by(0.5)
                                    .fixed_decimals(1)
                                    .show_value(false),
                                )
                                .changed()
                            {
                                app.settings.ui_font_size = font_size;
                                ui_font_size_changed = true;
                            }
                            ui.label(
                                RichText::new(format!("{:.1} px", app.settings.ui_font_size))
                                    .size(13.0)
                                    .monospace()
                                    .color(palette.muted),
                            );
                        });
                    });

                ui.add_space(12.0);
                section_label(ui, palette, tr(lang, "interface"));
                ui.add_space(4.0);
                Frame::new()
                    .fill(palette.panel)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        let previous_ui_font = app.settings.ui_font.clone();
                        font_picker(
                            ui,
                            palette,
                            lang,
                            "ui_font",
                            "settings_ui_font_filter",
                            &mut app.font_filter_ui,
                            &mut app.settings.ui_font,
                            &app.fonts.ui_families,
                        );
                        if previous_ui_font != app.settings.ui_font {
                            fonts_changed = true;
                        }
                        ui.add_space(7.0);
                        let previous_terminal_font = app.settings.terminal_font.clone();
                        font_picker(
                            ui,
                            palette,
                            lang,
                            "terminal_font",
                            "settings_terminal_font_filter",
                            &mut app.font_filter_mono,
                            &mut app.settings.terminal_font,
                            &app.fonts.mono_families,
                        );
                        if previous_terminal_font != app.settings.terminal_font {
                            fonts_changed = true;
                        }
                        ui.add_space(9.0);
                        ui.label(
                            RichText::new(tr(lang, "preview"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        let terminal_palette =
                            crate::theme::TerminalPalette::for_theme(app.settings.terminal_theme);
                        Frame::new()
                            .fill(terminal_palette.background)
                            .stroke(Stroke::new(1.0, palette.border))
                            .corner_radius(CornerRadius::same(3))
                            .inner_margin(Margin::same(7))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("RShell · root@prod-node-01:~# kubectl get pods")
                                        .size(13.0)
                                        .family(FontFamily::Monospace)
                                        .color(terminal_palette.foreground),
                                );
                            });
                    });

                ui.add_space(12.0);
                section_label(ui, palette, tr(lang, "terminal"));
                ui.add_space(4.0);
                Frame::new()
                    .fill(palette.panel)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(tr(lang, "terminal"))
                                    .size(13.0)
                                    .color(palette.text_soft),
                            );
                            if ui
                                .add(
                                    egui::Slider::new(&mut app.settings.terminal_zoom, 0.75..=1.6)
                                        .text(tr(lang, "zoom")),
                                )
                                .changed()
                            {
                                terminal_settings_changed = true;
                            }
                            ui.label(
                                RichText::new(format!(
                                    "{:.0}%",
                                    app.settings.terminal_zoom * 100.0
                                ))
                                .size(13.0)
                                .monospace()
                                .color(palette.muted),
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(tr(lang, "command_history_limit"))
                                    .size(13.0)
                                    .color(palette.text_soft),
                            );
                            let mut limit = app.settings.command_history_limit as i32;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut limit)
                                        .range(10..=10_000)
                                        .speed(10),
                                )
                                .changed()
                            {
                                app.settings.command_history_limit = limit as usize;
                                terminal_settings_changed = true;
                            }
                            ui.label(
                                RichText::new(tr(lang, "commands_unit"))
                                    .size(13.0)
                                    .color(palette.muted),
                            );
                        });
                        ui.label(
                            RichText::new(tr(lang, "command_history_hint"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(tr(lang, "terminal_color_theme"))
                                    .size(13.0)
                                    .color(palette.text_soft),
                            );
                            ComboBox::from_id_salt("terminal_color_theme")
                                .icon(combo_icon)
                                .selected_text(app.settings.terminal_theme.label())
                                .width(190.0)
                                .show_ui(ui, |ui| {
                                    for theme in [
                                        crate::domain::TerminalColorTheme::RShellDark,
                                        crate::domain::TerminalColorTheme::RShellLight,
                                        crate::domain::TerminalColorTheme::SolarizedDark,
                                        crate::domain::TerminalColorTheme::Dracula,
                                        crate::domain::TerminalColorTheme::Paper,
                                    ] {
                                        if ui
                                            .selectable_label(
                                                app.settings.terminal_theme == theme,
                                                theme.label(),
                                            )
                                            .clicked()
                                        {
                                            app.settings.terminal_theme = theme;
                                            terminal_theme_changed = true;
                                        }
                                    }
                                });
                        });
                        if ui
                            .checkbox(
                                &mut app.settings.auto_scroll,
                                RichText::new(tr(lang, "auto_scroll")).size(13.0),
                            )
                            .changed()
                        {
                            terminal_settings_changed = true;
                        }
                        ui.add_enabled(
                            false,
                            egui::Checkbox::new(
                                &mut app.settings.taskbar_progress,
                                RichText::new(tr(lang, "taskbar_progress")).size(13.0),
                            ),
                        );
                        ui.add_enabled(
                            false,
                            egui::Checkbox::new(
                                &mut app.settings.taskbar_attention,
                                RichText::new(tr(lang, "taskbar_attention")).size(13.0),
                            ),
                        );
                    });

                ui.add_space(12.0);
                section_label(ui, palette, tr(lang, "password_security"));
                ui.add_space(4.0);
                Frame::new()
                    .fill(palette.panel)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(tr(lang, "security_boundary"))
                                .size(13.0)
                                .color(palette.muted),
                        );
                        ui.add_space(4.0);
                        if app.security_record.has_password() {
                            ui.label(
                                RichText::new(tr(lang, "current_password"))
                                    .size(13.0)
                                    .color(palette.muted),
                            );
                            TextInput::singleline(&mut app.security_current)
                                .password(true)
                                .desired_width(ui.available_width())
                                .show(ui, palette, lang);
                        } else {
                            ui.label(
                                RichText::new(tr(lang, "password_optional"))
                                    .size(13.0)
                                    .color(palette.muted),
                            );
                        }
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(tr(lang, "new_password"))
                                        .size(13.0)
                                        .color(palette.muted),
                                );
                                TextInput::singleline(&mut app.security_new)
                                    .password(true)
                                    .desired_width(275.0)
                                    .show(ui, palette, lang);
                            });
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(tr(lang, "confirm_password"))
                                        .size(13.0)
                                        .color(palette.muted),
                                );
                                TextInput::singleline(&mut app.security_confirm)
                                    .password(true)
                                    .desired_width(275.0)
                                    .show(ui, palette, lang);
                            });
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(tr(lang, "remember"))
                                    .size(13.0)
                                    .color(palette.muted),
                            );
                            remember_combo(ui, palette, lang, &mut app.security_remember);
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let label = if app.security_record.has_password() {
                                tr(lang, "change_password")
                            } else {
                                tr(lang, "set_password")
                            };
                            if primary_button(ui, palette, label).clicked() {
                                security_action = Some(if app.security_record.has_password() {
                                    SecurityAction::Change
                                } else {
                                    SecurityAction::Set
                                });
                            }
                            if app.security_record.has_password() {
                                if secondary_button(ui, palette, tr(lang, "remove_password"))
                                    .clicked()
                                {
                                    security_action = Some(SecurityAction::Remove);
                                }
                                if secondary_button(ui, palette, tr(lang, "lock_now")).clicked() {
                                    lock_now = true;
                                }
                            }
                        });
                        if let Some((success, message)) = &app.security_notice {
                            ui.add_space(4.0);
                            ui.colored_label(
                                if *success { palette.green } else { palette.red },
                                RichText::new(message).size(13.0),
                            );
                        }
                        if let Some(expiry) = app.security_record.remember_until {
                            ui.label(
                                RichText::new(format!(
                                    "{}: {}",
                                    tr(lang, "auto_unlock_expires"),
                                    remember_expiry_label(Some(expiry))
                                ))
                                .size(13.0)
                                .monospace()
                                .color(palette.muted),
                            );
                        }
                    });
                ui.add_space(8.0);
                let config_path = app.config.database_path.display().to_string();
                ui.horizontal(|ui| {
                    let label_width = (ui.available_width() - 30.0).max(80.0);
                    ui.add_sized(
                        [label_width, 18.0],
                        egui::Label::new(
                            RichText::new(format!("{}: {}", tr(lang, "config_file"), config_path))
                                .size(13.0)
                                .monospace()
                                .color(palette.muted),
                        )
                        .truncate(),
                    )
                    .on_hover_text(&config_path);
                    if icon_button(
                        ui,
                        palette,
                        Icon::Copy,
                        13.0,
                        tr(lang, "copy_config_file"),
                        false,
                    )
                    .clicked()
                    {
                        if !crate::ui::clipboard::copy_file_to_clipboard(&app.config.database_path)
                        {
                            crate::ui::clipboard::copy_text(ctx, config_path);
                        }
                        app.toast(tr(lang, "copied"));
                    }
                });
            });
        },
    );

    app.settings_open = !close_requested;
    if let Some(theme) = theme_action {
        if theme != app.settings.theme {
            app.settings.theme = theme;
            app.runtime_theme = theme;
            app.apply_theme(ctx);
        }
    }
    if fonts_changed {
        if let Err(error) = app.apply_fonts(ctx) {
            app.security_notice = Some((false, error));
        }
    }
    if language_changed
        || terminal_theme_changed
        || theme_action.is_some()
        || fonts_changed
        || ui_font_size_changed
        || terminal_settings_changed
        || lock_now
    {
        app.persist_settings();
    }
    if ui_font_size_changed {
        crate::theme::apply_ui_font_size(ctx, app.settings.ui_font_size);
    }
    if let Some(action) = security_action {
        app.update_password(action);
    }
    if lock_now {
        app.settings_open = false;
        app.lock_now();
    }
}

fn theme_choice(ui: &mut Ui, palette: Palette, label: &str, selected: bool) -> bool {
    let icon = if selected {
        Icon::CircleDot
    } else {
        Icon::Circle
    };
    let tint = if selected {
        palette.accent
    } else {
        palette.muted
    };
    ui.selectable_label(
        selected,
        (
            egui::Image::new(icon.source())
                .fit_to_exact_size(Vec2::splat(16.0))
                .tint(tint),
            RichText::new(label).size(13.0),
        ),
    )
    .clicked()
}

fn font_picker(
    ui: &mut Ui,
    palette: Palette,
    lang: Language,
    title: &'static str,
    filter_id: &'static str,
    filter: &mut String,
    selected: &mut String,
    families: &[String],
) {
    ui.horizontal(|ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new(tr(lang, title))
                .size(13.0)
                .color(palette.muted),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ComboBox::from_id_salt(filter_id)
                .icon(combo_icon)
                .selected_text(RichText::new(selected.as_str()).size(13.0))
                .width(290.0)
                .show_ui(ui, |ui| {
                    ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        let query = filter.to_ascii_lowercase();
                        let mut shown = 0;
                        for family in families {
                            if !query.is_empty() && !family.to_ascii_lowercase().contains(&query) {
                                continue;
                            }
                            if ui
                                .selectable_label(
                                    selected == family,
                                    RichText::new(family).size(13.0),
                                )
                                .clicked()
                            {
                                *selected = family.clone();
                            }
                            shown += 1;
                            if shown >= 120 {
                                break;
                            }
                        }
                    });
                });
            TextInput::singleline(filter)
                .hint_text(tr(lang, "font_search"))
                .desired_width(160.0)
                .font(egui::TextStyle::Small)
                .show(ui, palette, lang);
        });
    });
}
