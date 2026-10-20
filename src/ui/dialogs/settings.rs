use super::common::remember_label;
use super::*;
use crate::ui::components::icon_button;
use crate::ui::text_input::TextInput;
use std::time::{Duration, Instant};

const CONFIG_COPY_FEEDBACK_DURATION: Duration = Duration::from_secs(2);
const SETTINGS_WIDTH: f32 = 760.0;
const SETTINGS_HEIGHT: f32 = 760.0;
const SETTINGS_CONTENT_HEIGHT: f32 = 640.0;
const SETTINGS_LABEL_WIDTH: f32 = 160.0;
const SETTINGS_ROW_HEIGHT: f32 = 36.0;
const SETTINGS_CARD_MARGIN: f32 = 12.0;

#[derive(Default)]
struct SettingsChanges {
    fonts_changed: bool,
    theme_action: Option<ThemeMode>,
    language_changed: bool,
    ui_font_size_changed: bool,
    terminal_theme_changed: bool,
    terminal_settings_changed: bool,
    security_action: Option<SecurityAction>,
    lock_now: bool,
}

pub(super) fn settings_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let mut changes = SettingsChanges::default();

    let (close_requested, _) = dialog_window(
        ctx,
        "settings",
        tr(lang, "settings_title"),
        Icon::Settings,
        SETTINGS_WIDTH,
        Some(SETTINGS_HEIGHT),
        true,
        palette,
        |ui| {
            ScrollArea::vertical()
                .id_salt("settings_single_page_scroll")
                .max_height(SETTINGS_CONTENT_HEIGHT)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    appearance_settings(ui, app, palette, lang, &mut changes);
                    terminal_settings(ui, app, palette, lang, &mut changes);
                    history_system_settings(ui, app, palette, lang, &mut changes);
                    security_settings(ui, app, ctx, palette, lang, &mut changes);
                });
        },
    );

    app.settings_open = !close_requested;
    if let Some(theme) = changes.theme_action {
        if theme != app.settings.theme {
            app.settings.theme = theme;
            app.runtime_theme = theme;
            app.apply_theme(ctx);
        }
    }
    if changes.fonts_changed {
        if let Err(error) = app.apply_fonts(ctx) {
            app.security_notice = Some((false, error));
        }
    }
    if changes.language_changed
        || changes.terminal_theme_changed
        || changes.theme_action.is_some()
        || changes.fonts_changed
        || changes.ui_font_size_changed
        || changes.terminal_settings_changed
        || changes.lock_now
    {
        app.persist_settings();
    }
    if changes.ui_font_size_changed {
        crate::theme::apply_ui_font_size(ctx, app.settings.ui_font_size);
    }
    if let Some(action) = changes.security_action {
        app.update_password(action);
    }
    if changes.lock_now {
        app.settings_open = false;
        app.lock_now();
    }
}

fn appearance_settings(
    ui: &mut Ui,
    app: &mut ShellCoreApp,
    palette: Palette,
    lang: Language,
    changes: &mut SettingsChanges,
) {
    settings_section(ui, palette, tr(lang, "appearance"), |ui| {
        settings_row(ui, palette, tr(lang, "theme"), |ui| {
            let choice_width =
                ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(100.0);
            if theme_choice(
                ui,
                palette,
                tr(lang, "theme_dark"),
                app.settings.theme == ThemeMode::Dark,
                choice_width,
            ) {
                changes.theme_action = Some(ThemeMode::Dark);
            }
            if theme_choice(
                ui,
                palette,
                tr(lang, "theme_light"),
                app.settings.theme == ThemeMode::Light,
                choice_width,
            ) {
                changes.theme_action = Some(ThemeMode::Light);
            }
        });
        settings_row(ui, palette, tr(lang, "language"), |ui| {
            ComboBox::from_id_salt("settings_language")
                .icon(combo_icon)
                .selected_text(app.settings.language.label())
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(app.settings.language == Language::ZhCn, "简体中文")
                        .clicked()
                    {
                        app.settings.language = Language::ZhCn;
                        changes.language_changed = true;
                    }
                    if ui
                        .selectable_label(app.settings.language == Language::EnUs, "English")
                        .clicked()
                    {
                        app.settings.language = Language::EnUs;
                        changes.language_changed = true;
                    }
                });
        });
        settings_row(ui, palette, tr(lang, "ui_font_size"), |ui| {
            let value_width = 58.0;
            ui.spacing_mut().slider_width =
                (ui.available_width() - value_width - ui.spacing().item_spacing.x).max(100.0);
            let mut font_size = app.settings.ui_font_size;
            if ui
                .add(
                    egui::Slider::new(
                        &mut font_size,
                        crate::theme::UI_FONT_SIZE_MIN..=crate::theme::UI_FONT_SIZE_MAX,
                    )
                    .step_by(0.5)
                    .fixed_decimals(1)
                    .show_value(false),
                )
                .changed()
            {
                app.settings.ui_font_size = font_size;
                changes.ui_font_size_changed = true;
            }
            ui.add_sized(
                [value_width, SETTINGS_ROW_HEIGHT],
                egui::Label::new(
                    RichText::new(format!("{:.1} px", app.settings.ui_font_size))
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                ),
            );
        });

        let previous_ui_font = app.settings.ui_font.clone();
        settings_row(ui, palette, tr(lang, "ui_font"), |ui| {
            font_picker(
                ui,
                palette,
                lang,
                "settings_ui_font_filter",
                &mut app.font_filter_ui,
                &mut app.settings.ui_font,
                &app.fonts.ui_families,
            );
        });
        if previous_ui_font != app.settings.ui_font {
            changes.fonts_changed = true;
        }
    });
}

fn terminal_settings(
    ui: &mut Ui,
    app: &mut ShellCoreApp,
    palette: Palette,
    lang: Language,
    changes: &mut SettingsChanges,
) {
    settings_section(ui, palette, tr(lang, "terminal"), |ui| {
        let previous_terminal_font = app.settings.terminal_font.clone();
        settings_row(ui, palette, tr(lang, "terminal_font"), |ui| {
            font_picker(
                ui,
                palette,
                lang,
                "settings_terminal_font_filter",
                &mut app.font_filter_mono,
                &mut app.settings.terminal_font,
                &app.fonts.mono_families,
            );
        });
        if previous_terminal_font != app.settings.terminal_font {
            changes.fonts_changed = true;
        }

        settings_row(ui, palette, tr(lang, "preview"), |ui| {
            let width = ui.available_width();
            let terminal_palette =
                crate::theme::TerminalPalette::for_theme(app.settings.terminal_theme);
            Frame::new()
                .fill(terminal_palette.background)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(CornerRadius::same(3))
                .inner_margin(Margin::same(6))
                .show(ui, |ui| {
                    ui.set_width((width - 14.0).max(80.0));
                    ui.label(
                        RichText::new("RShell · root@prod-node-01:~# kubectl get pods")
                            .size(13.0)
                            .family(FontFamily::Monospace)
                            .color(terminal_palette.foreground),
                    );
                });
        });

        settings_row(ui, palette, tr(lang, "zoom"), |ui| {
            let value_width = 52.0;
            ui.spacing_mut().slider_width =
                (ui.available_width() - value_width - ui.spacing().item_spacing.x).max(100.0);
            if ui
                .add(
                    egui::Slider::new(&mut app.settings.terminal_zoom, 0.75..=1.6)
                        .show_value(false),
                )
                .changed()
            {
                changes.terminal_settings_changed = true;
            }
            ui.add_sized(
                [value_width, SETTINGS_ROW_HEIGHT],
                egui::Label::new(
                    RichText::new(format!("{:.0}%", app.settings.terminal_zoom * 100.0))
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                ),
            );
        });
        settings_row(ui, palette, tr(lang, "terminal_color_theme"), |ui| {
            ComboBox::from_id_salt("terminal_color_theme")
                .icon(combo_icon)
                .selected_text(app.settings.terminal_theme.label())
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for theme in [
                        crate::domain::TerminalColorTheme::RShellDark,
                        crate::domain::TerminalColorTheme::RShellLight,
                        crate::domain::TerminalColorTheme::SolarizedDark,
                        crate::domain::TerminalColorTheme::Dracula,
                        crate::domain::TerminalColorTheme::Paper,
                    ] {
                        if ui
                            .selectable_label(app.settings.terminal_theme == theme, theme.label())
                            .clicked()
                        {
                            app.settings.terminal_theme = theme;
                            changes.terminal_theme_changed = true;
                        }
                    }
                });
        });
        if settings_checkbox_row(
            ui,
            palette,
            tr(lang, "auto_scroll"),
            &mut app.settings.auto_scroll,
            true,
        ) {
            changes.terminal_settings_changed = true;
        }
    });
}

fn history_system_settings(
    ui: &mut Ui,
    app: &mut ShellCoreApp,
    palette: Palette,
    lang: Language,
    changes: &mut SettingsChanges,
) {
    settings_section(ui, palette, tr(lang, "command_history"), |ui| {
        settings_row(ui, palette, tr(lang, "command_history_limit"), |ui| {
            let mut limit = app.settings.command_history_limit as i32;
            if ui
                .add_sized(
                    [96.0, 26.0],
                    egui::DragValue::new(&mut limit)
                        .range(10..=10_000)
                        .speed(10),
                )
                .changed()
            {
                app.settings.command_history_limit = limit as usize;
                changes.terminal_settings_changed = true;
            }
            ui.label(
                RichText::new(tr(lang, "commands_unit"))
                    .size(13.0)
                    .color(palette.muted),
            );
        });
        settings_note(ui, palette, tr(lang, "command_history_hint"));
    });

    settings_section(ui, palette, tr(lang, "system_integration"), |ui| {
        settings_checkbox_row(
            ui,
            palette,
            tr(lang, "taskbar_progress"),
            &mut app.settings.taskbar_progress,
            false,
        );
        settings_checkbox_row(
            ui,
            palette,
            tr(lang, "taskbar_attention"),
            &mut app.settings.taskbar_attention,
            false,
        );
    });
}

fn security_settings(
    ui: &mut Ui,
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
    changes: &mut SettingsChanges,
) {
    settings_section(ui, palette, tr(lang, "app_password"), |ui| {
        ui.label(
            RichText::new(tr(lang, "security_boundary"))
                .size(13.0)
                .color(palette.muted),
        );
        ui.add_space(6.0);
        if app.security_record.has_password() {
            settings_row(ui, palette, tr(lang, "current_password"), |ui| {
                TextInput::singleline(&mut app.security_current)
                    .password(true)
                    .desired_width(ui.available_width())
                    .show(ui, palette, lang);
            });
        } else {
            settings_note(ui, palette, tr(lang, "password_optional"));
        }
        settings_row(ui, palette, tr(lang, "new_password"), |ui| {
            TextInput::singleline(&mut app.security_new)
                .password(true)
                .desired_width(ui.available_width())
                .show(ui, palette, lang);
        });
        settings_row(ui, palette, tr(lang, "confirm_password"), |ui| {
            TextInput::singleline(&mut app.security_confirm)
                .password(true)
                .desired_width(ui.available_width())
                .show(ui, palette, lang);
        });
        settings_row(ui, palette, tr(lang, "remember"), |ui| {
            remember_combo_sized(ui, lang, &mut app.security_remember, ui.available_width());
        });
        ui.add_space(2.0);
        settings_row(ui, palette, "", |ui| {
            let label = if app.security_record.has_password() {
                tr(lang, "change_password")
            } else {
                tr(lang, "set_password")
            };
            if primary_button(ui, palette, label).clicked() {
                changes.security_action = Some(if app.security_record.has_password() {
                    SecurityAction::Change
                } else {
                    SecurityAction::Set
                });
            }
            if app.security_record.has_password() {
                if secondary_button(ui, palette, tr(lang, "remove_password")).clicked() {
                    changes.security_action = Some(SecurityAction::Remove);
                }
                if secondary_button(ui, palette, tr(lang, "lock_now")).clicked() {
                    changes.lock_now = true;
                }
            }
        });
        if let Some((success, message)) = &app.security_notice {
            settings_row(ui, palette, "", |ui| {
                ui.label(RichText::new(message).size(13.0).color(if *success {
                    palette.green
                } else {
                    palette.red
                }));
            });
        }
        if let Some(expiry) = app.security_record.remember_until {
            settings_row(ui, palette, tr(lang, "auto_unlock_expires"), |ui| {
                ui.label(
                    RichText::new(remember_expiry_label(Some(expiry)))
                        .size(13.0)
                        .monospace()
                        .color(palette.muted),
                );
            });
        }
    });

    settings_section(ui, palette, tr(lang, "local_data"), |ui| {
        let config_path = app.config.database_path.display().to_string();
        let config_copied = app
            .config_copy_feedback
            .is_some_and(|at| at.elapsed() < CONFIG_COPY_FEEDBACK_DURATION);
        let copy_feedback = tr(lang, "config_file_copied");
        settings_row(ui, palette, tr(lang, "config_file"), |ui| {
            let feedback_width = if config_copied {
                ui.painter()
                    .layout_no_wrap(
                        copy_feedback.to_owned(),
                        FontId::proportional(13.0),
                        palette.green,
                    )
                    .size()
                    .x
                    + ui.spacing().item_spacing.x
            } else {
                0.0
            };
            let max_path_width =
                (ui.available_width() - 26.0 - ui.spacing().item_spacing.x - feedback_width)
                    .max(80.0);
            let path_width = ui
                .painter()
                .layout_no_wrap(config_path.clone(), FontId::monospace(13.0), palette.muted)
                .size()
                .x
                .min(max_path_width);
            ui.add_sized(
                [path_width, SETTINGS_ROW_HEIGHT],
                egui::Label::new(
                    RichText::new(&config_path)
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
                if config_copied {
                    Icon::CircleCheck
                } else {
                    Icon::Copy
                },
                13.0,
                if config_copied {
                    copy_feedback
                } else {
                    tr(lang, "copy_config_file")
                },
                config_copied,
            )
            .clicked()
            {
                if crate::ui::clipboard::copy_file_to_clipboard(&app.config.database_path) {
                    app.config_copy_feedback = Some(Instant::now());
                    ctx.request_repaint();
                    ctx.request_repaint_after(CONFIG_COPY_FEEDBACK_DURATION);
                } else {
                    crate::ui::clipboard::copy_text(ctx, config_path);
                    app.config_copy_feedback = None;
                }
            }
            if config_copied {
                ui.label(RichText::new(copy_feedback).size(13.0).color(palette.green));
            }
        });
    });
}

fn settings_section(
    ui: &mut Ui,
    palette: Palette,
    title: &str,
    add_contents: impl FnOnce(&mut Ui),
) {
    section_label(ui, palette, title);
    ui.add_space(6.0);
    let outer_width = ui.available_width();
    let inner_width = (outer_width - SETTINGS_CARD_MARGIN * 2.0 - 2.0).max(120.0);
    Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(SETTINGS_CARD_MARGIN as i8))
        .show(ui, |ui| {
            ui.set_width(inner_width);
            add_contents(ui);
        });
    ui.add_space(14.0);
}

fn settings_row<R>(
    ui: &mut Ui,
    palette: Palette,
    label: &str,
    add_control: impl FnOnce(&mut Ui) -> R,
) -> R {
    let control_width = settings_control_width(ui);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(SETTINGS_LABEL_WIDTH, SETTINGS_ROW_HEIGHT),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.set_width(SETTINGS_LABEL_WIDTH);
                ui.add_sized(
                    [SETTINGS_LABEL_WIDTH, SETTINGS_ROW_HEIGHT],
                    egui::Label::new(
                        RichText::new(label)
                            .size(13.0)
                            .monospace()
                            .color(palette.text_soft),
                    )
                    .truncate(),
                )
                .on_hover_text(label);
            },
        );
        ui.allocate_ui_with_layout(
            Vec2::new(control_width, SETTINGS_ROW_HEIGHT),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.set_width(control_width);
                add_control(ui)
            },
        )
        .inner
    })
    .inner
}

fn settings_checkbox_row(
    ui: &mut Ui,
    palette: Palette,
    label: &str,
    checked: &mut bool,
    enabled: bool,
) -> bool {
    ui.add_enabled(
        enabled,
        egui::Checkbox::new(
            checked,
            RichText::new(label).size(13.0).color(if enabled {
                palette.text_soft
            } else {
                palette.muted
            }),
        ),
    )
    .changed()
}

fn settings_note(ui: &mut Ui, palette: Palette, text: &str) {
    let control_width = settings_control_width(ui);
    ui.horizontal(|ui| {
        ui.add_space(SETTINGS_LABEL_WIDTH + ui.spacing().item_spacing.x);
        ui.allocate_ui_with_layout(
            Vec2::new(control_width, 30.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.label(RichText::new(text).size(12.0).color(palette.muted));
            },
        );
    });
}

fn settings_control_width(ui: &Ui) -> f32 {
    (ui.available_width() - SETTINGS_LABEL_WIDTH - ui.spacing().item_spacing.x).clamp(180.0, 560.0)
}

fn theme_choice(ui: &mut Ui, palette: Palette, label: &str, selected: bool, width: f32) -> bool {
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
    ui.add_sized(
        [width, 28.0],
        egui::Button::image_and_text(
            egui::Image::new(icon.source())
                .fit_to_exact_size(Vec2::splat(16.0))
                .tint(tint),
            RichText::new(label).size(13.0).color(if selected {
                palette.accent
            } else {
                palette.text_soft
            }),
        )
        .selected(selected)
        .fill(if selected {
            palette.accent_soft
        } else {
            palette.raised
        })
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(4)),
    )
    .clicked()
}

fn font_picker(
    ui: &mut Ui,
    palette: Palette,
    lang: Language,
    filter_id: &'static str,
    filter: &mut String,
    selected: &mut String,
    families: &[String],
) {
    let spacing = ui.spacing().item_spacing.x;
    let search_width = 132.0;
    let combo_width = (ui.available_width() - search_width - spacing).max(150.0);
    TextInput::singleline(filter)
        .hint_text(tr(lang, "font_search"))
        .desired_width(search_width)
        .font(egui::TextStyle::Small)
        .show(ui, palette, lang);
    ComboBox::from_id_salt(filter_id)
        .icon(combo_icon)
        .selected_text(RichText::new(selected.as_str()).size(13.0))
        .width(combo_width)
        .show_ui(ui, |ui| {
            ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                let query = filter.to_ascii_lowercase();
                let mut shown = 0;
                for family in families {
                    if !query.is_empty() && !family.to_ascii_lowercase().contains(&query) {
                        continue;
                    }
                    if ui
                        .selectable_label(selected == family, RichText::new(family).size(13.0))
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
}

fn remember_combo_sized(ui: &mut Ui, lang: Language, value: &mut RememberDuration, width: f32) {
    ComboBox::from_id_salt("settings_remember_duration")
        .icon(combo_icon)
        .selected_text(remember_label(lang, *value))
        .width(width)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, RememberDuration::None, tr(lang, "remember_none"));
            ui.selectable_value(value, RememberDuration::Week, tr(lang, "remember_week"));
            ui.selectable_value(value, RememberDuration::Month, tr(lang, "remember_month"));
            ui.selectable_value(
                value,
                RememberDuration::Forever,
                tr(lang, "remember_forever"),
            );
        });
}
