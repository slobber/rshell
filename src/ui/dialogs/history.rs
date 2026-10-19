use std::collections::HashMap;

use super::*;
use crate::app::CommandHistoryDialog;
use crate::storage::CommandHistoryEntry;

const BLOCK_GAP: f32 = 8.0;
const BLOCK_HEIGHT: f32 = 190.0;
const PICKER_ROW_HEIGHT: f32 = 22.0;
const PICKER_ROWS: isize = 5;

struct HistoryGroup {
    session_uid: String,
    is_current: bool,
    started_at: chrono::DateTime<chrono::Utc>,
    cwd: String,
    matched: String,
    matched_index: usize,
    commands: Vec<String>,
}

pub(super) fn command_history_window(
    app: &mut ShellCoreApp,
    ctx: &egui::Context,
    palette: Palette,
    lang: Language,
) {
    let Some(mut dialog) = app.command_history.take() else {
        return;
    };
    let mut chosen = None;
    let (header_close, _) = dialog_window(
        ctx,
        "command_history_dialog",
        tr(lang, "command_history"),
        Icon::Clock,
        760.0,
        Some(560.0),
        true,
        palette,
        |ui| {
            ui.horizontal(|ui| {
                singleline_input(
                    ui,
                    palette,
                    lang,
                    &mut dialog.query,
                    (ui.available_width() - 150.0).max(200.0),
                    false,
                    Some(tr(lang, "command_history_search_hint")),
                );
                ui.checkbox(
                    &mut dialog.same_path_only,
                    RichText::new(tr(lang, "command_history_same_path")).size(13.0),
                );
            });
            ui.add_space(4.0);
            let user = if dialog.user.is_empty() {
                "-"
            } else {
                dialog.user.as_str()
            };
            ui.label(
                RichText::new(format!(
                    "{}: {user}@{}  ·  {}",
                    tr(lang, "command_history_context"),
                    dialog.host_ip,
                    dialog.cwd
                ))
                .size(13.0)
                .monospace()
                .color(palette.muted),
            );
            ui.add_space(8.0);

            if dialog.query.trim().is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(tr(lang, "command_history_empty"))
                            .size(13.0)
                            .color(palette.muted),
                    );
                });
                return;
            }

            let groups = related_groups(&dialog.entries, &dialog);
            if groups.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(tr(lang, "command_history_no_results"))
                            .size(13.0)
                            .color(palette.muted),
                    );
                });
                return;
            }

            let block_width = ((ui.available_width() - BLOCK_GAP * 2.0) / 3.0).max(160.0);
            ScrollArea::vertical().show(ui, |ui| {
                for (row_index, row) in groups.chunks(3).enumerate() {
                    ui.horizontal(|ui| {
                        for (column, group) in row.iter().enumerate() {
                            let ordinal = row_index * 3 + column + 1;
                            if let Some(command) = history_group(
                                ui,
                                &mut dialog,
                                group,
                                ordinal,
                                block_width,
                                palette,
                                lang,
                            ) {
                                chosen = Some(command);
                            }
                            if column + 1 < row.len() {
                                ui.add_space(BLOCK_GAP);
                            }
                        }
                    });
                    ui.add_space(6.0);
                }
            });
        },
    );
    if header_close {
        return;
    }
    if let Some(command) = chosen {
        if app.insert_history_command(dialog.tab_id, command) {
            return;
        }
    }
    app.command_history = Some(dialog);
}

fn history_group(
    ui: &mut Ui,
    dialog: &mut CommandHistoryDialog,
    group: &HistoryGroup,
    ordinal: usize,
    width: f32,
    palette: Palette,
    lang: Language,
) -> Option<String> {
    let mut chosen = None;
    ui.allocate_ui_with_layout(
        Vec2::new(width, BLOCK_HEIGHT),
        Layout::top_down(Align::Min),
        |ui| {
            Frame::new()
                .fill(palette.panel)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(CornerRadius::same(5))
                .inner_margin(Margin::same(7))
                .show(ui, |ui| {
                    ui.set_width((width - 14.0).max(120.0));
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
                    ui.horizontal(|ui| {
                        let session_label = if group.is_current {
                            tr(lang, "command_history_current_session").to_owned()
                        } else {
                            format!("{} {ordinal}", tr(lang, "command_history_session"))
                        };
                        ui.label(
                            RichText::new(session_label)
                                .size(13.0)
                                .strong()
                                .color(palette.text),
                        );
                        ui.label(
                            RichText::new(group.started_at.format("%m-%d %H:%M").to_string())
                                .size(13.0)
                                .monospace()
                                .color(palette.muted),
                        );
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new(&group.cwd)
                                .size(13.0)
                                .monospace()
                                .color(palette.cyan),
                        )
                        .truncate(),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!(
                                "{}: {}",
                                tr(lang, "command_history_after"),
                                group.matched
                            ))
                            .size(13.0)
                            .monospace()
                            .color(palette.muted),
                        )
                        .truncate(),
                    );

                    let (picker_rect, picker_response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), PICKER_ROWS as f32 * PICKER_ROW_HEIGHT),
                        egui::Sense::hover(),
                    );
                    let max_selection = group.commands.len().saturating_sub(1);
                    let default_selection = group.matched_index.min(max_selection);
                    let selection_key = format!("{}\u{0}{}", group.session_uid, dialog.query);
                    let selected = dialog
                        .selections
                        .entry(selection_key)
                        .or_insert(default_selection);
                    if picker_response.hovered() {
                        let scroll = ui.input(|input| {
                            input
                                .events
                                .iter()
                                .filter_map(|event| match event {
                                    egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                                    _ => None,
                                })
                                .sum::<f32>()
                        });
                        if scroll < -0.001 {
                            *selected = (*selected + 1).min(max_selection);
                        } else if scroll > 0.001 {
                            *selected = selected.saturating_sub(1);
                        }
                        ui.ctx()
                            .input_mut(|input| input.smooth_scroll_delta = Vec2::ZERO);
                    }
                    *selected = (*selected).min(max_selection);

                    for offset in -(PICKER_ROWS / 2)..=(PICKER_ROWS / 2) {
                        let row_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                picker_rect.left(),
                                picker_rect.top()
                                    + (offset + PICKER_ROWS / 2) as f32 * PICKER_ROW_HEIGHT,
                            ),
                            Vec2::new(picker_rect.width(), PICKER_ROW_HEIGHT),
                        );
                        let is_center = offset == 0;
                        if is_center {
                            ui.painter().rect_filled(
                                row_rect.shrink2(Vec2::new(0.0, 1.0)),
                                CornerRadius::same(4),
                                palette.accent_soft,
                            );
                        }
                        let Ok(index) = usize::try_from(*selected as isize + offset) else {
                            continue;
                        };
                        let Some(command) = group.commands.get(index) else {
                            continue;
                        };
                        let row_response = ui.interact(
                            row_rect,
                            egui::Id::unique((
                                "history_picker",
                                group.session_uid.as_str(),
                                offset,
                            )),
                            if is_center {
                                egui::Sense::click()
                            } else {
                                egui::Sense::hover()
                            },
                        );
                        ui.painter()
                            .with_clip_rect(row_rect.shrink2(Vec2::new(3.0, 0.0)))
                            .text(
                                egui::pos2(row_rect.left() + 4.0, row_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                command,
                                FontId::monospace(13.0),
                                if is_center {
                                    palette.text
                                } else {
                                    palette.muted
                                },
                            );
                        if is_center && row_response.clicked() {
                            chosen = Some(command.clone());
                        }
                    }
                });
        },
    );
    chosen
}

fn related_groups(
    entries: &[CommandHistoryEntry],
    dialog: &CommandHistoryDialog,
) -> Vec<HistoryGroup> {
    let query = dialog.query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut sessions: HashMap<String, Vec<&CommandHistoryEntry>> = HashMap::new();
    for entry in entries {
        if entry.host_ip != dialog.host_ip
            || entry.user != dialog.user
            || (dialog.same_path_only && entry.cwd != dialog.cwd)
        {
            continue;
        }
        sessions
            .entry(entry.session_uid.clone())
            .or_default()
            .push(entry);
    }
    let mut groups = Vec::new();
    for (session_uid, list) in sessions {
        let Some(position) = list
            .iter()
            .rposition(|entry| entry.command.to_ascii_lowercase().contains(&query))
        else {
            continue;
        };
        let commands = list
            .iter()
            .map(|entry| entry.command.clone())
            .collect::<Vec<_>>();
        groups.push(HistoryGroup {
            is_current: session_uid == dialog.current_session_uid,
            session_uid,
            started_at: list[0].created_at,
            cwd: list[position].cwd.clone(),
            matched: list[position].command.clone(),
            matched_index: position,
            commands,
        });
    }
    groups.sort_by_key(|group| std::cmp::Reverse(group.started_at));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn entry(id: i64, session_uid: &str, cwd: &str, command: &str) -> CommandHistoryEntry {
        CommandHistoryEntry {
            id,
            session_uid: session_uid.to_owned(),
            host_ip: "10.0.0.8".to_owned(),
            user: "root".to_owned(),
            cwd: cwd.to_owned(),
            command: command.to_owned(),
            created_at: Utc::now(),
        }
    }

    fn dialog(query: &str, same_path_only: bool) -> CommandHistoryDialog {
        CommandHistoryDialog {
            tab_id: 1,
            query: query.to_owned(),
            entries: Vec::new(),
            current_session_uid: "current".to_owned(),
            host_ip: "10.0.0.8".to_owned(),
            user: "root".to_owned(),
            cwd: "/".to_owned(),
            same_path_only,
            selections: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn collects_context_from_current_and_other_sessions() {
        let entries = vec![
            entry(1, "session-a", "/", "df -h"),
            entry(2, "session-a", "/", "free -h"),
            entry(3, "session-b", "/", "DF -H"),
            entry(4, "session-b", "/", "uptime"),
            entry(5, "current", "/", "df -h"),
            entry(6, "current", "/", "ignored"),
        ];
        let groups = related_groups(&entries, &dialog("df -h", true));
        assert_eq!(groups.len(), 3);
        assert_eq!(
            groups
                .iter()
                .find(|group| group.session_uid == "session-a")
                .unwrap()
                .commands,
            vec!["df -h", "free -h"]
        );
        assert_eq!(
            groups
                .iter()
                .find(|group| group.session_uid == "session-b")
                .unwrap()
                .commands,
            vec!["DF -H", "uptime"]
        );
        assert_eq!(
            groups
                .iter()
                .find(|group| group.session_uid == "current")
                .unwrap()
                .commands,
            vec!["df -h", "ignored"]
        );
        assert!(
            groups
                .iter()
                .find(|group| group.session_uid == "current")
                .unwrap()
                .is_current
        );
    }

    #[test]
    fn uses_latest_match_and_keeps_preceding_commands() {
        let entries = vec![
            entry(1, "session-a", "/", "pwd"),
            entry(2, "session-a", "/", "df -h"),
            entry(3, "session-a", "/", "free -h"),
            entry(4, "session-a", "/", "df -h"),
            entry(5, "session-a", "/", "uptime"),
        ];
        let groups = related_groups(&entries, &dialog("df -h", true));
        let group = &groups[0];
        assert_eq!(group.matched_index, 3);
        assert_eq!(
            group.commands,
            vec!["pwd", "df -h", "free -h", "df -h", "uptime"]
        );
    }

    #[test]
    fn path_filter_excludes_other_directories() {
        let entries = vec![
            entry(1, "session-a", "/srv", "df -h"),
            entry(2, "session-a", "/srv", "free -h"),
        ];
        assert!(related_groups(&entries, &dialog("df -h", true)).is_empty());
        assert_eq!(
            related_groups(&entries, &dialog("df -h", false))[0].commands,
            vec!["df -h", "free -h"]
        );
    }
}
