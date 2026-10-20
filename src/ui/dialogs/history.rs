use std::collections::HashMap;

use super::*;
use crate::app::CommandHistoryDialog;
use crate::storage::CommandHistoryEntry;

const BLOCK_GAP: f32 = 8.0;
const BLOCK_ROW_GAP: f32 = 4.0;
const BLOCK_HEIGHT: f32 = 182.0;
const MAX_VISIBLE_ROWS: usize = 3;
const HISTORY_BASE_HEIGHT: f32 = 130.0;
const PICKER_ROW_HEIGHT: f32 = 18.0;
const PICKER_ROWS: isize = 5;
const BLOCK_TOOLBAR_WIDTH: f32 = 28.0;
const BLOCK_TOOLBAR_GAP: f32 = 6.0;
const BLOCK_TOOLBAR_BUTTON: f32 = 22.0;

struct HistoryGroup {
    session_uid: String,
    is_current: bool,
    started_at: chrono::DateTime<chrono::Utc>,
    cwd: String,
    matched: String,
    matched_index: usize,
    commands: Vec<String>,
}

#[derive(Default)]
struct HistoryToolbarAction {
    delete: bool,
    previous: bool,
    next: bool,
}

#[derive(Default)]
struct HistoryBlockAction {
    chosen: Option<String>,
    delete_session: bool,
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
    let mut delete_session = None;
    let visible_rows = related_groups(&dialog.entries, &dialog)
        .len()
        .div_ceil(3)
        .clamp(1, MAX_VISIBLE_ROWS);
    let dialog_height = HISTORY_BASE_HEIGHT
        + visible_rows as f32 * BLOCK_HEIGHT
        + (visible_rows.saturating_sub(1)) as f32 * BLOCK_ROW_GAP;
    let (header_close, _) = dialog_window(
        ctx,
        "command_history_dialog",
        tr(lang, "command_history"),
        Icon::Clock,
        760.0,
        Some(dialog_height),
        false,
        palette,
        |ui| {
            let query_before = dialog.query.clone();
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
            let query_changed = dialog.query != query_before;
            ui.add_space(2.0);
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
            ui.add_space(2.0);

            if query_changed {
                dialog.selections.clear();
            }
            let groups = related_groups(&dialog.entries, &dialog);
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

            let scrollbar_allowance = if groups.len() > MAX_VISIBLE_ROWS * 3 {
                18.0
            } else {
                0.0
            };
            let block_width =
                ((ui.available_width() - BLOCK_GAP * 2.0 - scrollbar_allowance) / 3.0).max(160.0);
            let viewport_height = MAX_VISIBLE_ROWS as f32 * BLOCK_HEIGHT
                + (MAX_VISIBLE_ROWS.saturating_sub(1)) as f32 * BLOCK_ROW_GAP;
            let scroll_visibility = if groups.len() > MAX_VISIBLE_ROWS * 3 {
                egui::scroll_area::ScrollBarVisibility::AlwaysVisible
            } else {
                egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded
            };
            ui.style_mut().spacing.scroll.floating = false;
            ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height(viewport_height)
                .scroll_source(egui::scroll_area::ScrollSource::SCROLL_BAR)
                .scroll_bar_visibility(scroll_visibility)
                .show(ui, |ui| {
                    for (row_index, row) in groups.chunks(3).enumerate() {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            for (column, group) in row.iter().enumerate() {
                                let ordinal = row_index * 3 + column + 1;
                                let action = history_group(
                                    ui,
                                    &mut dialog,
                                    group,
                                    ordinal,
                                    block_width,
                                    palette,
                                    lang,
                                );
                                if let Some(command) = action.chosen {
                                    chosen = Some(command);
                                }
                                if action.delete_session {
                                    delete_session = Some(group.session_uid.clone());
                                }
                                if column + 1 < row.len() {
                                    ui.add_space(BLOCK_GAP);
                                }
                            }
                        });
                        ui.add_space(BLOCK_ROW_GAP);
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
    if let Some(session_uid) = delete_session {
        match app.delete_command_history_session(&session_uid) {
            Ok(()) => {
                dialog.entries = app.load_command_history_entries();
                dialog.selections.clear();
            }
            Err(error) => app.storage_error = Some(error),
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
) -> HistoryBlockAction {
    let mut action = HistoryBlockAction::default();
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
                    let max_selection = group.commands.len().saturating_sub(1);
                    let default_selection = group.matched_index.min(max_selection);
                    let key = selection_key(&group.session_uid, &dialog.query);
                    let mut selected = (*dialog.selections.get(&key).unwrap_or(&default_selection))
                        .min(max_selection);

                    ui.horizontal_top(|ui| {
                        let content_width =
                            (ui.available_width() - BLOCK_TOOLBAR_WIDTH - BLOCK_TOOLBAR_GAP)
                                .max(100.0);
                        ui.vertical(|ui| {
                            ui.set_width(content_width);
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
                                    RichText::new(
                                        group.started_at.format("%m-%d %H:%M").to_string(),
                                    )
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
                                Vec2::new(
                                    ui.available_width(),
                                    PICKER_ROWS as f32 * PICKER_ROW_HEIGHT,
                                ),
                                egui::Sense::hover(),
                            );
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
                                    selected = (selected + 1).min(max_selection);
                                } else if scroll > 0.001 {
                                    selected = selected.saturating_sub(1);
                                }
                            }
                            selected = selected.min(max_selection);

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
                                let Ok(index) = usize::try_from(selected as isize + offset) else {
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
                                    action.chosen = Some(command.clone());
                                }
                            }
                        });
                        ui.add_space(BLOCK_TOOLBAR_GAP);
                        ui.vertical(|ui| {
                            ui.set_width(BLOCK_TOOLBAR_WIDTH);
                            let hits = group_hits(group, &dialog.query);
                            let previous = previous_hit(&hits, selected);
                            let next = next_hit(&hits, selected);
                            let toolbar = history_toolbar(
                                ui,
                                palette,
                                lang,
                                true,
                                previous.is_some(),
                                next.is_some(),
                            );
                            if toolbar.previous {
                                if let Some(index) = previous {
                                    selected = index;
                                }
                            } else if toolbar.next {
                                if let Some(index) = next {
                                    selected = index;
                                }
                            }
                            action.delete_session = toolbar.delete;
                        });
                    });
                    dialog.selections.insert(key, selected);
                });
        },
    );
    action
}

fn selection_key(session_uid: &str, query: &str) -> String {
    format!("{session_uid}\u{0}{query}")
}

fn group_hits(group: &HistoryGroup, query: &str) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    group
        .commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            command
                .to_ascii_lowercase()
                .contains(&query)
                .then_some(index)
        })
        .collect()
}

fn previous_hit(hits: &[usize], selected: usize) -> Option<usize> {
    hits.iter()
        .copied()
        .filter(|index| *index < selected)
        .last()
}

fn next_hit(hits: &[usize], selected: usize) -> Option<usize> {
    hits.iter().copied().find(|index| *index > selected)
}

fn history_toolbar(
    ui: &mut Ui,
    palette: Palette,
    lang: Language,
    can_delete: bool,
    can_previous: bool,
    can_next: bool,
) -> HistoryToolbarAction {
    let mut action = HistoryToolbarAction::default();
    Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
            ui.vertical(|ui| {
                action.delete = toolbar_button(
                    ui,
                    palette,
                    Icon::Trash,
                    can_delete,
                    true,
                    tr(lang, "command_history_delete_session"),
                );
                action.previous = toolbar_button(
                    ui,
                    palette,
                    Icon::ChevronUp,
                    can_previous,
                    false,
                    tr(lang, "command_history_previous"),
                );
                action.next = toolbar_button(
                    ui,
                    palette,
                    Icon::ChevronDown,
                    can_next,
                    false,
                    tr(lang, "command_history_next"),
                );
            });
        });
    action
}

fn toolbar_button(
    ui: &mut Ui,
    palette: Palette,
    icon: Icon,
    enabled: bool,
    danger: bool,
    tooltip: &str,
) -> bool {
    let response = ui.allocate_response(Vec2::splat(BLOCK_TOOLBAR_BUTTON), egui::Sense::click());
    let hovered = enabled && response.hovered();
    if hovered {
        ui.painter()
            .rect_filled(response.rect, CornerRadius::same(4), palette.hover);
    }
    let color = if !enabled {
        palette.border_strong
    } else if danger {
        palette.red
    } else if response.hovered() {
        palette.text
    } else {
        palette.text_soft
    };
    let icon_rect = egui::Rect::from_center_size(response.rect.center(), Vec2::splat(14.0));
    crate::icons::paint(ui, icon, icon_rect, color);
    let clicked = enabled && response.clicked();
    response.on_hover_text(tooltip);
    clicked
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

    #[test]
    fn block_navigation_uses_only_its_own_hits() {
        let entries = vec![
            entry(1, "session-a", "/", "pwd"),
            entry(2, "session-a", "/", "df -h"),
            entry(3, "session-a", "/", "free -h"),
            entry(4, "session-a", "/", "df -h"),
            entry(5, "session-a", "/", "uptime"),
            entry(6, "session-b", "/", "df -h"),
        ];
        let dialog = dialog("df", true);
        let groups = related_groups(&entries, &dialog);
        let session_a = groups
            .iter()
            .find(|group| group.session_uid == "session-a")
            .unwrap();
        let hits = group_hits(session_a, "df");
        assert_eq!(hits, vec![1, 3]);
        assert_eq!(previous_hit(&hits, 3), Some(1));
        assert_eq!(next_hit(&hits, 1), Some(3));
        assert!(previous_hit(&hits, 1).is_none());
        assert!(next_hit(&hits, 3).is_none());

        let session_b = groups
            .iter()
            .find(|group| group.session_uid == "session-b")
            .unwrap();
        let hits = group_hits(session_b, "df");
        assert_eq!(hits, vec![0]);
        assert!(previous_hit(&hits, 0).is_none());
        assert!(next_hit(&hits, 0).is_none());
    }
}
