//! The named preset row: a preset field, the name prompt and the New…, Rename…, Delete, Load… and Save… entries (`NamedPresetRow`, `NamedPresetEvent`, `NamedPresetOutput`).

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{AsIdSalt, IdSalt, Key, Label, Modifiers, Painter, Rect, Response, Ui, UiBuilder, vec2};

use super::KeyTest;
use super::files::{PresetFileEvent, PresetFiles};
use crate::composites::{Field, StatusLine};
use crate::controls::{Button, ButtonVariant, IconButton, TextField};
use crate::direction;
use crate::docking::panel_menu::PanelCommand;
use crate::edit::{CancelReason, EditEvent};
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::menu::{ContextMenu, MenuEntry, MenuItem, PressMenu};
use crate::presets::model::PresetError;
use crate::presets::picker::{PickerField, PresetPicker};
use crate::presets::{PresetRow, PresetStyle, ThumbInfo};
use crate::theme::{Tokens, TokensOf as _, role_format};

crate::words! {
    /// The words of a [`NamedPresetRow`]. `{name}` stands for a preset's name.
    pub struct NamedPresetLabels {
        new: "namedPreset.new" = "New…";
        rename: "namedPreset.rename" = "Rename…";
        delete: "namedPreset.delete" = "Delete";
        save_as_new: "namedPreset.saveAsNew" = "Save as New Preset…";
        manage: "namedPreset.manage" = "Manage presets";
        name: "namedPreset.name" = "Name";
        ok: "namedPreset.ok" = "OK";
        custom: "namedPreset.custom" = "Custom";
        built_in: "namedPreset.builtIn" = "{name} (built-in)";
        name_taken: "namedPreset.nameTaken" = "A preset with this name already exists.";
        name_invalid: "namedPreset.nameInvalid" = "This name can't be used.";
    }
}

/// How a [`NamedPresetRow`] lays out its entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NamedPresetStyle {
    /// Secondary buttons after the field: Load…, Save…, New… (Photoshop's Contour Editor,
    /// PhotoCraft). Rename… and Delete are the field's context menu.
    #[default]
    Buttons,
    /// One "Manage presets" icon button with a menu: Save as New Preset…, Rename…, Delete, then
    /// the file entries (Illustrator's Image Trace, LightCraft, SoundCraft).
    Menu,
}

/// What the user did in a [`NamedPresetRow`], in order. The app does the work and makes one undo
/// step for each (typing in the Name field is a draft, not an edit).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NamedPresetEvent<K> {
    /// A preset was picked in the field: the app applies it.
    Apply(K),
    /// New… or Save as New Preset… committed `name` (trimmed; an empty name is the suggestion).
    Create { name: String },
    /// Rename… committed `name` for `key`.
    Rename { key: K, name: String },
    /// Delete was chosen for `key`; the row acts at once (the app confirms if it wants to).
    Delete(K),
    /// A file pick finished.
    File(PresetFileEvent),
}

/// Why a typed name cannot be used.
type CheckName<'a> = Box<dyn Fn(&str) -> Option<PresetError> + 'a>;

/// What [`NamedPresetRow::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the app carries out the row's events"]
pub struct NamedPresetOutput<K> {
    /// The row.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<NamedPresetEvent<K>>,
    /// A file job's status line, for the status bar.
    pub status: Option<StatusLine>,
}

/// One row that names and manages presets of a dialog's settings: a [`PresetPicker`] field
/// showing the current preset ("Custom" when nothing matches), the name prompt, and the entries
/// New…, Rename…, Delete, Load… and Save…. The Contour Editor, Image Trace, curve presets and
/// plugin presets.
///
/// ```ignore
/// let out = NamedPresetRow::new("contour-editor", "Contour preset", &rows)
///     .selected(current.as_ref())                  // None shows "Custom"
///     .style(NamedPresetStyle::Buttons)
///     .suggest_name(format!("Contour {}", user_count + 1))
///     .check_name(|name| library.name_error(name))
///     .built_in(|key| is_built_in(key))
///     .files(&contour_files)
///     .show(ui, |painter, rect, info| paint_contour(painter, rect, info.key));
/// for event in out.events { /* Apply, Create, Rename, Delete, File */ }
/// ```
///
/// - **Naming.** New… (or Save as New Preset…) opens the name row below the field, focused, the
///   suggested name selected; Rename… opens it on the selected user preset with its name. Enter
///   and OK commit; Esc cancels the name row only (the Name field owns Enter and Esc through
///   `focus::key_owner`, so a second Esc reaches the dialog); losing focus any other way keeps
///   the draft. An empty name creates the suggestion (PhotoCraft's "Contour"); a name
///   `check_name` refuses shows the error text under the field (`status.bad.fg`, the field's
///   description, polite live region) and blocks the commit. F2 on the field starts Rename…
///   when a user preset is selected.
/// - **Built-in presets** (`built_in`): Rename… and Delete are disabled; the closed field reads
///   "{name} (built-in)" ([`NamedPresetLabels::built_in`]) only when the app sets
///   `built_in`.
/// - **Files**: the entries of [`PresetFiles`]; their picks finish inside the row and come back
///   as `File` events with a status line.
/// - **Layout.** One row with `preset.toolbar_gap` gaps: field, then while naming the Name
///   label and a `TextField`
///   [`PresetStyle::name_width`](crate::presets::style::PresetStyle::name_width) wide, then the
///   buttons or the Manage button. When the width cannot hold them, the buttons wrap to a second
///   line. Tab visits the field, the Name field and the buttons in reading order; right to left
///   mirrors the order.
/// - **AccessKit.** A group named by the label; the Name field named [`NamedPresetLabels::name`];
///   the Manage button named "Manage presets" with `has_popup`.
///
/// [`PresetPicker`]: crate::presets::picker::PresetPicker
#[must_use = "show the row with `show`"]
pub struct NamedPresetRow<'a, K> {
    id_salt: IdSalt,
    label: String,
    rows: &'a [PresetRow<K>],
    selected: Option<&'a K>,
    style: NamedPresetStyle,
    field: PickerField,
    suggest_name: Option<String>,
    check_name: Option<CheckName<'a>>,
    built_in: Option<KeyTest<'a, K>>,
    files: Option<&'a PresetFiles<'a, K>>,
    labels: Option<NamedPresetLabels>,
}

impl<'a, K> NamedPresetRow<'a, K> {
    /// A row named `label` over the presets `rows` (built-in rows first, then the user's).
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, rows: &'a [PresetRow<K>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            rows,
            selected: None,
            style: NamedPresetStyle::default(),
            field: PickerField::Name,
            suggest_name: None,
            check_name: None,
            built_in: None,
            files: None,
            labels: None,
        }
    }

    /// The preset the settings match; `None` shows "Custom".
    pub fn selected(mut self, selected: Option<&'a K>) -> Self {
        self.selected = selected;
        self
    }

    /// Buttons (default) or a Manage menu.
    pub fn style(mut self, style: NamedPresetStyle) -> Self {
        self.style = style;
        self
    }

    /// How the field shows the current preset (default its name; the Contour Editor shows a
    /// square thumbnail).
    pub fn field(mut self, field: PickerField) -> Self {
        self.field = field;
        self
    }

    /// The name New… suggests and an empty name creates ("Contour 3").
    pub fn suggest_name(mut self, name: impl Into<String>) -> Self {
        self.suggest_name = Some(name.into());
        self
    }

    /// Why a typed name cannot be used (`PresetLibrary::name_error`); `None` accepts it.
    pub fn check_name(mut self, check: impl Fn(&str) -> Option<PresetError> + 'a) -> Self {
        self.check_name = Some(Box::new(check));
        self
    }

    /// Which presets are built in: Rename… and Delete are disabled for them.
    pub fn built_in(mut self, built_in: impl Fn(&K) -> bool + 'a) -> Self {
        self.built_in = Some(Rc::new(built_in));
        self
    }

    /// The file entries.
    pub fn files(mut self, files: &'a PresetFiles<'a, K>) -> Self {
        self.files = Some(files);
        self
    }

    /// The row's words (default English).
    pub fn labels(mut self, labels: NamedPresetLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

/// The open name row: `target` is the preset being renamed, `None` for a new one.
#[derive(Clone)]
struct Naming<K> {
    target: Option<K>,
    text: String,
    fresh: bool,
}

/// What the entries of the row and its menus ask for.
#[derive(Clone)]
enum Command {
    New,
    Rename,
    Delete,
    File(String),
}

impl<K: Clone + Eq + Hash + Debug + Send + Sync + 'static> NamedPresetRow<'_, K> {
    /// Shows the row across the available width; `thumb` paints a preset's thumbnail (for a
    /// thumbnail field and its popup).
    pub fn show(self, ui: &mut Ui, thumb: impl FnMut(&Painter, Rect, ThumbInfo<'_, K>)) -> NamedPresetOutput<K> {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:395-518 (`editor`: Load…, Save…, New… :444-454, Name row :408, suggestion :462, empty name "Contour"); the Name field's width is `PresetStyle::name_width`, the buttons size to their labels
        let Self { id_salt, label, rows, selected, style, field, suggest_name, check_name, built_in, files, labels } = self;
        let tokens = Tokens::of(ui);
        let preset = PresetStyle::from_tokens(&tokens);
        let words = labels.unwrap_or_else(|| NamedPresetLabels::of(ui));
        let id = ui.make_persistent_id(id_salt);
        let naming_id = id.with("naming");
        let ctx = ui.ctx().clone();
        let enabled = ui.is_enabled();
        let mut events = Vec::new();

        let files_out = files.map(|files| files.take_events(&ctx, id_salt)).unwrap_or_default();
        events.extend(files_out.events.into_iter().map(NamedPresetEvent::File));

        let current = selected.and_then(|key| rows.iter().find(|row| row.key == *key));
        let is_built_in = |key: &K| built_in.as_ref().is_some_and(|test| test(key));
        let user_preset = current.map(|row| &row.key).filter(|key| !is_built_in(key));
        let closed_name = current.filter(|row| is_built_in(&row.key)).map(|row| craft_i18n::fmt(&words.built_in, &[("name", row.label.as_str())]));
        let mut naming: Option<Naming<K>> = ctx.data(|d| d.get_temp::<Option<Naming<K>>>(naming_id)).flatten();
        let mut commands: Vec<Command> = Vec::new();

        let layout = direction::row(ui).with_main_wrap(true);
        // The first line's height, as `Ui::horizontal_wrapped` starts: a wrapped row centred in the
        // whole available height would float away from the top.
        let first_line = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), ui.spacing().interact_size.y));
        let group = UiBuilder::new()
            .id_salt(id.with("row"))
            .max_rect(first_line)
            .layout(layout)
            .accessibility_role(Role::Group)
            .accessibility_label(label.as_str());
        let row = ui.scope_builder(group, |ui| {
            ui.spacing_mut().item_spacing = vec2(preset.toolbar_gap, preset.toolbar_gap);
            let mut picker = PresetPicker::new(("named-preset-picker", id), label.as_str(), rows)
                .selected(selected)
                .field(field)
                .closed_name(closed_name)
                .show(ui, thumb);
            if let Some(key) = picker.picked.take() {
                events.push(NamedPresetEvent::Apply(key));
            }
            let field_response = picker.response;
            if enabled && user_preset.is_some() && field_response.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F2)) {
                commands.push(Command::Rename);
            }

            if let Some(open) = naming.as_mut() {
                let suggestion = suggest_name.as_deref().unwrap_or("");
                let probe = effective_name(&open.text, open.target.is_none().then_some(suggestion));
                let unchanged = open.target.as_ref().and_then(|key| rows.iter().find(|row| row.key == *key)).is_some_and(|row| row.label == probe);
                let error = match &check_name {
                    Some(check) if !probe.is_empty() && !unchanged => check(&probe).map(|error| match error {
                        PresetError::NameTaken => words.name_taken.clone(),
                        _ => words.name_invalid.clone(),
                    }),
                    _ => None,
                };
                let ok = Button::new(words.ok.as_str()).variant(ButtonVariant::Primary).id_salt("ok");
                let field_style = Field::style(&tokens);
                let name_label = ui.fonts_mut(|f| {
                    f.layout_job(LayoutJob::single_section(words.name.clone(), role_format(&tokens, field_style.label_role, field_style.label)))
                });
                let needed = name_label.size().x + preset.name_width + ok.natural_width(ui) + 2.0 * preset.toolbar_gap;
                let room = ui.available_size_before_wrap().x;
                if room < needed && room < ui.max_rect().width() - 0.5 {
                    ui.end_row();
                }
                ui.add(Label::new(name_label).selectable(false));
                let field_id = id.with("name-field");
                if open.fresh {
                    ui.memory_mut(|m| m.request_focus(field_id));
                    let mut state = egui::text_edit::TextEditState::default();
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::select_all_str(&open.text)));
                    state.store(&ctx, field_id);
                    open.fresh = false;
                }
                let edit = Field::unlabelled()
                    .error(error.clone())
                    .show(ui, |ui, invalid| {
                        TextField::new(("named-preset-name", id), words.name.as_str(), &open.text)
                            .id(field_id)
                            .width(preset.name_width)
                            .invalid(invalid)
                            .show(ui)
                    })
                    .inner;
                let mut commit = ui.add_enabled(enabled, ok).clicked();
                if let Some(
                    EditEvent::Begin { value: Some(value), .. }
                    | EditEvent::Update { value, .. }
                    | EditEvent::Set { value, .. }
                    | EditEvent::Commit { value, .. },
                ) = &edit.event
                {
                    open.text.clone_from(value);
                }
                let cancelled = edit.escaped || matches!(edit.event, Some(EditEvent::Cancel { reason: CancelReason::Escape, .. }));
                if edit.submitted {
                    // The Name field owns Enter: a dialog around the row does not also press OK.
                    ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                    commit = true;
                }
                if cancelled {
                    // The press ends the name row only; a second Esc reaches the dialog.
                    ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
                    naming = None;
                } else if commit {
                    let probe = effective_name(&open.text, open.target.is_none().then_some(suggestion));
                    if !probe.is_empty() && error.is_none() {
                        match open.target.clone() {
                            None => events.push(NamedPresetEvent::Create { name: probe }),
                            Some(key) if !unchanged => events.push(NamedPresetEvent::Rename { key, name: probe }),
                            Some(_) => {}
                        }
                        naming = None;
                    }
                }
            }

            let file_entries = files.map(|files| files.entries(&ctx, selected)).unwrap_or_default();
            let file_command = |entry: &MenuEntry<PanelCommand>| {
                entry.item().and_then(|item| item.command().map(|command| (item.label().to_owned(), command.to_owned(), item.is_enabled())))
            };
            match style {
                NamedPresetStyle::Buttons => {
                    for (text, command, on) in file_entries.iter().filter_map(file_command) {
                        if ui.add_enabled(enabled && on, Button::new(text).id_salt(command.clone())).clicked() {
                            commands.push(Command::File(command));
                        }
                    }
                    if ui.add_enabled(enabled && naming.is_none(), Button::new(words.new.as_str()).id_salt("new")).clicked() {
                        commands.push(Command::New);
                    }
                }
                NamedPresetStyle::Menu => {
                    let manage = ui.add_enabled(enabled, IconButton::new(Icon::More, words.manage.as_str()).id_salt("manage"));
                    let mut entries: Vec<MenuEntry<Command>> = vec![
                        MenuItem::new(words.save_as_new.as_str(), Command::New).enabled(naming.is_none()).into(),
                        MenuItem::new(words.rename.as_str(), Command::Rename).enabled(user_preset.is_some()).into(),
                        MenuItem::new(words.delete.as_str(), Command::Delete).enabled(user_preset.is_some()).into(),
                    ];
                    let files_menu: Vec<_> = file_entries.iter().filter_map(file_command).collect();
                    if !files_menu.is_empty() {
                        entries.push(MenuEntry::Separator);
                        entries.extend(
                            files_menu
                                .into_iter()
                                .map(|(text, command, on)| MenuEntry::from(MenuItem::new(text, Command::File(command)).enabled(on))),
                        );
                    }
                    commands.extend(PressMenu::new(&entries).show(ui, &manage).chosen);
                }
            }
            if style == NamedPresetStyle::Buttons {
                let entries: Vec<MenuEntry<Command>> = vec![
                    MenuItem::new(words.rename.as_str(), Command::Rename).enabled(user_preset.is_some()).into(),
                    MenuItem::new(words.delete.as_str(), Command::Delete).enabled(user_preset.is_some()).into(),
                ];
                commands.extend(ContextMenu::new(&entries).show(ui, &field_response).chosen);
            }
        });

        for command in commands {
            match command {
                Command::New => {
                    let text = suggest_name.clone().unwrap_or_default();
                    naming = Some(Naming { target: None, text, fresh: true });
                }
                Command::Rename => {
                    if let Some(row) = current.filter(|row| user_preset == Some(&row.key)) {
                        naming = Some(Naming { target: Some(row.key.clone()), text: row.label.clone(), fresh: true });
                    }
                }
                Command::Delete => {
                    if let Some(key) = user_preset {
                        events.push(NamedPresetEvent::Delete(key.clone()));
                    }
                }
                Command::File(command) => {
                    if let Some(files) = files {
                        files.start(&ctx, id_salt, &command, selected);
                    }
                }
            }
        }
        if naming.as_ref().is_some_and(|n| n.fresh) {
            ctx.request_repaint();
        }
        ctx.data_mut(|d| d.insert_temp(naming_id, naming));
        NamedPresetOutput { response: row.response, events, status: files_out.status }
    }
}

/// The name a commit uses: the typed name trimmed, or the suggestion when nothing is typed.
fn effective_name(text: &str, suggestion: Option<&str>) -> String {
    let typed = text.trim();
    if typed.is_empty() { suggestion.unwrap_or("").trim().to_owned() } else { typed.to_owned() }
}

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests {
    //! Screen tests of the named preset row and of the picker with files (task PR7).

    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use egui::text_edit::TextEditState;
    use egui::{Color32, Event, Id, PointerButton, Pos2};
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    use egui_kittest::{Harness, Node};

    use super::*;
    use crate::presets::menu::PresetExport;
    use crate::presets::picker::PresetPicker;
    use crate::theme::{Base, install};
    use craft_host::{AppInfo, FileError, Files, Host, PickId, PickRequest, Picked};

    const BUILT_IN: u32 = 1;
    const USER: u32 = 2;

    struct State {
        rows: Vec<PresetRow<u32>>,
        selected: Option<u32>,
        style: NamedPresetStyle,
        taken: Option<&'static str>,
        suggestion: &'static str,
        width: f32,
        with_files: bool,
        events: Vec<NamedPresetEvent<u32>>,
        dialog_enter: usize,
        dialog_escape: usize,
        field_id: Id,
        time: f64,
    }

    fn state() -> State {
        State {
            rows: vec![PresetRow::item(BUILT_IN, "Linear", 0), PresetRow::item(USER, "Mine", 0)],
            selected: None,
            style: NamedPresetStyle::Buttons,
            taken: None,
            suggestion: "Contour 3",
            width: 600.0,
            with_files: false,
            events: Vec::new(),
            dialog_enter: 0,
            dialog_escape: 0,
            field_id: Id::NULL,
            time: 0.0,
        }
    }

    fn show(ui: &mut Ui, state: &mut State) {
        ui.set_max_width(state.width);
        state.field_id = ui.make_persistent_id(egui::IdSalt::new("named")).with("name-field");
        let files =
            PresetFiles::new("contours", "Contours", &["shc"]).export(|_| Some(PresetExport { file_name: "Contours.shc".into(), bytes: vec![1] }));
        let taken = state.taken;
        let mut row = NamedPresetRow::new("named", "Contour preset", &state.rows)
            .selected(state.selected.as_ref())
            .style(state.style)
            .suggest_name(state.suggestion)
            .check_name(move |name| (Some(name) == taken).then_some(PresetError::NameTaken))
            .built_in(|key| *key == BUILT_IN);
        if state.with_files {
            row = row.files(&files);
        }
        let out = row.show(ui, |painter, rect, _| {
            painter.rect_filled(rect, 0.0, Color32::RED);
        });
        state.events.extend(out.events);
        state.dialog_enter += usize::from(ui.input(|i| i.key_pressed(Key::Enter)));
        state.dialog_escape += usize::from(ui.input(|i| i.key_pressed(Key::Escape)));
    }

    fn harness(state: State) -> Harness<'static, State> {
        let mut harness = Harness::<State>::builder()
            .with_size([640.0, 300.0])
            .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
            .build_ui_state(show, state);
        harness.run();
        harness
    }

    fn frame(harness: &mut Harness<'_, State>, gap: f64, events: Vec<Event>) {
        harness.state_mut().time += gap;
        let time = harness.state().time;
        harness.input_mut().time = Some(time);
        harness.input_mut().events.extend(events);
        harness.run_steps(1);
    }

    fn click(harness: &mut Harness<'_, State>, pos: Pos2) {
        let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        frame(harness, 1.0, vec![Event::PointerMoved(pos), button(true), button(false)]);
        harness.run();
    }

    fn press_key(harness: &mut Harness<'_, State>, key: Key) {
        let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
        frame(harness, 0.5, vec![event(true)]);
        frame(harness, 0.016, vec![event(false)]);
        harness.run();
    }

    fn at<'h>(harness: &'h Harness<'_, State>, label: &'h str) -> Node<'h> {
        harness.get_by_label(label)
    }

    fn open_new(harness: &mut Harness<'_, State>) {
        let pos = at(harness, "New…").rect().center();
        click(harness, pos);
    }

    fn name_row_open(harness: &Harness<'_, State>) -> bool {
        harness.query_by_role_and_label(egui::accesskit::Role::TextInput, "Name").is_some()
    }

    // New… opens the Name field focused with the whole suggestion selected.
    #[test]
    fn named_row_new_opens_the_name_row_with_the_suggestion_selected() {
        let mut harness = harness(state());
        assert!(!name_row_open(&harness));
        open_new(&mut harness);
        assert!(name_row_open(&harness), "New… opens the name row");
        let id = harness.state().field_id;
        assert!(harness.ctx.memory(|m| m.has_focus(id)), "the Name field has the focus");
        let range = TextEditState::load(&harness.ctx, id).and_then(|s| s.cursor.char_range()).expect("a selection");
        assert_eq!(range, egui::text::CCursorRange::select_all_str("Contour 3"), "the whole suggestion is selected");
    }

    // Enter reports `Create`; Esc closes the name row and leaves the dialog's Esc unread.
    #[test]
    fn named_row_enter_creates_and_escape_cancels_only_the_name_row() {
        let mut harness = harness(state());
        open_new(&mut harness);
        press_key(&mut harness, Key::Escape);
        assert!(!name_row_open(&harness), "Esc closes the name row");
        assert_eq!(harness.state().dialog_escape, 0, "the press did not reach the dialog");
        assert!(harness.state().events.is_empty());

        open_new(&mut harness);
        press_key(&mut harness, Key::Enter);
        assert_eq!(harness.state().events, [NamedPresetEvent::Create { name: "Contour 3".into() }]);
        assert_eq!(harness.state().dialog_enter, 0, "Enter did not reach the dialog");
        assert!(!name_row_open(&harness), "a commit closes the name row");

        press_key(&mut harness, Key::Escape);
        assert_eq!(harness.state().dialog_escape, 1, "with no name row, Esc reaches the dialog");
    }

    // A name `check_name` refuses shows `name_taken` under the field and Enter reports nothing.
    #[test]
    fn named_row_taken_name_shows_the_error_and_blocks_create() {
        let mut state = state();
        state.taken = Some("Contour 3");
        let mut harness = harness(state);
        open_new(&mut harness);
        let error = harness.get_by_label_contains("A preset with this name already exists.");
        assert!(
            error.rect().top() >= harness.get_by_role_and_label(egui::accesskit::Role::TextInput, "Name").rect().bottom() - 1.0,
            "the text sits under the Name field"
        );
        press_key(&mut harness, Key::Enter);
        assert!(harness.state().events.is_empty(), "a refused name reports nothing");
        assert!(name_row_open(&harness), "the name row stays open");
    }

    // Enter on an empty field reports `Create` with the suggested name.
    #[test]
    fn named_row_empty_name_creates_the_suggestion() {
        let mut harness = harness(state());
        open_new(&mut harness);
        press_key(&mut harness, Key::Backspace);
        press_key(&mut harness, Key::Enter);
        assert_eq!(harness.state().events, [NamedPresetEvent::Create { name: "Contour 3".into() }]);
    }

    // With `selected(None)` the field reads "Custom" and its AccessKit name ends in "Custom".
    #[test]
    fn named_row_custom_shows_when_nothing_is_selected() {
        let mut harness = harness(state());
        let field = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Contour preset: Custom");
        assert!(field.accesskit_node().label().is_some_and(|label| label.ends_with("Custom")));

        harness.state_mut().selected = Some(BUILT_IN);
        harness.run();
        harness.get_by_label("Contour preset: Linear (built-in)");
        harness.state_mut().selected = Some(USER);
        harness.run();
        harness.get_by_label("Contour preset: Mine");
    }

    // `Menu` style: the Manage menu lists Save as New Preset…, Rename…, Delete; the last two are disabled on a built-in preset.
    #[test]
    fn named_row_menu_style_lists_save_as_new_rename_delete_and_disables_for_built_in() {
        let mut state = state();
        state.style = NamedPresetStyle::Menu;
        state.selected = Some(BUILT_IN);
        let mut harness = harness(state);
        let pos = at(&harness, "Manage presets").rect().center();
        click(&mut harness, pos);
        harness.get_by_label("Save as New Preset…");
        assert!(at(&harness, "Rename…").accesskit_node().is_disabled(), "a built-in preset cannot be renamed");
        assert!(at(&harness, "Delete").accesskit_node().is_disabled(), "a built-in preset cannot be deleted");

        press_key(&mut harness, Key::Escape);
        harness.state_mut().selected = Some(USER);
        harness.run();
        let pos = at(&harness, "Manage presets").rect().center();
        click(&mut harness, pos);
        assert!(!at(&harness, "Rename…").accesskit_node().is_disabled());
        let pos = at(&harness, "Delete").rect().center();
        click(&mut harness, pos);
        assert_eq!(harness.state().events, [NamedPresetEvent::Delete(USER)]);
    }

    // At 220 pt wide, naming, the buttons wrap below the field.
    #[test]
    fn named_row_buttons_style_wraps_to_two_lines_when_narrow() {
        let mut state = state();
        state.with_files = true;
        let mut harness = harness(state);
        let field = at(&harness, "Contour preset: Custom").rect();
        assert!(at(&harness, "New…").rect().top() < field.bottom(), "wide: the buttons share the field's line");

        harness.state_mut().width = 220.0;
        harness.run_steps(3);
        open_new(&mut harness);
        let field = at(&harness, "Contour preset: Custom").rect();
        assert!(name_row_open(&harness));
        assert!(at(&harness, "New…").rect().top() >= field.bottom(), "narrow: New… wraps below the field");
        assert!(at(&harness, "Save…").rect().top() >= field.bottom(), "narrow: Save… wraps below the field");
    }

    #[derive(Default)]
    struct Fake {
        next: u64,
        requests: Vec<(PickId, PickRequest)>,
        answers: HashMap<u64, Picked>,
        forgotten: Vec<PickId>,
        written: Vec<(PathBuf, Vec<u8>)>,
    }

    #[derive(Default)]
    struct FakeFiles(Mutex<Fake>);

    impl FakeFiles {
        fn lock(&self) -> std::sync::MutexGuard<'_, Fake> {
            self.0.lock().unwrap_or_else(|e| e.into_inner())
        }
    }

    impl Files for FakeFiles {
        fn pick(&self, request: PickRequest) -> PickId {
            let mut fake = self.lock();
            fake.next += 1;
            let id = PickId(fake.next);
            fake.requests.push((id, request));
            id
        }

        fn take_answer(&self, id: PickId) -> Option<Picked> {
            self.lock().answers.remove(&id.0)
        }

        fn forget(&self, id: PickId) {
            self.lock().forgotten.push(id);
        }

        fn pick_pending(&self) -> bool {
            false
        }

        fn read(&self, path: &std::path::Path) -> Result<Vec<u8>, FileError> {
            Err(FileError::NotFound(path.to_owned()))
        }

        fn write(&self, path: &std::path::Path, bytes: &[u8]) -> Result<(), FileError> {
            self.lock().written.push((path.to_owned(), bytes.to_vec()));
            Ok(())
        }

        fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
            None
        }
    }

    struct PickerState {
        rows: Vec<PresetRow<u32>>,
        events: Vec<PresetFileEvent>,
        time: f64,
    }

    fn show_picker(ui: &mut Ui, state: &mut PickerState) {
        let files = PresetFiles::new("contours", "Contours", &["shc"])
            .replace(true)
            .export(|_| Some(PresetExport { file_name: "Contours.shc".into(), bytes: vec![1, 2, 3] }));
        let out = PresetPicker::new("picker", "Contour", &state.rows).files(&files).show(ui, |_, _, _| {});
        state.events.extend(out.files.events);
    }

    fn picker_frame(harness: &mut Harness<'_, PickerState>, events: Vec<Event>) {
        harness.state_mut().time += 1.0;
        let time = harness.state().time;
        harness.input_mut().time = Some(time);
        harness.input_mut().events.extend(events);
        harness.run_steps(1);
    }

    fn picker_click(harness: &mut Harness<'_, PickerState>, label: &str) {
        let pos = harness.get_by_label(label).rect().center();
        let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        picker_frame(harness, vec![Event::PointerMoved(pos), button(true), button(false)]);
        harness.run();
    }

    // A fake `Files`: Load… and Replace… report `Loaded` with the bytes and `replace`; Save… writes the export; a second pick forgets the first ticket.
    #[test]
    fn picker_files_entries_load_replace_and_save_through_files() {
        let files = Arc::new(FakeFiles::default());
        let state = PickerState { rows: vec![PresetRow::item(1, "Linear", 0)], events: Vec::new(), time: 0.0 };
        let setup = files.clone();
        let mut harness = Harness::<PickerState>::builder()
            .with_size([400.0, 400.0])
            .with_context_setup(move |ctx| {
                install(ctx, &Base::suite_default());
                Host::builder(AppInfo { id: "test", name: "Test", version: "1.0" }).files(setup.clone()).install(ctx);
            })
            .build_ui_state(show_picker, state);
        harness.run();
        let answer = |harness: &mut Harness<'_, PickerState>, picked: Picked| {
            let id = files.lock().requests.last().expect("a pick was asked").0;
            files.lock().answers.insert(id.0, picked);
            picker_frame(harness, Vec::new());
            harness.run();
        };

        picker_click(&mut harness, "Contour: Custom");
        picker_click(&mut harness, "Load…");
        let request = files.lock().requests.last().unwrap().1.clone();
        assert_eq!((request.purpose.as_str(), request.filters[0].extensions.clone()), ("contours", vec!["shc".to_owned()]));
        answer(&mut harness, Picked::Contents(vec![("a.shc".into(), vec![9, 8])]));
        assert_eq!(harness.state().events, [PresetFileEvent::Loaded { file_name: "a.shc".into(), bytes: vec![9, 8], replace: false }]);

        picker_click(&mut harness, "Contour: Custom");
        picker_click(&mut harness, "Replace…");
        answer(&mut harness, Picked::Contents(vec![("b.shc".into(), vec![7])]));
        assert_eq!(harness.state().events[1], PresetFileEvent::Loaded { file_name: "b.shc".into(), bytes: vec![7], replace: true });

        picker_click(&mut harness, "Contour: Custom");
        picker_click(&mut harness, "Save…");
        let request = files.lock().requests.last().unwrap().1.clone();
        assert_eq!(request.file_name.as_deref(), Some("Contours.shc"));
        answer(&mut harness, Picked::Paths(vec![PathBuf::from("/out/Contours.shc")]));
        assert_eq!(files.lock().written, [(PathBuf::from("/out/Contours.shc"), vec![1, 2, 3])]);
        assert_eq!(harness.state().events[2], PresetFileEvent::Saved { file_name: "Contours.shc".into() });

        picker_click(&mut harness, "Contour: Custom");
        picker_click(&mut harness, "Load…");
        let first = files.lock().requests.last().unwrap().0;
        picker_click(&mut harness, "Contour: Custom");
        picker_click(&mut harness, "Replace…");
        assert_eq!(files.lock().forgotten, [first], "a second pick forgets the first ticket");
    }
}
