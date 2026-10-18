use super::*;
use crate::domain::ConnectionBackend;
use crate::ui::components::{
    context_menu_item, context_menu_separator, host_icon, prepare_context_menu,
};

pub(super) fn draw_folder(
    ui: &mut Ui,
    folder: &mut HostFolder,
    depth: usize,
    palette: Palette,
    language: Language,
    search: &str,
    selected: &mut Option<u64>,
) -> FolderAction {
    if !folder_matches(folder, search) {
        return FolderAction::default();
    }
    let payload = HostTreeDrag {
        item: TreeItem::Folder(folder.id),
        title: folder.title.clone(),
    };
    let mut dropped_item = None;
    let (zone, dropped) =
        ui.dnd_drop_zone::<HostTreeDrag, _>(Frame::new().inner_margin(Margin::ZERO), |ui| {
            let width = ui.available_width();
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::click_and_drag());
            let x = rect.left() + 7.0 + depth as f32 * 14.0;
            if response.hovered() {
                ui.painter().rect_filled(
                    rect.shrink2(Vec2::new(2.0, 0.0)),
                    CornerRadius::same(3),
                    palette.hover,
                );
            }
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(x + 6.0, rect.center().y),
                Vec2::splat(11.0),
            );
            crate::icons::paint(
                ui,
                if folder.expanded {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                },
                icon_rect,
                palette.muted,
            );
            let folder_rect = egui::Rect::from_center_size(
                egui::pos2(x + 23.0, rect.center().y),
                Vec2::splat(18.0),
            );
            crate::icons::paint(
                ui,
                if folder.expanded {
                    Icon::FolderOpen
                } else {
                    Icon::Folder
                },
                folder_rect,
                palette.purple,
            );
            let title_clip = egui::Rect::from_min_max(
                egui::pos2(x + 33.0, rect.top()),
                egui::pos2(rect.right() - 86.0, rect.bottom()),
            );
            ui.painter().with_clip_rect(title_clip).text(
                egui::pos2(x + 33.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &folder.title,
                FontId::proportional(13.0),
                palette.text,
            );
            ui.painter().text(
                egui::pos2(rect.right() - 9.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                if language == Language::ZhCn {
                    format!("{} 台主机", folder.host_count())
                } else {
                    format!("{} hosts", folder.host_count())
                },
                FontId::monospace(13.0),
                palette.muted,
            );
            response
        });
    let response = zone.inner;
    response.dnd_set_drag_payload(payload);
    if let Some(payload) = dropped {
        dropped_item = Some(payload.item);
    }
    if response.clicked() {
        folder.expanded = !folder.expanded;
    }

    let mut action = FolderAction {
        action: HostTreeAction::default(),
        changed: response.clicked(),
        visible: true,
    };
    response.context_menu(|ui| {
        prepare_context_menu(ui, palette);
        if context_menu_item(
            ui,
            palette,
            Icon::Pencil,
            crate::i18n::tr(language, "rename"),
            "",
            true,
            false,
        ) {
            action.action.rename_folder = Some(folder.id);
            ui.close();
        }
        context_menu_separator(ui, palette);
        if context_menu_item(
            ui,
            palette,
            Icon::Trash,
            crate::i18n::tr(language, "delete"),
            "",
            true,
            true,
        ) {
            action.action.delete_folder = Some(folder.id);
            ui.close();
        }
    });
    if let Some(item) = dropped_item {
        if item != TreeItem::Folder(folder.id) {
            action.action.move_item = Some((item, folder.id));
        }
    }
    if folder.expanded || !search.is_empty() {
        for host in &folder.hosts {
            if host_matches(host, search) {
                let host_action = draw_host(ui, host, depth + 1, palette, language, selected);
                merge_action(&mut action.action, host_action);
            }
        }
        for child in &mut folder.children {
            let child_action =
                draw_folder(ui, child, depth + 1, palette, language, search, selected);
            action.visible |= child_action.visible;
            action.changed |= child_action.changed;
            merge_action(&mut action.action, child_action.action);
        }
    }
    action
}

pub(super) struct FolderAction {
    pub(super) action: HostTreeAction,
    pub(super) visible: bool,
    pub(super) changed: bool,
}

impl Default for FolderAction {
    fn default() -> Self {
        Self {
            action: HostTreeAction::default(),
            visible: false,
            changed: false,
        }
    }
}

pub(super) fn draw_host(
    ui: &mut Ui,
    host: &Host,
    depth: usize,
    palette: Palette,
    language: Language,
    selected: &mut Option<u64>,
) -> HostTreeAction {
    let payload = HostTreeDrag {
        item: TreeItem::Host(host.id),
        title: host.name.clone(),
    };
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 24.0), Sense::click_and_drag());
    let is_selected = *selected == Some(host.id);
    if is_selected {
        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(2.0, 0.0)),
            CornerRadius::same(3),
            palette.raised,
        );
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(rect.left() + 2.0, rect.top() + 2.0),
                egui::pos2(rect.left() + 4.0, rect.bottom() - 2.0),
            ),
            CornerRadius::ZERO,
            palette.accent,
        );
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(2.0, 0.0)),
            CornerRadius::same(3),
            palette.hover,
        );
    }
    let x = rect.left() + 12.0 + depth as f32 * 12.0;
    let dot_color = match host.status {
        HostStatus::Online => palette.green,
        HostStatus::Connecting => palette.amber,
        HostStatus::Offline => palette.muted,
        HostStatus::Error => palette.red,
    };
    ui.painter()
        .circle_filled(egui::pos2(x + 3.0, rect.center().y), 2.5, dot_color);
    let icon_rect =
        egui::Rect::from_center_size(egui::pos2(x + 15.0, rect.center().y), Vec2::splat(16.0));
    crate::icons::paint(
        ui,
        host_icon(host),
        icon_rect,
        if is_selected {
            palette.accent
        } else {
            palette.text_soft
        },
    );
    let clip = egui::Rect::from_min_max(
        egui::pos2(x + 24.0, rect.top()),
        egui::pos2(rect.right() - 48.0, rect.bottom()),
    );
    ui.painter().with_clip_rect(clip).text(
        egui::pos2(x + 24.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &host.name,
        FontId::proportional(13.0),
        if is_selected {
            palette.accent
        } else {
            palette.text
        },
    );
    let (badge, badge_color) = if host.backend == crate::domain::ConnectionBackend::LocalWsl {
        ("WSL", palette.cyan)
    } else if host.backend == crate::domain::ConnectionBackend::Rdp {
        ("RDP", palette.cyan)
    } else {
        ("SSH", palette.cyan)
    };
    let badge_font = FontId::monospace(10.0);
    let badge_galley =
        ui.painter()
            .layout_no_wrap(badge.to_owned(), badge_font.clone(), badge_color);
    let badge_width = badge_galley.size().x + 10.0;
    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - badge_width - 5.0, rect.center().y - 8.0),
        Vec2::new(badge_width, 16.0),
    );
    ui.painter()
        .rect_filled(badge_rect, CornerRadius::same(3), palette.canvas);
    ui.painter().rect_stroke(
        badge_rect,
        CornerRadius::same(3),
        Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        badge_rect.center(),
        egui::Align2::CENTER_CENTER,
        badge,
        badge_font,
        badge_color,
    );
    response.dnd_set_drag_payload(payload);
    if response.clicked() {
        *selected = Some(host.id);
    }
    if response.double_clicked() {
        *selected = Some(host.id);
        return HostTreeAction {
            selected: Some(host.id),
            open_terminal: Some(host.id),
            ..Default::default()
        };
    }
    let mut action = HostTreeAction::default();
    response.context_menu(|ui| {
        prepare_context_menu(ui, palette);
        if context_menu_item(
            ui,
            palette,
            Icon::Terminal,
            crate::i18n::tr(language, "open_terminal"),
            "",
            true,
            false,
        ) {
            action.open_terminal = Some(host.id);
            ui.close();
        }
        if host.backend == ConnectionBackend::Ssh
            && context_menu_item(
                ui,
                palette,
                Icon::FolderOpen,
                crate::i18n::tr(language, "open_sftp"),
                "",
                true,
                false,
            )
        {
            action.open_sftp = Some(host.id);
            ui.close();
        }
        context_menu_separator(ui, palette);
        if context_menu_item(
            ui,
            palette,
            Icon::Settings,
            crate::i18n::tr(language, "edit_connection"),
            "",
            true,
            false,
        ) {
            action.edit = Some(host.id);
            ui.close();
        }
    });
    action
}

pub(super) fn merge_action(target: &mut HostTreeAction, source: HostTreeAction) {
    target.selected = source.selected.or(target.selected);
    target.open_terminal = source.open_terminal.or(target.open_terminal);
    target.open_sftp = source.open_sftp.or(target.open_sftp);
    target.edit = source.edit.or(target.edit);
    target.rename_folder = source.rename_folder.or(target.rename_folder);
    target.delete_folder = source.delete_folder.or(target.delete_folder);
    target.move_item = source.move_item.or(target.move_item);
    target.changed |= source.changed;
}

fn folder_matches(folder: &HostFolder, search: &str) -> bool {
    search.is_empty()
        || folder
            .title
            .to_ascii_lowercase()
            .contains(&search.to_ascii_lowercase())
        || folder.hosts.iter().any(|host| host_matches(host, search))
        || folder
            .children
            .iter()
            .any(|child| folder_matches(child, search))
}

fn host_matches(host: &Host, search: &str) -> bool {
    search.is_empty()
        || host
            .name
            .to_ascii_lowercase()
            .contains(&search.to_ascii_lowercase())
        || host.ip.contains(search)
}
