use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, Frame, Layout, Margin, Order, Pos2, Rect,
    RichText, Sense, Stroke, Ui, Vec2, ViewportCommand,
};

use crate::app::{AppLock, ConsoleTab, ShellCoreApp};
use crate::domain::{Axis, HostStatus, TransferDirection};
use crate::icons::Icon;
use crate::theme::Palette;
use crate::ui::components::{combo_icon, icon_button, labeled_icon_button, status_dot};

mod menu;
mod ribbon;
mod status;
mod titlebar;

pub use menu::menus;
pub use ribbon::quick_ribbon;
pub use status::{resize_handles, status_bar};
pub use titlebar::title_bar;
