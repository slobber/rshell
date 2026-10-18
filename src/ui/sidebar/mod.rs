use eframe::egui::{
    self, Align, CornerRadius, FontId, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke,
    Ui, Vec2,
};

use crate::app::{HostTreeDrag, ShellCoreApp, SidebarTab, TreeItem};
use crate::domain::{Host, HostFolder, HostStatus, TransferStatus, TunnelState};
use crate::i18n::Language;
use crate::icons::Icon;
use crate::theme::Palette;
use crate::ui::components::{icon_button, progress_bar, status_dot};
use crate::ui::text_input::TextInput;

mod panels;
mod tree;

#[derive(Default)]
struct HostTreeAction {
    selected: Option<u64>,
    open_terminal: Option<u64>,
    open_sftp: Option<u64>,
    edit: Option<u64>,
    rename_folder: Option<u64>,
    delete_folder: Option<u64>,
    move_item: Option<(TreeItem, u64)>,
    changed: bool,
}

pub fn draw(app: &mut ShellCoreApp, ui: &mut Ui) {
    let palette = Palette::for_mode(app.settings.theme);
    if app.settings.sidebar_collapsed {
        draw_collapsed(app, ui, palette);
    } else {
        draw_expanded(app, ui, palette);
    }
}

fn draw_collapsed(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    egui::Panel::left("session_sidebar_collapsed")
        .exact_size(43.0)
        .resizable(false)
        .frame(
            Frame::new()
                .fill(palette.panel_alt)
                .stroke(Stroke::new(1.0, palette.border))
                .inner_margin(Margin::symmetric(4, 8)),
        )
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                if crate::icons::app_logo(ui, 20.0)
                    .on_hover_text("RShell")
                    .clicked()
                {
                    app.settings.sidebar_collapsed = false;
                    app.persist_settings();
                }
                ui.add_space(10.0);
                if icon_button(
                    ui,
                    palette,
                    Icon::Server,
                    18.0,
                    app.t("hosts"),
                    app.sidebar_tab == SidebarTab::Hosts,
                )
                .clicked()
                {
                    app.sidebar_tab = SidebarTab::Hosts;
                    app.settings.sidebar_collapsed = false;
                    app.persist_settings();
                }
                if icon_button(
                    ui,
                    palette,
                    Icon::Network,
                    18.0,
                    app.t("tunnels"),
                    app.sidebar_tab == SidebarTab::Tunnels,
                )
                .clicked()
                {
                    app.sidebar_tab = SidebarTab::Tunnels;
                    app.settings.sidebar_collapsed = false;
                    app.persist_settings();
                }
                if icon_button(
                    ui,
                    palette,
                    Icon::SwitchHorizontal,
                    18.0,
                    app.t("sftp_queue"),
                    app.sidebar_tab == SidebarTab::Queue,
                )
                .clicked()
                {
                    app.sidebar_tab = SidebarTab::Queue;
                    app.settings.sidebar_collapsed = false;
                    app.persist_settings();
                }
            });
        });
}

fn draw_expanded(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    let mut add_host = false;
    let mut add_folder = false;
    let mut selected = app.active_host_id;
    let mut tree_action = HostTreeAction::default();

    egui::Panel::left("session_sidebar_v2")
        .exact_size(240.0)
        .resizable(false)
        .frame(
            Frame::new()
                .fill(palette.panel_alt)
                .inner_margin(Margin::ZERO),
        )
        .show(ui, |ui| {
            header(app, ui, palette);
            toolbar(app, ui, palette, &mut add_host, &mut add_folder);
            sidebar_tabs(app, ui, palette);

            let properties_height = if app.sidebar_tab == SidebarTab::Hosts {
                190.0
            } else {
                0.0
            };
            let body_height = (ui.available_height() - properties_height).max(120.0);
            let body_width = ui.available_width();
            ui.allocate_ui_with_layout(
                Vec2::new(body_width, body_height),
                Layout::top_down(Align::Min),
                |ui| {
                    ScrollArea::vertical()
                        .id_salt("sidebar_tree_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(body_width);
                            ui.spacing_mut().item_spacing.y = 1.0;
                            match app.sidebar_tab {
                                SidebarTab::Hosts => {
                                    if app.hosts.is_empty() {
                                        empty_tree(app, ui, palette);
                                    } else {
                                        let mut any_visible = false;
                                        for folder in &mut app.hosts {
                                            let action = tree::draw_folder(
                                                ui,
                                                folder,
                                                0,
                                                palette,
                                                app.settings.language,
                                                &app.host_search,
                                                &mut selected,
                                            );
                                            any_visible |= action.visible;
                                            tree_action.changed |= action.changed;
                                            tree::merge_action(&mut tree_action, action.action);
                                        }
                                        if !any_visible && !app.host_search.is_empty() {
                                            ui.add_space(18.0);
                                            ui.vertical_centered(|ui| {
                                                ui.label(
                                                    RichText::new(app.t("no_hosts_match"))
                                                        .size(13.0)
                                                        .color(palette.muted),
                                                );
                                            });
                                        }
                                    }
                                }
                                SidebarTab::Tunnels => panels::draw_tunnels(app, ui, palette),
                                SidebarTab::Queue => panels::draw_queue(app, ui, palette),
                            }
                        });
                },
            );
            if app.sidebar_tab == SidebarTab::Hosts {
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.set_width(ui.available_width());
                    panels::properties(app, ui, palette);
                });
            }
        });

    if let Some(id) = tree_action.selected {
        app.active_host_id = Some(id);
    }
    if let Some(id) = tree_action.open_terminal {
        app.active_host_id = Some(id);
        app.ensure_connected(id);
        app.new_terminal_tab_for_host(id);
    }
    if let Some(id) = tree_action.open_sftp {
        app.new_sftp_tab_for_host(id);
    }
    if let Some(id) = tree_action.edit {
        app.open_edit_host(id);
    }
    if let Some((item, target_folder)) = tree_action.move_item {
        app.move_tree_item(item, target_folder);
    }
    if let Some(id) = tree_action.rename_folder {
        app.open_rename_folder(id);
    }
    if let Some(id) = tree_action.delete_folder {
        app.folder_delete = Some(id);
    }
    if tree_action.changed {
        app.persist_hosts();
    }
    if add_host {
        app.open_new_session();
    }
    if add_folder {
        app.folder_dialog = Some(crate::app::FolderDraft::default());
    }
}

fn header(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 50.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 18.0, rect.center().y),
        Vec2::splat(18.0),
    );
    crate::icons::paint(ui, Icon::Server, icon_rect, palette.accent);
    ui.painter().text(
        egui::pos2(rect.left() + 32.0, rect.top() + 15.0),
        egui::Align2::LEFT_CENTER,
        app.t("session_manager"),
        FontId::proportional(13.0),
        palette.text,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 32.0, rect.top() + 33.0),
        egui::Align2::LEFT_CENTER,
        format!(
            "{} {} · {}",
            app.connected_hosts.len(),
            app.t("active_sessions"),
            app.t("hosts_count").replace(
                "{count}",
                &app.hosts
                    .iter()
                    .map(|folder| folder.host_count())
                    .sum::<usize>()
                    .to_string()
            )
        ),
        FontId::monospace(13.0),
        palette.muted,
    );
    let collapse = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 16.0, rect.center().y),
        Vec2::splat(25.0),
    );
    let response = ui.interact(
        collapse,
        egui::Id::unique("collapse_sidebar"),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(response.rect, CornerRadius::same(3), palette.hover);
    }
    crate::icons::paint(
        ui,
        Icon::LayoutSidebarCollapseLeft,
        collapse.shrink(5.0),
        palette.text_soft,
    );
    if response.clicked() {
        app.settings.sidebar_collapsed = true;
        app.persist_settings();
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1.0, palette.border),
    );
}

fn toolbar(
    app: &mut ShellCoreApp,
    ui: &mut Ui,
    palette: Palette,
    add_host: &mut bool,
    add_folder: &mut bool,
) {
    Frame::new()
        .fill(palette.panel_alt)
        .inner_margin(Margin::symmetric(4, 4))
        .show(ui, |ui| {
            let content_width = ui.available_width();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let icon_width = 26.0;
                let width = (content_width - icon_width * 3.0 - 14.0).max(80.0);
                let icon = egui::Image::new(Icon::Plus.source())
                    .fit_to_exact_size(Vec2::splat(14.0))
                    .tint(palette.text);
                let response = ui.add_sized(
                    [width, 26.0],
                    egui::Button::image_and_text(
                        icon,
                        RichText::new(app.t("add_host"))
                            .size(13.0)
                            .color(palette.text),
                    )
                    .fill(palette.panel)
                    .stroke(Stroke::new(1.0, palette.border)),
                );
                if response.clicked() {
                    *add_host = true;
                }
                if icon_button(
                    ui,
                    palette,
                    Icon::FolderPlus,
                    14.0,
                    app.t("new_folder"),
                    false,
                )
                .clicked()
                {
                    *add_folder = true;
                }
                if icon_button(
                    ui,
                    palette,
                    Icon::Search,
                    14.0,
                    app.t("search_host"),
                    app.host_search_open,
                )
                .clicked()
                {
                    app.host_search_open = !app.host_search_open;
                    if !app.host_search_open {
                        app.host_search.clear();
                    }
                }
                if icon_button(ui, palette, Icon::Refresh, 14.0, app.t("refresh"), false).clicked()
                {
                    app.toast(app.t("refresh"));
                }
            });
            if app.host_search_open {
                ui.add_space(4.0);
                Frame::new()
                    .fill(palette.canvas)
                    .stroke(Stroke::new(1.0, palette.border))
                    .corner_radius(CornerRadius::same(3))
                    .inner_margin(Margin::symmetric(6, 3))
                    .show(ui, |ui| {
                        ui.set_width(content_width - 12.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            crate::icons::image(ui, Icon::Search, 18.0, palette.muted);
                            let hint = app.t("search_host").to_owned();
                            TextInput::singleline(&mut app.host_search)
                                .desired_size([ui.available_width(), 26.0])
                                .hint_text(hint)
                                .frame(egui::Frame::NONE)
                                .font(egui::TextStyle::Small)
                                .show(ui, palette, app.settings.language);
                        });
                    });
            }
        });
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0, palette.border),
    );
}

fn sidebar_tabs(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    let width = ui.available_width() / 3.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    for (index, tab) in [
        (SidebarTab::Hosts, app.t("hosts"), Icon::Server),
        (SidebarTab::Tunnels, app.t("tunnels_tab"), Icon::Network),
        (
            SidebarTab::Queue,
            app.t("queue_tab"),
            Icon::SwitchHorizontal,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let tab_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + width * index as f32, rect.top()),
            Vec2::new(width, rect.height()),
        );
        let response = ui.interact(
            tab_rect,
            egui::Id::unique(("sidebar_tab", index)),
            Sense::click(),
        );
        let active = app.sidebar_tab == tab.0;
        if active {
            ui.painter()
                .rect_filled(tab_rect, CornerRadius::ZERO, palette.raised);
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    tab_rect.min,
                    egui::pos2(tab_rect.min.x + 2.0, tab_rect.max.y),
                ),
                CornerRadius::ZERO,
                palette.accent,
            );
        } else if response.hovered() {
            ui.painter()
                .rect_filled(tab_rect, CornerRadius::ZERO, palette.hover);
        }
        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(tab_rect.left() + 15.0, tab_rect.center().y),
            Vec2::splat(18.0),
        );
        crate::icons::paint(
            ui,
            tab.2,
            icon_rect,
            if active {
                palette.accent
            } else {
                palette.muted
            },
        );
        ui.painter().text(
            egui::pos2(tab_rect.left() + 25.0, tab_rect.center().y),
            egui::Align2::LEFT_CENTER,
            tab.1,
            FontId::proportional(13.0),
            if active {
                palette.accent
            } else {
                palette.muted
            },
        );
        if response.clicked() {
            app.sidebar_tab = tab.0;
        }
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1.0, palette.border),
    );
}

fn empty_tree(app: &mut ShellCoreApp, ui: &mut Ui, palette: Palette) {
    ui.add_space(34.0);
    ui.vertical_centered(|ui| {
        crate::icons::image(ui, Icon::Server2, 30.0, palette.muted);
        ui.add_space(7.0);
        ui.label(
            RichText::new(app.t("empty_tree_title"))
                .size(13.0)
                .strong()
                .color(palette.text),
        );
    });
}
