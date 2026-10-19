use super::*;
use crate::domain::{Host, SftpColumnWidths};
use crate::ui::components::host_icon;
use crate::ui::text_input::TextInput;

#[derive(Default)]
pub(super) struct BrowserAction {
    pub(super) open_dir: Option<String>,
    pub(super) open_file: Option<String>,
    pub(super) go_up: bool,
    pub(super) refresh: bool,
    pub(super) navigate: Option<String>,
    pub(super) source_change: Option<Option<u64>>,
    pub(super) drop_files: Option<SftpDragPayload>,
}

pub(super) fn browser_pane(
    ui: &mut Ui,
    app: &ShellCoreApp,
    side: SftpPaneSide,
    path: &str,
    path_input: &mut String,
    entries: &[FileEntry],
    selected: &mut Vec<String>,
    selection_anchor: &mut Option<usize>,
    columns_state: &mut SftpColumnWidths,
    remote: bool,
    header_host: Option<&Host>,
    source_choices: Option<&[(u64, String)]>,
    include_local: bool,
    loading: bool,
    error: Option<&str>,
    palette: Palette,
) -> BrowserAction {
    let mut source_change = None;
    let width = ui.available_width();
    let (header_rect, _) = ui.allocate_exact_size(Vec2::new(width, 58.0), Sense::hover());
    ui.painter()
        .rect_filled(header_rect, CornerRadius::ZERO, palette.panel);
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        Stroke::new(1.0, palette.border),
    );
    let selector_width = if source_choices.is_some() { 190.0 } else { 0.0 };
    let field_top = header_rect.top() + 5.0;
    let field_height = 29.0;
    let field_center = field_top + field_height / 2.0;
    let icon = header_host.map(host_icon).unwrap_or(Icon::Desktop);
    let icon_rect = Rect::from_center_size(
        egui::pos2(header_rect.left() + 14.0, field_center),
        Vec2::splat(20.0),
    );
    crate::icons::paint(
        ui,
        icon,
        icon_rect,
        if header_host.is_some() {
            palette.cyan
        } else {
            palette.accent
        },
    );
    let path_left = if source_choices.is_some() {
        header_rect.left() + 25.0 + selector_width + 8.0
    } else {
        header_rect.left() + 68.0
    };
    if let Some(choices) = source_choices {
        let selected_label = header_host
            .map(|host| host.name.clone())
            .unwrap_or_else(|| app.t("local").to_owned());
        let selector_rect = Rect::from_min_size(
            egui::pos2(header_rect.left() + 25.0, field_top),
            Vec2::new(selector_width, field_height),
        );
        let response = ui.interact(
            selector_rect,
            egui::Id::unique(("sftp_source", side)),
            Sense::click(),
        );
        ui.painter().rect_filled(
            selector_rect,
            CornerRadius::same(4),
            if response.hovered() {
                palette.hover
            } else {
                palette.raised
            },
        );
        ui.painter().rect_stroke(
            selector_rect,
            CornerRadius::same(4),
            Stroke::new(1.0, palette.border),
            egui::StrokeKind::Inside,
        );
        let label_clip = Rect::from_min_max(
            egui::pos2(selector_rect.left() + 8.0, selector_rect.top()),
            egui::pos2(selector_rect.right() - 26.0, selector_rect.bottom()),
        );
        ui.painter().with_clip_rect(label_clip).text(
            egui::pos2(selector_rect.left() + 8.0, field_center),
            egui::Align2::LEFT_CENTER,
            selected_label,
            FontId::monospace(13.0),
            palette.text_soft,
        );
        crate::icons::paint(
            ui,
            Icon::ChevronDown,
            Rect::from_center_size(
                egui::pos2(selector_rect.right() - 12.0, field_center),
                Vec2::splat(12.0),
            ),
            palette.muted,
        );
        egui::Popup::menu(&response)
            .width(selector_width)
            .show(|ui| {
                ui.set_min_width(selector_width);
                if include_local
                    && ui
                        .selectable_label(header_host.is_none(), app.t("local"))
                        .clicked()
                {
                    source_change = Some(None);
                    ui.close();
                }
                for (host_id, name) in choices {
                    if ui
                        .selectable_label(header_host.is_some_and(|host| host.id == *host_id), name)
                        .clicked()
                    {
                        source_change = Some(Some(*host_id));
                        ui.close();
                    }
                }
            });
    } else {
        ui.painter().text(
            egui::pos2(header_rect.left() + 25.0, field_center),
            egui::Align2::LEFT_CENTER,
            if remote {
                app.t("remote")
            } else {
                app.t("local")
            },
            FontId::proportional(13.0),
            palette.text,
        );
    }
    let path_rect = Rect::from_min_max(
        egui::pos2(path_left, field_top),
        egui::pos2(header_rect.right() - 76.0, field_top + field_height),
    );
    let path_response = TextInput::singleline(path_input)
        .frame(egui::Frame::NONE)
        .font(FontId::monospace(13.0))
        .text_color(palette.text_soft)
        .put(ui, path_rect, palette, app.settings.language);
    let navigate_requested =
        path_response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    let up_rect = Rect::from_center_size(
        egui::pos2(header_rect.right() - 48.0, field_center),
        Vec2::splat(24.0),
    );
    let refresh_rect = Rect::from_center_size(
        egui::pos2(header_rect.right() - 18.0, field_center),
        Vec2::splat(24.0),
    );
    let up_response = ui.interact(
        up_rect,
        egui::Id::unique(("sftp_up", side, path)),
        Sense::click(),
    );
    let refresh_response = ui.interact(
        refresh_rect,
        egui::Id::unique(("sftp_refresh", side, path)),
        Sense::click(),
    );
    crate::icons::paint(
        ui,
        Icon::ArrowUp,
        up_rect.shrink(3.0),
        if up_response.hovered() {
            palette.text
        } else {
            palette.muted
        },
    );
    crate::icons::paint(
        ui,
        Icon::Refresh,
        refresh_rect.shrink(3.0),
        if refresh_response.hovered() {
            palette.text
        } else {
            palette.muted
        },
    );
    let footer = if let Some(error) = error {
        error.to_owned()
    } else if loading {
        app.t("loading").to_owned()
    } else {
        format!(
            "{} {} · {}",
            entries.len(),
            app.t("items"),
            if remote { "SFTP" } else { "LOCAL" }
        )
    };
    ui.painter().text(
        egui::pos2(header_rect.left() + 9.0, header_rect.top() + 47.0),
        egui::Align2::LEFT_CENTER,
        footer,
        FontId::monospace(13.0),
        if error.is_some() {
            palette.red
        } else {
            palette.muted
        },
    );

    table_header(
        ui,
        width,
        side,
        remote,
        palette,
        app.settings.language,
        columns_state,
    );
    let table_columns = columns(width, remote, *columns_state);
    let mut action = BrowserAction {
        source_change,
        ..Default::default()
    };
    if navigate_requested && !path_input.trim().is_empty() {
        action.navigate = Some(path_input.trim().to_owned());
    }
    let row_height = 28.0;
    let table_height = ui.available_height();
    ui.allocate_ui_with_layout(
        Vec2::new(width, table_height),
        Layout::top_down(Align::Min),
        |ui| {
            let (_, dropped) = ui.dnd_drop_zone::<SftpDragPayload, _>(
                egui::Frame::new().inner_margin(egui::Margin::ZERO),
                |ui| {
                    ScrollArea::vertical()
                        .id_salt(("file_list", side, path))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(width);
                            if entries.is_empty() {
                                ui.add_space(26.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(app.t("empty_directory"))
                                            .size(13.0)
                                            .color(palette.muted),
                                    );
                                });
                            }
                            for (index, entry) in entries.iter().enumerate() {
                                let selected_row = selected.iter().any(|name| name == &entry.name);
                                let response = file_row(
                                    ui,
                                    entry,
                                    selected_row,
                                    remote,
                                    &table_columns,
                                    width,
                                    row_height,
                                    palette,
                                );
                                if response.clicked() {
                                    let modifiers = ui.input(|input| input.modifiers);
                                    update_selection(
                                        selected,
                                        selection_anchor,
                                        entries,
                                        index,
                                        modifiers.shift,
                                        modifiers.ctrl || modifiers.command,
                                    );
                                }
                                if response.drag_started() && !selected_row {
                                    selected.clear();
                                    selected.push(entry.name.clone());
                                    *selection_anchor = Some(index);
                                }
                                let drag_names = if selected.iter().any(|name| name == &entry.name)
                                {
                                    selected.clone()
                                } else {
                                    vec![entry.name.clone()]
                                };
                                response.dnd_set_drag_payload(SftpDragPayload {
                                    source: side,
                                    names: drag_names,
                                });
                                if response.double_clicked() {
                                    if entry.is_dir {
                                        action.open_dir = Some(entry.name.clone());
                                    } else {
                                        action.open_file = Some(entry.name.clone());
                                    }
                                }
                            }
                        });
                },
            );
            action.drop_files = dropped.map(|payload| (*payload).clone());
        },
    );
    if up_response.clicked() {
        action.go_up = true;
    }
    if refresh_response.clicked() {
        action.refresh = true;
    }
    action
}

fn table_header(
    ui: &mut Ui,
    width: f32,
    side: SftpPaneSide,
    remote: bool,
    palette: Palette,
    language: crate::i18n::Language,
    columns_state: &mut SftpColumnWidths,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.canvas);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1.0, palette.border),
    );
    let columns = columns(width, remote, *columns_state);
    let header_font = FontId::monospace(13.0);
    let mut x = rect.left() + 7.0;
    ui.painter().text(
        egui::pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        crate::i18n::tr(language, "name"),
        header_font.clone(),
        palette.muted,
    );
    x += columns.name;
    if remote {
        ui.painter().text(
            egui::pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            crate::i18n::tr(language, "perm"),
            header_font.clone(),
            palette.muted,
        );
        x += columns.perm;
        if columns.owner > 0.0 {
            ui.painter().text(
                egui::pos2(x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                crate::i18n::tr(language, "owner"),
                header_font.clone(),
                palette.muted,
            );
            x += columns.owner;
        }
    }
    ui.painter().text(
        egui::pos2(x + columns.size - 4.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        crate::i18n::tr(language, "size"),
        header_font.clone(),
        palette.muted,
    );
    x += columns.size;
    if columns.date > 0.0 {
        ui.painter().text(
            egui::pos2(x + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            crate::i18n::tr(language, "modified"),
            header_font,
            palette.muted,
        );
    }

    let resize = |x: f32, id: u8, adjust: &mut dyn FnMut(f32)| {
        let handle = Rect::from_center_size(
            egui::pos2(x, rect.center().y),
            Vec2::new(7.0, rect.height()),
        );
        let response = ui.interact(
            handle,
            egui::Id::unique(("sftp_col_resize", side, id)),
            Sense::drag(),
        );
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            ui.painter()
                .vline(x, rect.y_range(), Stroke::new(2.0, palette.accent));
        }
        if response.dragged() {
            adjust(ui.input(|input| input.pointer.delta().x));
        }
    };

    if remote {
        let name_end = rect.left() + 7.0 + columns.name;
        resize(name_end, 0, &mut |delta| {
            columns_state.perm = (columns_state.perm - delta).clamp(50.0, 180.0);
        });
        if columns.owner > 0.0 {
            let owner_end = name_end + columns.perm + columns.owner;
            resize(owner_end, 1, &mut |delta| {
                columns_state.owner = (columns_state.owner + delta).clamp(50.0, 180.0);
            });
        }
        if columns.date > 0.0 {
            let size_end = name_end + columns.perm + columns.owner + columns.size;
            resize(size_end, 2, &mut |delta| {
                columns_state.size = (columns_state.size + delta).clamp(50.0, 140.0);
            });
        }
    } else if columns.date > 0.0 {
        let size_end = rect.left() + 7.0 + columns.name + columns.size;
        resize(size_end, 0, &mut |delta| {
            columns_state.size = (columns_state.size + delta).clamp(50.0, 140.0);
        });
    }
}

struct Columns {
    name: f32,
    perm: f32,
    owner: f32,
    size: f32,
    date: f32,
}

fn columns(width: f32, remote: bool, state: SftpColumnWidths) -> Columns {
    if remote {
        let perm = if width > 360.0 { state.perm } else { 0.0 };
        let owner = if width > 450.0 { state.owner } else { 0.0 };
        let size = state.size;
        let date = if width > 480.0 { state.date } else { 0.0 };
        Columns {
            name: (width - perm - owner - size - date - 12.0).max(65.0),
            perm,
            owner,
            size,
            date,
        }
    } else {
        let size = state.size;
        let date = if width > 440.0 { state.date } else { 0.0 };
        Columns {
            name: (width - size - date - 12.0).max(65.0),
            perm: 0.0,
            owner: 0.0,
            size,
            date,
        }
    }
}

fn update_selection(
    selected: &mut Vec<String>,
    anchor: &mut Option<usize>,
    entries: &[FileEntry],
    index: usize,
    extend_range: bool,
    toggle: bool,
) {
    let Some(entry) = entries.get(index) else {
        return;
    };
    if extend_range {
        if let Some(anchor) = *anchor {
            let start = anchor.min(index);
            let end = anchor.max(index);
            selected.clear();
            selected.extend(entries[start..=end].iter().map(|entry| entry.name.clone()));
            return;
        }
    }
    if toggle {
        if let Some(position) = selected.iter().position(|name| name == &entry.name) {
            selected.remove(position);
        } else {
            selected.push(entry.name.clone());
        }
    } else {
        selected.clear();
        selected.push(entry.name.clone());
    }
    *anchor = Some(index);
}

fn paint_ellipsized(
    ui: &Ui,
    pos: egui::Pos2,
    max_width: f32,
    text: &str,
    font: FontId,
    color: egui::Color32,
) {
    let full = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), color);
    let galley = if full.size().x <= max_width {
        full
    } else {
        let chars = text.chars().collect::<Vec<_>>();
        let mut low = 0;
        let mut high = chars.len();
        while low < high {
            let mid = (low + high).div_ceil(2);
            let candidate = format!("{}…", chars.iter().take(mid).collect::<String>());
            if ui
                .painter()
                .layout_no_wrap(candidate, font.clone(), color)
                .size()
                .x
                <= max_width
            {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        let candidate = format!("{}…", chars.iter().take(low).collect::<String>());
        ui.painter().layout_no_wrap(candidate, font, color)
    };
    ui.painter().galley(
        egui::pos2(pos.x, pos.y - galley.size().y / 2.0),
        galley,
        color,
    );
}

fn file_row(
    ui: &mut Ui,
    entry: &FileEntry,
    selected: bool,
    remote: bool,
    columns: &Columns,
    width: f32,
    height: f32,
    palette: Palette,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.raised);
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(2.0, rect.height())),
            CornerRadius::ZERO,
            palette.accent,
        );
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.hover);
    }
    let icon = if entry.is_dir {
        Icon::Folder
    } else if entry.archive {
        Icon::FileZip
    } else if entry.name.ends_with(".conf") || entry.name.ends_with(".yaml") {
        Icon::FileCode
    } else {
        Icon::FileText
    };
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 15.0, rect.center().y),
        Vec2::splat(18.0),
    );
    crate::icons::paint(
        ui,
        icon,
        icon_rect,
        if entry.is_dir {
            palette.purple
        } else {
            palette.accent
        },
    );
    paint_ellipsized(
        ui,
        egui::pos2(rect.left() + 23.0, rect.center().y),
        (columns.name - 16.0).max(20.0),
        &entry.name,
        FontId::proportional(13.0),
        if selected {
            palette.accent
        } else {
            palette.text
        },
    );
    let mut x = rect.left() + 7.0 + columns.name;
    let mono = FontId::monospace(13.0);
    if remote {
        if columns.perm > 0.0 {
            ui.painter().text(
                egui::pos2(x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &entry.perm,
                mono.clone(),
                palette.text_soft,
            );
            x += columns.perm;
        }
        if columns.owner > 0.0 {
            ui.painter().text(
                egui::pos2(x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &entry.owner,
                mono.clone(),
                palette.muted,
            );
            x += columns.owner;
        }
    }
    ui.painter().text(
        egui::pos2(x + columns.size - 4.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        entry.size_label(),
        mono.clone(),
        palette.muted,
    );
    x += columns.size;
    if columns.date > 0.0 {
        ui.painter().text(
            egui::pos2(x + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &entry.modified,
            mono,
            palette.muted,
        );
    }
    response
}

pub(super) fn parent_path(path: &str, remote: bool) -> String {
    if path == "~" || path == "/" {
        return path.to_owned();
    }
    if remote {
        let trimmed = path.trim_end_matches('/');
        if let Some((parent, _)) = trimmed.rsplit_once('/') {
            if parent.is_empty() {
                "/".to_owned()
            } else {
                format!("{parent}/")
            }
        } else {
            "/".to_owned()
        }
    } else if let Some((parent, _)) = path.rsplit_once('\\') {
        if parent.is_empty() {
            "~".to_owned()
        } else {
            parent.to_owned()
        }
    } else {
        "~".to_owned()
    }
}

pub(super) fn join_path(parent: &str, name: &str) -> String {
    if parent.ends_with('/') {
        format!("{parent}{name}")
    } else if parent.contains('\\') {
        format!("{parent}\\{name}")
    } else {
        format!("{parent}/{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::update_selection;
    use crate::domain::FileEntry;

    fn entry(name: &str) -> FileEntry {
        FileEntry {
            name: name.to_owned(),
            is_dir: false,
            size: 0,
            modified: String::new(),
            perm: String::new(),
            owner: String::new(),
            archive: false,
            symlink: false,
        }
    }

    #[test]
    fn selection_supports_toggle_and_range() {
        let entries = vec![entry("a"), entry("b"), entry("c"), entry("d")];
        let mut selected = Vec::new();
        let mut anchor = None;

        update_selection(&mut selected, &mut anchor, &entries, 0, false, false);
        assert_eq!(selected, ["a"]);

        update_selection(&mut selected, &mut anchor, &entries, 2, false, true);
        assert_eq!(selected, ["a", "c"]);

        update_selection(&mut selected, &mut anchor, &entries, 0, true, false);
        assert_eq!(selected, ["a", "b", "c"]);

        update_selection(&mut selected, &mut anchor, &entries, 1, false, true);
        assert_eq!(selected, ["a", "c"]);

        update_selection(&mut selected, &mut anchor, &entries, 3, false, false);
        assert_eq!(selected, ["d"]);
        assert_eq!(anchor, Some(3));
    }
}
