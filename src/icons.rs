use eframe::egui::{self, Color32, Response, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Activity,
    AlertTriangle,
    ArrowDown,
    ArrowUp,
    ArrowsExchange,
    Bolt,
    Broadcast,
    BrandAlma,
    BrandAlpine,
    BrandApple,
    BrandArchLinux,
    BrandCentOs,
    BrandDebian,
    BrandDeepin,
    BrandFedora,
    BrandKylin,
    BrandLinux,
    BrandOpenSuse,
    BrandPowerShell,
    BrandRedHat,
    BrandRocky,
    BrandUbuntu,
    BrandUnionTech,
    BrandWindows,
    ChartBar,
    ChevronDown,
    ChevronRight,
    ChevronUp,
    ChevronsLeft,
    ChevronsRight,
    Circle,
    CircleCheck,
    CircleDot,
    CirclesRelation,
    Clock,
    CloudDownload,
    CloudUpload,
    Clipboard,
    Code,
    Copy,
    Database,
    Desktop,
    Eye,
    File,
    FileCode,
    FileText,
    FileZip,
    Folder,
    FolderOpen,
    FolderPlus,
    Info,
    Key,
    Keyboard,
    LayoutColumns,
    LayoutGrid,
    LayoutRows,
    LayoutSidebar,
    LayoutSidebarCollapseLeft,
    LayoutSidebarCollapseRight,
    Lock,
    LockOpen,
    Login,
    Logout,
    Maximize,
    Minimize,
    Minus,
    Moon,
    Network,
    Pencil,
    Plug,
    PlugX,
    Plus,
    Refresh,
    Redo,
    Search,
    SelectAll,
    Server,
    Server2,
    Settings,
    ShieldLock,
    Square,
    Sun,
    SwitchHorizontal,
    Terminal,
    TerminalWindow,
    Trash,
    Undo,
    User,
    WindowMaximize,
    World,
    X,
    Scissors,
}

impl Icon {
    pub fn source(self) -> egui::ImageSource<'static> {
        use Icon::*;
        match self {
            Activity => egui::include_image!("../assets/icons/activity.svg"),
            AlertTriangle => egui::include_image!("../assets/icons/alert-triangle.svg"),
            ArrowDown => egui::include_image!("../assets/icons/arrow-down.svg"),
            ArrowUp => egui::include_image!("../assets/icons/arrow-up.svg"),
            ArrowsExchange => egui::include_image!("../assets/icons/arrows-exchange.svg"),
            Bolt => egui::include_image!("../assets/icons/bolt.svg"),
            Broadcast => egui::include_image!("../assets/icons/broadcast.svg"),
            BrandAlma => egui::include_image!("../assets/icons/brand-alma.svg"),
            BrandAlpine => egui::include_image!("../assets/icons/brand-alpine.svg"),
            BrandApple => egui::include_image!("../assets/icons/brand-apple.svg"),
            BrandArchLinux => egui::include_image!("../assets/icons/brand-archlinux.svg"),
            BrandCentOs => egui::include_image!("../assets/icons/brand-centos.svg"),
            BrandDebian => egui::include_image!("../assets/icons/brand-debian.svg"),
            BrandDeepin => egui::include_image!("../assets/icons/brand-deepin.svg"),
            BrandFedora => egui::include_image!("../assets/icons/brand-fedora.svg"),
            BrandKylin => egui::include_image!("../assets/icons/brand-kylin.svg"),
            BrandLinux => egui::include_image!("../assets/icons/brand-linux.svg"),
            BrandOpenSuse => egui::include_image!("../assets/icons/brand-opensuse.svg"),
            BrandPowerShell => egui::include_image!("../assets/icons/brand-powershell.svg"),
            BrandRedHat => egui::include_image!("../assets/icons/brand-redhat.svg"),
            BrandRocky => egui::include_image!("../assets/icons/brand-rocky.svg"),
            BrandUbuntu => egui::include_image!("../assets/icons/brand-ubuntu.svg"),
            BrandUnionTech => egui::include_image!("../assets/icons/brand-uniontech.svg"),
            BrandWindows => egui::include_image!("../assets/icons/brand-windows.svg"),
            ChartBar => egui::include_image!("../assets/icons/chart-bar.svg"),
            ChevronDown => egui::include_image!("../assets/icons/chevron-down.svg"),
            ChevronRight => egui::include_image!("../assets/icons/chevron-right.svg"),
            ChevronUp => egui::include_image!("../assets/icons/chevron-up.svg"),
            ChevronsLeft => egui::include_image!("../assets/icons/chevrons-left.svg"),
            ChevronsRight => egui::include_image!("../assets/icons/chevrons-right.svg"),
            Circle => egui::include_image!("../assets/icons/circle.svg"),
            CircleCheck => egui::include_image!("../assets/icons/circle-check.svg"),
            CircleDot => egui::include_image!("../assets/icons/circle-dot.svg"),
            CirclesRelation => egui::include_image!("../assets/icons/circles-relation.svg"),
            Clock => egui::include_image!("../assets/icons/clock.svg"),
            CloudDownload => egui::include_image!("../assets/icons/cloud-download.svg"),
            CloudUpload => egui::include_image!("../assets/icons/cloud-upload.svg"),
            Clipboard => egui::include_image!("../assets/icons/clipboard.svg"),
            Code => egui::include_image!("../assets/icons/code.svg"),
            Copy => egui::include_image!("../assets/icons/copy.svg"),
            Database => egui::include_image!("../assets/icons/database.svg"),
            Desktop => egui::include_image!("../assets/icons/device-desktop.svg"),
            Eye => egui::include_image!("../assets/icons/eye.svg"),
            File => egui::include_image!("../assets/icons/file.svg"),
            FileCode => egui::include_image!("../assets/icons/file-code.svg"),
            FileText => egui::include_image!("../assets/icons/file-text.svg"),
            FileZip => egui::include_image!("../assets/icons/file-zip.svg"),
            Folder => egui::include_image!("../assets/icons/folder.svg"),
            FolderOpen => egui::include_image!("../assets/icons/folder-open.svg"),
            FolderPlus => egui::include_image!("../assets/icons/folder-plus.svg"),
            Info => egui::include_image!("../assets/icons/info-circle.svg"),
            Key => egui::include_image!("../assets/icons/key.svg"),
            Keyboard => egui::include_image!("../assets/icons/keyboard.svg"),
            LayoutColumns => egui::include_image!("../assets/icons/layout-columns.svg"),
            LayoutGrid => egui::include_image!("../assets/icons/layout-grid.svg"),
            LayoutRows => egui::include_image!("../assets/icons/layout-rows.svg"),
            LayoutSidebar => egui::include_image!("../assets/icons/layout-sidebar.svg"),
            LayoutSidebarCollapseLeft => {
                egui::include_image!("../assets/icons/layout-sidebar-left-collapse.svg")
            }
            LayoutSidebarCollapseRight => {
                egui::include_image!("../assets/icons/layout-sidebar-right-collapse.svg")
            }
            Lock => egui::include_image!("../assets/icons/lock.svg"),
            LockOpen => egui::include_image!("../assets/icons/lock-open.svg"),
            Login => egui::include_image!("../assets/icons/login-2.svg"),
            Logout => egui::include_image!("../assets/icons/logout.svg"),
            Maximize => egui::include_image!("../assets/icons/maximize.svg"),
            Minimize => egui::include_image!("../assets/icons/minimize.svg"),
            Minus => egui::include_image!("../assets/icons/minus.svg"),
            Moon => egui::include_image!("../assets/icons/moon.svg"),
            Network => egui::include_image!("../assets/icons/network.svg"),
            Pencil => egui::include_image!("../assets/icons/pencil.svg"),
            Plug => egui::include_image!("../assets/icons/plug-connected.svg"),
            PlugX => egui::include_image!("../assets/icons/plug-connected-x.svg"),
            Plus => egui::include_image!("../assets/icons/plus.svg"),
            Refresh => egui::include_image!("../assets/icons/refresh.svg"),
            Redo => egui::include_image!("../assets/icons/redo.svg"),
            Search => egui::include_image!("../assets/icons/search.svg"),
            SelectAll => egui::include_image!("../assets/icons/select-all.svg"),
            Server => egui::include_image!("../assets/icons/server.svg"),
            Server2 => egui::include_image!("../assets/icons/server-2.svg"),
            Settings => egui::include_image!("../assets/icons/settings.svg"),
            ShieldLock => egui::include_image!("../assets/icons/shield-lock.svg"),
            Square => egui::include_image!("../assets/icons/square.svg"),
            Sun => egui::include_image!("../assets/icons/sun.svg"),
            SwitchHorizontal => {
                egui::include_image!("../assets/icons/switch-horizontal.svg")
            }
            Terminal => egui::include_image!("../assets/icons/terminal.svg"),
            TerminalWindow => egui::include_image!("../assets/icons/terminal-2.svg"),
            Trash => egui::include_image!("../assets/icons/trash.svg"),
            Undo => egui::include_image!("../assets/icons/undo.svg"),
            User => egui::include_image!("../assets/icons/user.svg"),
            WindowMaximize => egui::include_image!("../assets/icons/window-maximize.svg"),
            World => egui::include_image!("../assets/icons/world.svg"),
            X => egui::include_image!("../assets/icons/x.svg"),
            Scissors => egui::include_image!("../assets/icons/scissors.svg"),
        }
    }
}

pub fn image(ui: &mut Ui, icon: Icon, size: f32, color: Color32) -> Response {
    let size = size.max(18.0);
    ui.add(
        egui::Image::new(icon.source())
            .fit_to_exact_size(Vec2::splat(size))
            .tint(color),
    )
}

pub fn app_logo(ui: &mut Ui, size: f32) -> Response {
    let size = size.max(18.0);
    ui.add(
        egui::Image::new(egui::include_image!("../assets/app-icon.png"))
            .fit_to_exact_size(Vec2::splat(size))
            .sense(egui::Sense::click()),
    )
}

pub fn app_logo_muted(ui: &mut Ui, size: f32) -> Response {
    let size = size.max(18.0);
    ui.add(
        egui::Image::new(egui::include_image!("../assets/app-icon-muted.png"))
            .fit_to_exact_size(Vec2::splat(size)),
    )
}

pub fn paint(ui: &Ui, icon: Icon, rect: egui::Rect, tint: Color32) {
    egui::Image::new(icon.source())
        .fit_to_exact_size(rect.size())
        .tint(tint)
        .paint_at(ui, rect);
}

#[cfg(test)]
mod tests {
    #[test]
    fn svg_icons_are_tintable_and_use_consistent_strokes() {
        let icon_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
        let mut checked = 0;
        for entry in std::fs::read_dir(icon_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|value| value.to_str()) != Some("svg") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(
                !source.contains("currentColor"),
                "{} must use a white base for egui tinting",
                path.display()
            );
            assert!(
                source.contains("stroke=\"#ffffff\"") || source.contains("fill=\"#ffffff\""),
                "{} must use a white stroke or fill",
                path.display()
            );
            if source.contains("stroke-width=") {
                assert!(
                    source.contains("stroke-width=\"1.25\""),
                    "{} must use stroke-width 1.25",
                    path.display()
                );
            }
            checked += 1;
        }
        assert!(checked >= 60, "expected the vendored Tabler icon set");
    }

    #[test]
    fn application_icon_assets_exist() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(root.join("assets/app-icon.png").is_file());
        assert!(root.join("assets/app-icon-muted.png").is_file());
        let ico = std::fs::read(root.join("assets/app-icon.ico")).unwrap();
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 7);
    }

    #[test]
    fn domestic_os_brand_icons_rasterize_visible_shapes() {
        let icon_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
        for name in ["brand-deepin.svg", "brand-uniontech.svg", "brand-kylin.svg"] {
            let path = icon_dir.join(name);
            let data = std::fs::read(&path).unwrap();
            let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default())
                .unwrap_or_else(|error| panic!("{} failed to parse: {error}", path.display()));
            let size = tree.size().to_int_size();
            let mut pixmap = resvg::tiny_skia::Pixmap::new(160, 160).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(
                    160.0 / size.width() as f32,
                    160.0 / size.height() as f32,
                ),
                &mut pixmap.as_mut(),
            );
            let visible = pixmap
                .pixels()
                .iter()
                .filter(|pixel| pixel.alpha() > 24)
                .count();
            assert!(
                visible > 160 * 160 / 20,
                "{} rasterized almost nothing ({visible} visible pixels)",
                path.display()
            );
        }
    }
}
