use super::*;
use crate::ui::text_input::TextInput;

#[derive(Clone, Copy)]
enum PaletteAction {
    NewTerminal,
    NewSftp,
    NewSession,
    ToggleTheme,
    SplitColumns,
    SplitRows,
    ToggleConsole,
    Settings,
    Keyring,
    Logs,
    Transfers,
    Tunnels,
    Lock,
}

pub(super) fn command_palette(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let mut open = app.command_palette_open;
    let mut action = None;
    let mut close_requested = false;
    let commands = [
        (
            tr(lang, "new_terminal"),
            tr(lang, "cmd_new_terminal_desc"),
            PaletteAction::NewTerminal,
        ),
        (
            tr(lang, "new_sftp"),
            tr(lang, "cmd_new_sftp_desc"),
            PaletteAction::NewSftp,
        ),
        (
            tr(lang, "new_session"),
            tr(lang, "cmd_new_session_desc"),
            PaletteAction::NewSession,
        ),
        (
            tr(lang, "toggle_theme"),
            tr(lang, "cmd_theme_desc"),
            PaletteAction::ToggleTheme,
        ),
        (
            tr(lang, "split_columns"),
            tr(lang, "cmd_split_desc"),
            PaletteAction::SplitColumns,
        ),
        (
            tr(lang, "split_rows"),
            tr(lang, "cmd_split_desc"),
            PaletteAction::SplitRows,
        ),
        (
            tr(lang, "toggle_console"),
            tr(lang, "cmd_console_desc"),
            PaletteAction::ToggleConsole,
        ),
        (
            tr(lang, "settings"),
            tr(lang, "cmd_settings_desc"),
            PaletteAction::Settings,
        ),
        (
            tr(lang, "keyring"),
            tr(lang, "cmd_keyring_desc"),
            PaletteAction::Keyring,
        ),
        (
            tr(lang, "connection_log"),
            tr(lang, "cmd_logs_desc"),
            PaletteAction::Logs,
        ),
        (
            tr(lang, "transfer_queue"),
            tr(lang, "cmd_transfers_desc"),
            PaletteAction::Transfers,
        ),
        (
            tr(lang, "port_tunnels"),
            tr(lang, "cmd_tunnels_desc"),
            PaletteAction::Tunnels,
        ),
        (
            tr(lang, "lock_shell"),
            tr(lang, "cmd_lock_desc"),
            PaletteAction::Lock,
        ),
    ];
    let query = app.command_palette_query.to_ascii_lowercase();
    let mut filtered = commands
        .into_iter()
        .filter(|(label, description, _)| {
            query.is_empty()
                || label.to_ascii_lowercase().contains(&query)
                || description.to_ascii_lowercase().contains(&query)
        })
        .collect::<Vec<_>>();

    egui::Window::new("Command Palette")
        .open(&mut open)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_width(580.0)
        .anchor(Align2::CENTER_TOP, [0.0, 95.0])
        .frame(modal_frame(palette))
        .show(ctx, |ui| {
            let response = TextInput::singleline(&mut app.command_palette_query)
                .desired_size([ui.available_width(), 35.0])
                .hint_text(tr(lang, "type_command"))
                .font(FontId::proportional(13.0))
                .show(ui, palette, lang);
            if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                if let Some((_, _, chosen)) = filtered.first() {
                    action = Some(*chosen);
                }
            }
            if query.is_empty() {
                response.request_focus();
            }
            ui.add_space(4.0);
            for (label, description, chosen) in filtered.drain(..).take(10) {
                let clicked = ui
                    .add_sized(
                        [ui.available_width(), 33.0],
                        egui::Button::new(
                            RichText::new(format!("{label}   ·   {description}"))
                                .size(13.0)
                                .color(palette.text_soft),
                        )
                        .fill(palette.panel)
                        .stroke(Stroke::new(1.0, palette.border)),
                    )
                    .clicked();
                if clicked {
                    action = Some(chosen);
                }
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr(lang, "enter_to_run"))
                    .size(13.0)
                    .monospace()
                    .color(palette.muted),
            );
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                close_requested = true;
            }
        });
    if close_requested {
        open = false;
    }
    app.command_palette_open = open;
    if let Some(action) = action {
        app.command_palette_open = false;
        match action {
            PaletteAction::NewTerminal => app.new_terminal_tab(),
            PaletteAction::NewSftp => app.new_sftp_tab(),
            PaletteAction::NewSession => app.open_new_session(),
            PaletteAction::ToggleTheme => app.toggle_theme(ctx),
            PaletteAction::SplitColumns => app.split_active(Axis::Horizontal),
            PaletteAction::SplitRows => app.split_active(Axis::Vertical),
            PaletteAction::ToggleConsole => {
                app.settings.console_collapsed = !app.settings.console_collapsed;
                app.persist_settings();
            }
            PaletteAction::Settings => app.settings_open = true,
            PaletteAction::Keyring => app.keyring_open = true,
            PaletteAction::Logs => {
                app.console_tab = ConsoleTab::Logs;
                app.settings.console_collapsed = false;
            }
            PaletteAction::Transfers => {
                app.console_tab = ConsoleTab::Transfers;
                app.settings.console_collapsed = false;
            }
            PaletteAction::Tunnels => {
                app.console_tab = ConsoleTab::Tunnels;
                app.settings.console_collapsed = false;
            }
            PaletteAction::Lock => app.lock_now(),
        }
    }
}
