use eframe::egui::{self, Color32, FontSelection, Key, Rect, Response, TextEdit, Ui, Vec2};

use crate::i18n::{tr, Language};
use crate::icons::Icon;
use crate::theme::Palette;
use crate::ui::clipboard::{clipboard_text, copy_text};
use crate::ui::components::{context_menu_item, context_menu_separator, prepare_context_menu};

pub struct TextInput<'a> {
    value: &'a mut String,
    multiline: bool,
    password: bool,
    hint: Option<String>,
    desired_width: Option<f32>,
    desired_size: Option<Vec2>,
    font: Option<FontSelection>,
    text_color: Option<Color32>,
    vertical_align: Option<egui::Align>,
    frame: Option<egui::Frame>,
    code_editor: bool,
}

pub fn singleline(value: &mut String) -> TextInput<'_> {
    TextInput::singleline(value)
}

pub fn multiline(value: &mut String) -> TextInput<'_> {
    TextInput::multiline(value)
}

pub fn singleline_input(
    ui: &mut Ui,
    palette: Palette,
    lang: Language,
    value: &mut String,
    width: f32,
    password: bool,
    hint: Option<&str>,
) -> Response {
    TextInput::singleline(value)
        .desired_width(width)
        .password(password)
        .hint_text_opt(hint)
        .show(ui, palette, lang)
}

impl<'a> TextInput<'a> {
    pub fn singleline(value: &'a mut String) -> Self {
        Self {
            value,
            multiline: false,
            password: false,
            hint: None,
            desired_width: None,
            desired_size: None,
            font: None,
            text_color: None,
            vertical_align: None,
            frame: None,
            code_editor: false,
        }
    }

    pub fn multiline(value: &'a mut String) -> Self {
        Self {
            multiline: true,
            ..Self::singleline(value)
        }
    }

    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    pub fn desired_size(mut self, size: impl Into<Vec2>) -> Self {
        self.desired_size = Some(size.into());
        self
    }

    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    pub fn hint_text(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn hint_text_opt(mut self, hint: Option<&str>) -> Self {
        self.hint = hint.map(str::to_owned);
        self
    }

    pub fn font(mut self, font: impl Into<FontSelection>) -> Self {
        self.font = Some(font.into());
        self
    }

    pub fn text_color(mut self, color: Color32) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn vertical_align(mut self, align: egui::Align) -> Self {
        self.vertical_align = Some(align);
        self
    }

    pub fn frame(mut self, frame: egui::Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    pub fn code_editor(mut self) -> Self {
        self.code_editor = true;
        self
    }

    pub fn show(self, ui: &mut Ui, palette: Palette, lang: Language) -> Response {
        self.render(ui, None, palette, lang)
    }

    pub fn put(self, ui: &mut Ui, rect: Rect, palette: Palette, lang: Language) -> Response {
        self.render(ui, Some(rect), palette, lang)
    }

    fn render(self, ui: &mut Ui, rect: Option<Rect>, palette: Palette, lang: Language) -> Response {
        let value = self.value;
        let mut edit = if self.multiline {
            TextEdit::multiline(&mut *value)
        } else {
            TextEdit::singleline(&mut *value)
        };
        if self.password {
            edit = edit.password(true);
        }
        if let Some(hint) = self.hint.as_deref() {
            edit = edit.hint_text(hint);
        }
        if let Some(width) = self.desired_width {
            edit = edit.desired_width(width);
        }
        if let Some(font) = self.font {
            edit = edit.font(font);
        }
        if let Some(color) = self.text_color {
            edit = edit.text_color(color);
        }
        if let Some(align) = self.vertical_align {
            edit = edit.align(egui::Align2::LEFT_TOP.with_y(align));
        }
        if let Some(frame) = self.frame {
            edit = edit.frame(frame);
        }
        if self.code_editor {
            edit = edit.code_editor();
        }

        let response = if let Some(rect) = rect {
            ui.put(rect, edit)
        } else if let Some(size) = self.desired_size {
            ui.add_sized(size, edit)
        } else {
            edit.show(ui).response.response
        };
        attach_context_menu(
            ui,
            &response,
            value,
            self.password,
            self.multiline,
            palette,
            lang,
        );
        response
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextMenuAction {
    None,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
    Clear,
}

fn attach_context_menu(
    ui: &mut Ui,
    response: &Response,
    value: &mut String,
    password: bool,
    multiline: bool,
    palette: Palette,
    lang: Language,
) {
    if response.secondary_clicked() {
        response.request_focus();
    }

    let id = response.id;
    let state = egui::text_edit::TextEditState::load(ui.ctx(), id);
    let cursor_range = state.as_ref().and_then(|state| state.cursor.char_range());
    let has_selection = cursor_range.is_some_and(|range| !range.is_empty());
    let has_text = !value.is_empty();
    let can_copy = has_text && !password;
    let can_cut = has_selection && !password;
    let current_state = (
        cursor_range.unwrap_or_else(|| {
            egui::text::CCursorRange::one(egui::text::CCursor::new(value.chars().count()))
        }),
        value.clone(),
    );
    let can_undo = state
        .as_ref()
        .is_some_and(|state| state.undoer().has_undo(&current_state));
    let can_redo = state
        .as_ref()
        .is_some_and(|state| state.undoer().has_redo(&current_state));

    let undo_label = tr(lang, "undo");
    let redo_label = tr(lang, "redo");
    let cut_label = tr(lang, "cut");
    let copy_label = tr(lang, "copy");
    let paste_label = tr(lang, "paste");
    let delete_label = tr(lang, "delete");
    let select_all_label = tr(lang, "select_all");
    let clear_label = tr(lang, "clear");
    let mut action = TextMenuAction::None;
    response.context_menu(|ui| {
        prepare_context_menu(ui, palette);
        if context_menu_item(
            ui,
            palette,
            Icon::Undo,
            undo_label,
            "Ctrl+Z",
            can_undo,
            false,
        ) {
            action = TextMenuAction::Undo;
            ui.close();
        }
        if context_menu_item(
            ui,
            palette,
            Icon::Redo,
            redo_label,
            "Ctrl+Y",
            can_redo,
            false,
        ) {
            action = TextMenuAction::Redo;
            ui.close();
        }
        context_menu_separator(ui, palette);
        if context_menu_item(
            ui,
            palette,
            Icon::Scissors,
            cut_label,
            "Ctrl+X",
            can_cut,
            false,
        ) {
            action = TextMenuAction::Cut;
            ui.close();
        }
        if context_menu_item(
            ui,
            palette,
            Icon::Copy,
            copy_label,
            "Ctrl+C",
            can_copy,
            false,
        ) {
            action = TextMenuAction::Copy;
            ui.close();
        }
        if context_menu_item(
            ui,
            palette,
            Icon::Clipboard,
            paste_label,
            "Ctrl+V",
            true,
            false,
        ) {
            action = TextMenuAction::Paste;
            ui.close();
        }
        if context_menu_item(
            ui,
            palette,
            Icon::Trash,
            delete_label,
            "Del",
            has_selection,
            true,
        ) {
            action = TextMenuAction::Delete;
            ui.close();
        }
        context_menu_separator(ui, palette);
        if context_menu_item(
            ui,
            palette,
            Icon::SelectAll,
            select_all_label,
            "Ctrl+A",
            has_text,
            false,
        ) {
            action = TextMenuAction::SelectAll;
            ui.close();
        }
        if context_menu_item(ui, palette, Icon::X, clear_label, "", has_text, false) {
            action = TextMenuAction::Clear;
            ui.close();
        }
    });

    match action {
        TextMenuAction::Undo => undo_or_redo(ui, id, value, false),
        TextMenuAction::Redo => undo_or_redo(ui, id, value, true),
        TextMenuAction::Cut => {
            if can_cut {
                copy_selection_or_all(ui, id, value);
                clear_selection(ui, id, value);
            }
        }
        TextMenuAction::Copy => {
            if can_copy {
                copy_selection_or_all(ui, id, value);
            }
        }
        TextMenuAction::Paste => paste(ui, id, value, multiline),
        TextMenuAction::Delete => clear_selection(ui, id, value),
        TextMenuAction::SelectAll => select_all(ui, id, value),
        TextMenuAction::Clear => clear_value(ui, id, value),
        TextMenuAction::None => {}
    }
    if action != TextMenuAction::None {
        response.request_focus();
    }

    if !response.has_focus() {
        return;
    }
    let (copy_event, cut_event, paste_event, copy_key, cut_key, paste_key) = ui.input(|input| {
        let command = input.modifiers.ctrl || input.modifiers.command;
        (
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Copy)),
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Cut)),
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Paste(_))),
            command && input.key_pressed(Key::C),
            command && input.key_pressed(Key::X),
            command && input.key_pressed(Key::V),
        )
    });
    if !password && copy_key && !copy_event {
        copy_selection_or_all(ui, id, value);
    }
    if !password && cut_key && !cut_event {
        copy_selection_or_all(ui, id, value);
        clear_selection(ui, id, value);
    }
    if paste_key && !paste_event {
        paste(ui, id, value, multiline);
    }
}

fn paste(ui: &mut Ui, id: egui::Id, value: &mut String, multiline: bool) {
    let Some(text) = clipboard_text() else {
        return;
    };
    let text = normalize_paste(text, multiline);
    insert_at_cursor(ui, id, value, &text);
}

fn normalize_paste(text: String, multiline: bool) -> String {
    if multiline {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.replace("\r\n", " ").replace(['\r', '\n'], " ")
    }
}

fn copy_selection_or_all(ui: &Ui, id: egui::Id, value: &str) {
    if let Some(text) = selected_text(ui, id, value).filter(|text| !text.is_empty()) {
        copy_text(ui.ctx(), text);
    }
}

fn select_all(ui: &mut Ui, id: egui::Id, value: &str) {
    if let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), id) {
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::select_all_str(value)));
        state.store(ui.ctx(), id);
    }
}

fn clear_value(ui: &mut Ui, id: egui::Id, value: &mut String) {
    value.clear();
    if let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), id) {
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(0),
            )));
        state.store(ui.ctx(), id);
    }
}

fn undo_or_redo(ui: &mut Ui, id: egui::Id, value: &mut String, redo: bool) {
    let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), id) else {
        return;
    };
    let cursor_range = state.cursor.char_range().unwrap_or_else(|| {
        egui::text::CCursorRange::one(egui::text::CCursor::new(value.chars().count()))
    });
    let current = (cursor_range, value.clone());
    let mut undoer = state.undoer();
    let next = if redo {
        undoer.redo(&current)
    } else {
        undoer.undo(&current)
    }
    .cloned();
    let Some((cursor_range, text)) = next else {
        return;
    };
    *value = text;
    state.cursor.set_char_range(Some(cursor_range));
    state.set_undoer(undoer);
    state.store(ui.ctx(), id);
}

fn selected_range(ui: &Ui, id: egui::Id) -> Option<egui::text::CCursorRange> {
    egui::text_edit::TextEditState::load(ui.ctx(), id).and_then(|state| state.cursor.char_range())
}

fn selected_text(ui: &Ui, id: egui::Id, value: &str) -> Option<String> {
    let Some(range) = selected_range(ui, id) else {
        return Some(value.to_owned());
    };
    if range.is_empty() {
        return Some(value.to_owned());
    }
    let range = range.as_sorted_char_range();
    Some(
        value
            .chars()
            .skip(range.start.0)
            .take(range.end.0.saturating_sub(range.start.0))
            .collect(),
    )
}

fn clear_selection(ui: &Ui, id: egui::Id, value: &mut String) {
    let Some(range) = selected_range(ui, id) else {
        return;
    };
    if range.is_empty() {
        return;
    }
    let range = range.as_sorted_char_range();
    let cursor = range.start.0;
    let start = byte_index(value, range.start.0);
    let end = byte_index(value, range.end.0);
    value.replace_range(start..end, "");
    let cursor = cursor.min(value.chars().count());
    if let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), id) {
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(cursor),
            )));
        state.store(ui.ctx(), id);
    }
}

fn insert_at_cursor(ui: &Ui, id: egui::Id, value: &mut String, text: &str) {
    let range = selected_range(ui, id)
        .map(|range| range.as_sorted_char_range())
        .unwrap_or_else(|| {
            let end = value.chars().count();
            egui::text::CharIndex(end)..egui::text::CharIndex(end)
        });
    let start = byte_index(value, range.start.0);
    let end = byte_index(value, range.end.0);
    value.replace_range(start..end, text);
    let cursor = egui::text::CCursor::new(value[..start].chars().count() + text.chars().count());
    if let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), id) {
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(cursor)));
        state.store(ui.ctx(), id);
    }
}

fn byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

#[cfg(test)]
mod tests {
    use super::{byte_index, normalize_paste};

    #[test]
    fn byte_index_handles_multibyte_text() {
        let value = "主机 localhost";
        assert_eq!(byte_index(value, 0), 0);
        assert_eq!(byte_index(value, 2), "主机".len());
        assert_eq!(byte_index(value, value.chars().count()), value.len());
    }

    #[test]
    fn singleline_paste_flattens_line_endings() {
        let pasted = "first\r\nsecond\rthird\nfourth";
        let flattened = normalize_paste(pasted.to_owned(), false);
        assert_eq!(flattened, "first second third fourth");
    }

    #[test]
    fn multiline_paste_keeps_lines_and_normalizes_carriage_returns() {
        let pasted = "first\r\nsecond\rthird";
        assert_eq!(
            normalize_paste(pasted.to_owned(), true),
            "first\nsecond\nthird"
        );
    }
}
