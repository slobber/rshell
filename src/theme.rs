use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

use crate::domain::{TerminalColorTheme, ThemeMode};

/// Default minimum for labels, controls, hints and feedback. Compact status
/// metadata can use [`STATUS_FONT_SIZE`].
pub const MIN_UI_FONT_SIZE: f32 = 13.0;
const UI_ZOOM_BASELINE: f32 = 14.0;

/// Font size for dense status metadata in the ribbon and status strip.
pub const STATUS_FONT_SIZE: f32 = 11.0;

pub const UI_FONT_SIZE_MIN: f32 = 12.0;
pub const UI_FONT_SIZE_MAX: f32 = 18.0;

pub fn apply_ui_font_size(ctx: &egui::Context, font_size: f32) {
    let font_size = font_size.clamp(UI_FONT_SIZE_MIN, UI_FONT_SIZE_MAX);
    ctx.set_zoom_factor(font_size / UI_ZOOM_BASELINE);
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub canvas: Color32,
    pub surface: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub raised: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_soft: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub green: Color32,
    pub cyan: Color32,
    pub amber: Color32,
    pub red: Color32,
    pub purple: Color32,
    pub terminal_bg: Color32,
    pub terminal_text: Color32,
    pub selection: Color32,
}

impl Palette {
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                canvas: Color32::from_rgb(13, 13, 28),
                surface: Color32::from_rgb(18, 18, 33),
                panel: Color32::from_rgb(26, 26, 42),
                panel_alt: Color32::from_rgb(30, 30, 46),
                raised: Color32::from_rgb(37, 37, 56),
                hover: Color32::from_rgb(49, 50, 68),
                border: Color32::from_rgb(49, 50, 68),
                border_strong: Color32::from_rgb(69, 71, 90),
                text: Color32::from_rgb(227, 224, 247),
                text_soft: Color32::from_rgb(195, 198, 210),
                muted: Color32::from_rgb(147, 153, 178),
                accent: Color32::from_rgb(137, 180, 250),
                accent_soft: Color32::from_rgb(35, 55, 84),
                on_accent: Color32::from_rgb(8, 27, 51),
                green: Color32::from_rgb(166, 227, 161),
                cyan: Color32::from_rgb(137, 220, 235),
                amber: Color32::from_rgb(249, 226, 175),
                red: Color32::from_rgb(243, 139, 168),
                purple: Color32::from_rgb(203, 166, 247),
                terminal_bg: Color32::from_rgb(13, 13, 28),
                terminal_text: Color32::from_rgb(205, 214, 244),
                selection: Color32::from_rgb(49, 75, 113),
            },
            ThemeMode::Light => Self {
                canvas: Color32::from_rgb(243, 246, 251),
                surface: Color32::from_rgb(248, 250, 252),
                panel: Color32::from_rgb(241, 245, 249),
                panel_alt: Color32::from_rgb(255, 255, 255),
                raised: Color32::from_rgb(226, 232, 240),
                hover: Color32::from_rgb(219, 234, 254),
                border: Color32::from_rgb(226, 232, 240),
                border_strong: Color32::from_rgb(203, 213, 225),
                text: Color32::from_rgb(15, 23, 42),
                text_soft: Color32::from_rgb(51, 65, 85),
                muted: Color32::from_rgb(100, 116, 139),
                accent: Color32::from_rgb(37, 99, 235),
                accent_soft: Color32::from_rgb(219, 234, 254),
                on_accent: Color32::from_rgb(255, 255, 255),
                green: Color32::from_rgb(5, 150, 105),
                cyan: Color32::from_rgb(2, 132, 199),
                amber: Color32::from_rgb(217, 119, 6),
                red: Color32::from_rgb(225, 29, 72),
                purple: Color32::from_rgb(124, 58, 237),
                terminal_bg: Color32::from_rgb(250, 252, 255),
                terminal_text: Color32::from_rgb(15, 23, 42),
                selection: Color32::from_rgb(191, 219, 254),
            },
        }
    }

    pub fn log_color(self, level: crate::domain::LogLevel) -> Color32 {
        use crate::domain::LogLevel;
        match level {
            LogLevel::Info => self.cyan,
            LogLevel::Success => self.green,
            LogLevel::Warning => self.amber,
            LogLevel::Error => self.red,
            LogLevel::Transfer => self.purple,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalPalette {
    pub background: Color32,
    pub foreground: Color32,
    pub cursor: Color32,
    pub selection: Color32,
    pub colors: [Color32; 16],
}

impl TerminalPalette {
    pub fn selection_fill(self) -> Color32 {
        let mix = |base: u8, accent: u8| ((u16::from(base) + u16::from(accent)) / 2) as u8;
        let alpha = 150_u16;
        let composite = |background: u8, accent: u8| {
            ((u16::from(background) * (255 - alpha) + u16::from(accent) * alpha) / 255) as u8
        };
        let selection = [
            mix(self.selection.r(), self.cursor.r()),
            mix(self.selection.g(), self.cursor.g()),
            mix(self.selection.b(), self.cursor.b()),
        ];
        Color32::from_rgb(
            composite(self.background.r(), selection[0]),
            composite(self.background.g(), selection[1]),
            composite(self.background.b(), selection[2]),
        )
    }

    pub fn for_theme(theme: TerminalColorTheme) -> Self {
        match theme {
            TerminalColorTheme::RShellDark => Self {
                background: Color32::from_rgb(13, 13, 28),
                foreground: Color32::from_rgb(205, 214, 244),
                cursor: Color32::from_rgb(137, 180, 250),
                selection: Color32::from_rgb(49, 75, 113),
                colors: [
                    Color32::from_rgb(30, 30, 46),
                    Color32::from_rgb(243, 139, 168),
                    Color32::from_rgb(166, 227, 161),
                    Color32::from_rgb(249, 226, 175),
                    Color32::from_rgb(137, 180, 250),
                    Color32::from_rgb(203, 166, 247),
                    Color32::from_rgb(137, 220, 235),
                    Color32::from_rgb(205, 214, 244),
                    Color32::from_rgb(88, 91, 112),
                    Color32::from_rgb(243, 139, 168),
                    Color32::from_rgb(166, 227, 161),
                    Color32::from_rgb(249, 226, 175),
                    Color32::from_rgb(137, 180, 250),
                    Color32::from_rgb(203, 166, 247),
                    Color32::from_rgb(137, 220, 235),
                    Color32::from_rgb(245, 245, 255),
                ],
            },
            TerminalColorTheme::RShellLight => Self {
                background: Color32::from_rgb(250, 252, 255),
                foreground: Color32::from_rgb(30, 41, 59),
                cursor: Color32::from_rgb(37, 99, 235),
                selection: Color32::from_rgb(191, 219, 254),
                colors: [
                    Color32::from_rgb(51, 65, 85),
                    Color32::from_rgb(185, 28, 28),
                    Color32::from_rgb(21, 128, 61),
                    Color32::from_rgb(161, 98, 7),
                    Color32::from_rgb(29, 78, 216),
                    Color32::from_rgb(147, 51, 234),
                    Color32::from_rgb(14, 116, 144),
                    Color32::from_rgb(100, 116, 139),
                    Color32::from_rgb(100, 116, 139),
                    Color32::from_rgb(220, 38, 38),
                    Color32::from_rgb(22, 163, 74),
                    Color32::from_rgb(202, 138, 4),
                    Color32::from_rgb(37, 99, 235),
                    Color32::from_rgb(168, 85, 247),
                    Color32::from_rgb(8, 145, 178),
                    Color32::from_rgb(15, 23, 42),
                ],
            },
            TerminalColorTheme::SolarizedDark => Self {
                background: Color32::from_rgb(0, 43, 54),
                foreground: Color32::from_rgb(131, 148, 150),
                cursor: Color32::from_rgb(147, 161, 161),
                selection: Color32::from_rgb(7, 54, 66),
                colors: [
                    Color32::from_rgb(7, 54, 66),
                    Color32::from_rgb(220, 50, 47),
                    Color32::from_rgb(133, 153, 0),
                    Color32::from_rgb(181, 137, 0),
                    Color32::from_rgb(38, 139, 210),
                    Color32::from_rgb(211, 54, 130),
                    Color32::from_rgb(42, 161, 152),
                    Color32::from_rgb(238, 232, 213),
                    Color32::from_rgb(0, 43, 54),
                    Color32::from_rgb(203, 75, 22),
                    Color32::from_rgb(88, 110, 117),
                    Color32::from_rgb(101, 123, 131),
                    Color32::from_rgb(131, 148, 150),
                    Color32::from_rgb(108, 113, 196),
                    Color32::from_rgb(147, 161, 161),
                    Color32::from_rgb(253, 246, 227),
                ],
            },
            TerminalColorTheme::Dracula => Self {
                background: Color32::from_rgb(40, 42, 54),
                foreground: Color32::from_rgb(248, 248, 242),
                cursor: Color32::from_rgb(255, 121, 198),
                selection: Color32::from_rgb(68, 71, 90),
                colors: [
                    Color32::from_rgb(33, 34, 44),
                    Color32::from_rgb(255, 85, 85),
                    Color32::from_rgb(80, 250, 123),
                    Color32::from_rgb(241, 250, 140),
                    Color32::from_rgb(139, 233, 253),
                    Color32::from_rgb(255, 121, 198),
                    Color32::from_rgb(139, 233, 253),
                    Color32::from_rgb(248, 248, 242),
                    Color32::from_rgb(98, 114, 164),
                    Color32::from_rgb(255, 110, 110),
                    Color32::from_rgb(105, 255, 148),
                    Color32::from_rgb(255, 255, 165),
                    Color32::from_rgb(154, 237, 254),
                    Color32::from_rgb(255, 146, 208),
                    Color32::from_rgb(154, 237, 254),
                    Color32::from_rgb(255, 255, 255),
                ],
            },
            TerminalColorTheme::Paper => Self {
                background: Color32::from_rgb(253, 246, 227),
                foreground: Color32::from_rgb(88, 110, 117),
                cursor: Color32::from_rgb(38, 139, 210),
                selection: Color32::from_rgb(238, 232, 213),
                colors: [
                    Color32::from_rgb(7, 54, 66),
                    Color32::from_rgb(220, 50, 47),
                    Color32::from_rgb(133, 153, 0),
                    Color32::from_rgb(181, 137, 0),
                    Color32::from_rgb(38, 139, 210),
                    Color32::from_rgb(211, 54, 130),
                    Color32::from_rgb(42, 161, 152),
                    Color32::from_rgb(238, 232, 213),
                    Color32::from_rgb(0, 43, 54),
                    Color32::from_rgb(203, 75, 22),
                    Color32::from_rgb(88, 110, 117),
                    Color32::from_rgb(101, 123, 131),
                    Color32::from_rgb(131, 148, 150),
                    Color32::from_rgb(108, 113, 196),
                    Color32::from_rgb(147, 161, 161),
                    Color32::from_rgb(253, 246, 227),
                ],
            },
        }
    }

    pub fn color(self, index: u8) -> Color32 {
        self.colors[usize::from(index.min(15))]
    }
}

pub fn install_theme(ctx: &egui::Context, mode: ThemeMode) {
    let p = Palette::for_mode(mode);
    let egui_theme = match mode {
        ThemeMode::Dark => egui::Theme::Dark,
        ThemeMode::Light => egui::Theme::Light,
    };
    let mut style = (*ctx.style_of(egui_theme)).clone();
    style.visuals = match mode {
        ThemeMode::Dark => Visuals::dark(),
        ThemeMode::Light => Visuals::light(),
    };
    style.visuals.override_text_color = Some(p.text);
    style.visuals.panel_fill = p.surface;
    style.visuals.window_fill = p.panel_alt;
    style.visuals.extreme_bg_color = p.canvas;
    style.visuals.text_edit_bg_color = Some(p.canvas);
    style.visuals.faint_bg_color = p.panel;
    style.visuals.code_bg_color = p.panel;
    style.visuals.window_stroke = Stroke::new(1.0, p.border_strong);
    style.visuals.window_corner_radius = CornerRadius::same(8);
    style.visuals.menu_corner_radius = CornerRadius::same(6);
    style.visuals.selection.bg_fill = p.selection;
    style.visuals.selection.stroke = Stroke::new(1.0, p.accent);
    style.visuals.hyperlink_color = p.accent;
    style.visuals.warn_fg_color = p.amber;
    style.visuals.error_fg_color = p.red;

    style.visuals.widgets.noninteractive.bg_fill = p.panel;
    style.visuals.widgets.noninteractive.weak_bg_fill = p.panel;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    style.visuals.widgets.inactive.bg_fill = p.raised;
    style.visuals.widgets.inactive.weak_bg_fill = p.panel_alt;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text_soft);
    style.visuals.widgets.hovered.bg_fill = p.hover;
    style.visuals.widgets.hovered.weak_bg_fill = p.hover;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, p.border_strong);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    style.visuals.widgets.active.bg_fill = p.accent_soft;
    style.visuals.widgets.active.weak_bg_fill = p.accent_soft;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, p.accent);
    style.visuals.widgets.open = style.visuals.widgets.hovered;
    for widget in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(4);
        widget.expansion = 0.0;
    }

    style.spacing.item_spacing = egui::vec2(7.0, 5.0);
    style.spacing.button_padding = egui::vec2(9.0, 4.0);
    style.spacing.interact_size.y = 28.0;
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.floating = true;
    style.spacing.indent = 14.0;
    style.interaction.selectable_labels = false;
    style.interaction.multi_widget_text_select = false;
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(18.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(MIN_UI_FONT_SIZE, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(MIN_UI_FONT_SIZE, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(MIN_UI_FONT_SIZE, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(MIN_UI_FONT_SIZE, FontFamily::Proportional),
        ),
    ]
    .into();
    ctx.set_style_of(egui_theme, style);
    ctx.set_theme(egui_theme);
}

#[cfg(test)]
mod tests {
    use super::{install_theme, TerminalColorTheme, TerminalPalette, ThemeMode};

    #[test]
    fn labels_are_not_selectable_by_default() {
        let ctx = eframe::egui::Context::default();
        install_theme(&ctx, ThemeMode::Dark);
        let style = ctx.style_of(eframe::egui::Theme::Dark);
        assert!(!style.interaction.selectable_labels);
        assert!(!style.interaction.multi_widget_text_select);
    }

    #[test]
    fn terminal_selection_is_visible_in_every_theme() {
        for theme in [
            TerminalColorTheme::RShellDark,
            TerminalColorTheme::RShellLight,
            TerminalColorTheme::SolarizedDark,
            TerminalColorTheme::Dracula,
            TerminalColorTheme::Paper,
        ] {
            let palette = TerminalPalette::for_theme(theme);
            let fill = palette.selection_fill();
            let background = palette.background;
            let difference = (i16::from(fill.r()) - i16::from(background.r())).unsigned_abs()
                + (i16::from(fill.g()) - i16::from(background.g())).unsigned_abs()
                + (i16::from(fill.b()) - i16::from(background.b())).unsigned_abs();
            assert!(
                difference > 100,
                "selection fill should contrast with {theme:?}"
            );
        }
    }
}
