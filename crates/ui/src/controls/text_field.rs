//! [`TextField`] and [`SearchField`]: one line of text in a sunken box, typed with egui's
//! `TextEdit` through [`behavior::Typing`](crate::behavior::Typing).

use std::collections::VecDeque;

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{
    Align2, AsIdSalt, Color32, CornerRadius, EventFilter, Id, IdSalt, Key, Modifiers, Rect, Response, Shape, Stroke, StrokeKind, Ui, pos2, vec2,
};

use super::slider::with_values;
use super::{IconButton, icon_size};
use crate::behavior::{Escape, Typing, TypingLook};
use crate::direction;
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, EditResponse, GestureHost, PolicyArgs, UndoMarks};
use crate::icons::{self, Icon};
use crate::primitives::{Content, Gap, Stack, Surface, SurfaceEdge, SurfaceKind, SurfaceStyle, Text};
use crate::story::{Level, SizeArg, Story, StoryArgs, variant_list, variant_row};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius, margin_x};
use crate::typed_story;

/// A text field's sizes and colours, from [`TextField::style`]; [`SearchField`] draws with the
/// same.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextFieldStyle {
    /// Height of the box: `field.height`.
    pub height: f32,
    /// Default width: `field.width`. A justified layout stretches it.
    pub width: f32,
    /// Between the box's edge and its text or icon: `field.pad_x`.
    pub pad_x: f32,
    /// Side of an icon: `field.icon`.
    pub icon: f32,
    /// Between an icon and the text: `field.gap`.
    pub gap: f32,
    /// Type role of the text and the placeholder: `field.role`.
    pub role: TypeRole,
    /// The box: a sunken [`Surface`] (its border, or bevel) filled with `field.fill.rest`, with
    /// `field.radius` corners and a `field.border.rest` border.
    pub surface: SurfaceStyle,
    /// `field.fill.disabled`.
    pub fill_disabled: Color32,
    /// `field.border.hover`, `field.border.focused` and `field.border.disabled`; a bevel keeps
    /// its own colours.
    pub border_hover: Color32,
    pub border_focused: Color32,
    pub border_disabled: Color32,
    /// `field.text.rest` and `field.text.disabled`.
    pub text: Color32,
    pub text_disabled: Color32,
    /// `field.placeholder.rest` and `field.placeholder.disabled`; icons in the box take them too.
    pub placeholder: Color32,
    pub placeholder_disabled: Color32,
    /// `field.selection.fill` and `field.selection.text`.
    pub selection: Color32,
    pub selected_text: Color32,
    /// `field.caret`.
    pub caret: Color32,
    /// The edge of an invalid field: `field.invalid`.
    pub invalid: Color32,
}

/// One line of text the host owns:
///
/// ```
/// use craft_ui::controls::TextField;
/// use craft_ui::edit::EditEvent;
/// # egui::__run_test_ui(|ui| { let mut name = String::from("Layer 1");
/// let edit = TextField::new("layer-name", "Layer name", &name).placeholder("Untitled").show(ui);
/// match edit.event {
///     Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => name = value,
///     // Begin and Cancel go to the document's undo history (`edit::GestureHost`).
///     _ => {}
/// }
/// if edit.submitted {
///     // Enter: a dialog's OK.
/// }
/// # });
/// ```
///
/// The field shows the host's value and reports every change back through the edit lifecycle
/// ([`crate::edit`]): a typing run is one gesture, which begins with the first change and
/// commits on Enter or when the focus leaves. The keys follow PhotoCraft's fields:
///
/// - **Enter** commits and leaves the field, and reports [`submitted`](EditResponse::submitted)
///   for a dialog's OK; [`keep_focus`](TextField::keep_focus) keeps the focus.
/// - **Esc** cancels the run (the text from before it comes back), leaves the field and passes
///   the press on: it stays in egui's input and the field reports
///   [`escaped`](EditResponse::escaped), so a dialog closes on the same press.
///
/// The other rules (input methods, in-field undo, changes from the host) are [`Typing`]'s.
///
/// - `label` is the accessible name; the field draws no label (a form row or the
///   [`Field`](crate::composites::Field) composite shows it).
/// - The placeholder shows while the field is empty, in `field.placeholder`.
/// - [`invalid`](TextField::invalid) draws the edge in `field.invalid` and reports the value
///   invalid to AccessKit; the message is the form's.
/// - The edge turns `field.border.focused` while the field has focus, from the keyboard or the
///   pointer; a focused invalid field draws the invalid colour as a line inside that edge. A
///   bevelled box (Classic) keeps its bevel and draws these lines inside it.
/// - A secondary click, or the Menu key or Shift+F10 in the focused field, opens the
///   [`EditMenu`](crate::behavior::EditMenu) (Undo, Cut, Copy, Paste, Delete, Select All); the
///   field keeps its focus. [`edit_menu(false)`](TextField::edit_menu) leaves the press to the host.
/// - AccessKit: a `TextInput` (a `PasswordInput` with [`password`](TextField::password); a
///   `MultilineTextInput` with [`multiline`](TextField::multiline)) named `label`, with its value
///   and placeholder.
#[derive(Clone, Debug)]
#[must_use = "show the field with `show`"]
pub struct TextField<'a> {
    id_salt: IdSalt,
    label: String,
    value: &'a str,
    placeholder: String,
    size: ControlSize,
    role: Option<TypeRole>,
    policy: EditPolicy,
    enabled: bool,
    invalid: bool,
    leading: Option<Icon>,
    password: bool,
    char_limit: usize,
    keep_focus: bool,
    multiline: Option<usize>,
    edit_menu: bool,
}

impl<'a> TextField<'a> {
    /// A field named `label` (its accessible name) showing the host's `value`; `id_salt` keeps its
    /// text state, focus and gesture when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a str) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            placeholder: String::new(),
            size: ControlSize::Md,
            role: None,
            policy: EditPolicy::default(),
            enabled: true,
            invalid: false,
            leading: None,
            password: false,
            char_limit: usize::MAX,
            keep_focus: false,
            multiline: None,
            edit_menu: true,
        }
    }

    /// The text shown while the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// The type role of the text and the placeholder instead of `field.role`, for a prominent
    /// field such as [`TypeRole::BodyLarge`]. The box grows to the role's line height when the
    /// size's height is less.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = Some(role);
        self
    }

    /// How the typing run reports ([`EditPolicy`]); the default reports every change at once and
    /// cancels on Esc.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the invalid edge and reports the value invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// An icon at the reading start, in the placeholder colour; decorative, so screen readers
    /// skip it.
    pub fn leading(mut self, icon: Icon) -> Self {
        self.leading = Some(icon);
        self
    }

    /// Shows a bullet per character.
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// The most characters typing can make.
    pub fn char_limit(mut self, limit: usize) -> Self {
        self.char_limit = limit;
        self
    }

    /// Whether the field keeps the focus after Enter commits; off by default.
    pub fn keep_focus(mut self, keep: bool) -> Self {
        self.keep_focus = keep;
        self
    }

    /// A field `rows` lines tall that wraps its text, for a note or a description: the first line
    /// is `field.height` tall and every further row adds a line of text. Enter inserts a line break
    /// and Cmd/Ctrl+Enter submits ([`submitted`](EditResponse::submitted)), commits and leaves the
    /// field as Enter does in a line; Tab leaves it. Esc and the typing run are the line's.
    /// AccessKit: a `MultilineTextInput`.
    pub fn multiline(mut self, rows: usize) -> Self {
        self.multiline = Some(rows.max(1));
        self
    }

    /// Whether a secondary click, or the Menu key or Shift+F10 while the field has the focus,
    /// opens the [`EditMenu`](crate::behavior::EditMenu); on by default. Turn it off where the
    /// host answers the secondary press itself.
    pub fn edit_menu(mut self, on: bool) -> Self {
        self.edit_menu = on;
        self
    }

    /// Shows the field and returns this frame's edit. The response covers the box.
    pub fn show(self, ui: &mut Ui) -> EditResponse<String> {
        let spec = Spec {
            id_salt: self.id_salt,
            label: &self.label,
            value: self.value,
            placeholder: &self.placeholder,
            size: self.size,
            role: self.role,
            enabled: self.enabled,
            invalid: self.invalid,
            leading: self.leading,
            clear: None,
            policy: self.policy,
            password: self.password,
            char_limit: self.char_limit,
            keep_focus: self.keep_focus,
            keys: EventFilter::default(),
            multiline: self.multiline,
            edit_menu: self.edit_menu,
        };
        show_field(ui, &spec)
    }

    pub fn style(tokens: &UiTokens, size: ControlSize) -> TextFieldStyle {
        let f = &tokens.component.field;
        let mut surface = Surface::style(tokens, SurfaceKind::Sunken);
        surface.fill = f.fill.rest.to_egui();
        surface.radius = corner_radius(f.radius);
        if let SurfaceEdge::Border(stroke) = surface.edge {
            surface.edge = SurfaceEdge::Border(Stroke::new(stroke.width, f.border.rest.to_egui()));
        }
        TextFieldStyle {
            height: f.height.get(size),
            width: f.width.get(size),
            pad_x: f.pad_x,
            icon: f.icon.get(size),
            gap: f.gap,
            role: f.role,
            surface,
            fill_disabled: f.fill.disabled.to_egui(),
            border_hover: f.border.hover.to_egui(),
            border_focused: f.border.focused.to_egui(),
            border_disabled: f.border.disabled.to_egui(),
            text: f.text.rest.to_egui(),
            text_disabled: f.text.disabled.to_egui(),
            placeholder: f.placeholder.rest.to_egui(),
            placeholder_disabled: f.placeholder.disabled.to_egui(),
            selection: f.selection.to_egui(),
            selected_text: f.selected_text.to_egui(),
            caret: f.caret.to_egui(),
            invalid: f.invalid.to_egui(),
        }
    }
}

/// A text field for a search: a [`TextField`] with a leading search icon and a clear button at
/// the end while there is text.
///
/// ```
/// use craft_ui::controls::SearchField;
/// # egui::__run_test_ui(|ui| { let mut query = String::new();
/// let edit = SearchField::new("layer-search", "Search layers", &query, "Clear").placeholder("Search").show(ui);
/// if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
///     query = value.clone();
/// }
/// # });
/// ```
///
/// A search filters as the user types: every change goes out at once and Esc doesn't restore
/// the text from before (the policy is not [cancellable](EditPolicy::cancellable)). Esc in a
/// field with text clears it and commits the empty text, keeping the focus and the press; Esc in
/// an empty field leaves it and goes on to the host ([`escaped`](EditResponse::escaped)), as a
/// text field's. The clear button does the same as Esc and gives the field the focus; it is for
/// the pointer only, not a Tab stop. Enter reports [`submitted`](EditResponse::submitted) ("find
/// next") and leaves the field; [`keep_focus`](SearchField::keep_focus) keeps it there.
///
/// AccessKit: a `SearchInput` named `label`; the clear button is a button named `clear_label`,
/// which is also its tooltip. The search icon is decorative.
#[derive(Clone, Debug)]
#[must_use = "show the field with `show`"]
pub struct SearchField<'a> {
    id_salt: IdSalt,
    label: String,
    value: &'a str,
    clear_label: String,
    placeholder: String,
    size: ControlSize,
    policy: EditPolicy,
    enabled: bool,
    keep_focus: bool,
    keys: EventFilter,
    edit_menu: bool,
}

impl<'a> SearchField<'a> {
    /// A search field named `label` showing the host's `value`; `clear_label` names the clear
    /// button ("Clear", translated by the app).
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a str, clear_label: impl Into<String>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            clear_label: clear_label.into(),
            placeholder: String::new(),
            size: ControlSize::Md,
            policy: EditPolicy::default().cancellable(false),
            enabled: true,
            keep_focus: false,
            keys: EventFilter::default(),
            edit_menu: true,
        }
    }

    /// The text shown while the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// How the typing run reports; by default every change at once, not cancellable.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether the field keeps the focus after Enter; off by default.
    pub fn keep_focus(mut self, keep: bool) -> Self {
        self.keep_focus = keep;
        self
    }

    /// Whether a secondary click, or the Menu key or Shift+F10 while the field has the focus,
    /// opens the [`EditMenu`](crate::behavior::EditMenu); on by default.
    pub fn edit_menu(mut self, on: bool) -> Self {
        self.edit_menu = on;
        self
    }

    /// Keys an embedding control reads while the field has the focus: a popup list's arrows
    /// and Esc ([`Typing::event_filter`]).
    pub(crate) fn keys(mut self, keys: EventFilter) -> Self {
        self.keys = keys;
        self
    }

    /// Shows the field and returns this frame's edit. The response covers the box.
    pub fn show(self, ui: &mut Ui) -> EditResponse<String> {
        let spec = Spec {
            id_salt: self.id_salt,
            label: &self.label,
            value: self.value,
            placeholder: &self.placeholder,
            size: self.size,
            role: None,
            enabled: self.enabled,
            invalid: false,
            leading: Some(Icon::Search),
            clear: Some(&self.clear_label),
            policy: self.policy,
            password: false,
            char_limit: usize::MAX,
            keep_focus: self.keep_focus,
            keys: self.keys,
            multiline: None,
            edit_menu: self.edit_menu,
        };
        show_field(ui, &spec)
    }
}

/// What [`show_field`] draws.
struct Spec<'a> {
    id_salt: IdSalt,
    label: &'a str,
    value: &'a str,
    placeholder: &'a str,
    size: ControlSize,
    role: Option<TypeRole>,
    enabled: bool,
    invalid: bool,
    leading: Option<Icon>,
    /// The clear button's label, for a search field: Esc clears, and AccessKit reports a
    /// `SearchInput`.
    clear: Option<&'a str>,
    policy: EditPolicy,
    password: bool,
    char_limit: usize,
    keep_focus: bool,
    /// The embedding control's keys.
    keys: EventFilter,
    multiline: Option<usize>,
    edit_menu: bool,
}

/// Where the parts of a field go: the room kept at the reading start and end of the text, and
/// the clear button's side.
#[derive(Clone, Copy, Debug)]
struct Layout {
    start: f32,
    end: f32,
    clear: f32,
}

impl Layout {
    fn new(style: &TextFieldStyle, spec: &Spec<'_>, clear_side: f32) -> Self {
        let start = style.pad_x + spec.leading.map_or(0.0, |_| style.icon + style.gap);
        let inset = clear_inset(style.height, clear_side);
        let end = spec.clear.map_or(style.pad_x, |_| inset + clear_side + style.gap);
        Self { start, end, clear: clear_side }
    }

    /// The leading icon's rect in `rect`, at the reading start.
    fn leading(&self, ui: &Ui, style: &TextFieldStyle, rect: Rect) -> Rect {
        let x = direction::start_x(ui, rect) + direction::forward(ui) * (style.pad_x + style.icon / 2.0);
        Rect::from_center_size(pos2(x, rect.center().y), vec2(style.icon, style.icon))
    }

    /// The clear button's rect in `rect`, at the reading end.
    fn clear(&self, ui: &Ui, rect: Rect) -> Rect {
        let half = self.clear / 2.0;
        let x = direction::end_x(ui, rect) - direction::forward(ui) * (clear_inset(rect.height(), self.clear) + half);
        Rect::from_center_size(pos2(x, rect.center().y), vec2(self.clear, self.clear))
    }
}

/// The clear button's distance from the box's end: the same as from its top and bottom.
fn clear_inset(height: f32, side: f32) -> f32 {
    ((height - side) / 2.0).max(0.0)
}

const CLEAR_SIZE: ControlSize = ControlSize::Sm;

fn show_field(ui: &mut Ui, spec: &Spec<'_>) -> EditResponse<String> {
    let tokens = Tokens::of(ui);
    let mut style = TextField::style(&tokens, spec.size);
    if let Some(role) = spec.role {
        style.role = role;
        style.height = style.height.max(tokens.typography.get(role).line_height);
    }
    let layout = Layout::new(&style, spec, IconButton::style(&tokens, CLEAR_SIZE).side);
    let enabled = spec.enabled && ui.is_enabled();
    let width = if ui.layout().horizontal_justify() { ui.available_width() } else { style.width };
    let (left, right) = if direction::is_rtl(ui) { (layout.end, layout.start) } else { (layout.start, layout.end) };
    let font = tokens.font(style.role);
    // Further rows add a line of text; the box's own padding stays the first line's.
    let line = ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let extra = spec.multiline.map_or(0.0, |rows| (rows - 1) as f32 * line);
    let look = TypingLook {
        size: vec2(width, style.height + extra),
        margin: match spec.multiline {
            Some(_) => margin_x(left, right) + egui::Margin::symmetric(0, ((style.height - line) / 2.0).max(0.0).round() as i8),
            None => margin_x(left, right),
        },
        align: direction::align2(ui, if spec.multiline.is_some() { Align2::LEFT_TOP } else { Align2::LEFT_CENTER }),
        font,
        text: if enabled { style.text } else { style.text_disabled },
        placeholder: if enabled { style.placeholder } else { style.placeholder_disabled },
        selection: style.selection,
        selected_text: style.selected_text,
        caret: style.caret,
    };
    let id = ui.make_persistent_id(spec.id_salt);
    let (escape, role) = match (spec.clear.is_some(), spec.multiline.is_some()) {
        (true, _) => (Escape::Clear, Role::SearchInput),
        (false, true) => (Escape::PassThrough, Role::MultilineTextInput),
        (false, false) => (Escape::PassThrough, Role::TextInput),
    };
    let typing = Typing::new(id, spec.label, spec.value)
        .placeholder(spec.placeholder)
        .policy(spec.policy)
        .enabled(spec.enabled)
        .invalid(spec.invalid)
        .password(spec.password)
        .char_limit(spec.char_limit)
        .keep_focus(spec.keep_focus)
        .escape(escape)
        .role(role)
        .event_filter(spec.keys)
        .edit_menu(spec.edit_menu);
    let typing = match spec.multiline {
        Some(rows) => typing.multiline(rows),
        None => typing,
    };
    // The box goes under the text, which `TextEdit` paints as it runs.
    let under = ui.painter().add(Shape::Noop);
    let mut typed = typing.show(ui, &look);
    let rect = typed.response().rect;
    ui.painter().set(under, Shape::Vec(field_box(ui, &style, rect, BoxState::of(typed.response(), enabled, spec.invalid))));
    if let Some(icon) = spec.leading {
        icons::paint(ui, layout.leading(ui, &style, rect).round_to_pixels(ui.pixels_per_point()), icon, icon_size(spec.size), look.placeholder);
    }
    if let Some(label) = spec.clear
        && enabled
        && !typed.text().is_empty()
    {
        let button = IconButton::new(Icon::Close, label).size(CLEAR_SIZE).focusable(false).id_salt(id.with("clear"));
        if button.show_at(ui, layout.clear(ui, rect)).clicked() {
            typed.clear(ui);
        }
    }
    typed.end(ui)
}

/// What a field-like box shows: [`TextField`]'s, `NumberField`'s, and a closed `Select`'s
/// (focused while it has the focus or its list is open).
#[derive(Clone, Copy, Debug)]
pub struct BoxState {
    pub enabled: bool,
    pub focused: bool,
    pub hovered: bool,
    pub invalid: bool,
}

impl BoxState {
    /// The state of a field whose box is `response`'s widget.
    pub fn of(response: &Response, enabled: bool, invalid: bool) -> Self {
        Self { enabled, focused: response.has_focus(), hovered: response.contains_pointer(), invalid }
    }
}

/// A field's box over `rect`: the surface in the state's colours, and inside a bevel the focused
/// or invalid edge. A focused invalid field draws the focused edge with the invalid colour as a
/// line inside it.
pub fn field_box(ui: &Ui, style: &TextFieldStyle, rect: Rect, state: BoxState) -> Vec<Shape> {
    let BoxState { enabled, focused, hovered, invalid } = state;
    let mut surface = style.surface;
    surface.fill = if enabled { surface.fill } else { style.fill_disabled };
    let (edge, inner) = match () {
        () if !enabled => (style.border_disabled, None),
        () if invalid && focused => (style.border_focused, Some(style.invalid)),
        () if invalid => (style.invalid, None),
        () if focused => (style.border_focused, None),
        () if hovered => (style.border_hover, None),
        () => (border_color(&surface), None),
    };
    let ppp = ui.pixels_per_point();
    let snapped = rect.round_to_pixels(ppp);
    let radius = surface.radius;
    let mut shapes = Vec::new();
    match surface.edge {
        SurfaceEdge::Border(stroke) => {
            surface.edge = SurfaceEdge::Border(Stroke::new(stroke.width, edge));
            shapes.extend(surface.shapes(rect, ppp));
            if let Some(inner) = inner {
                shapes.push(line_inside(snapped, radius, stroke.width, stroke.width, inner));
            }
        }
        SurfaceEdge::Bevel { width, .. } if enabled && (focused || invalid) => {
            shapes.extend(surface.shapes(rect, ppp));
            let bevel = (width * ppp).round().max(1.0) / ppp;
            shapes.push(line_inside(snapped, radius, bevel, 1.0, edge));
            if let Some(inner) = inner {
                shapes.push(line_inside(snapped, radius, bevel + 1.0, 1.0, inner));
            }
        }
        _ => shapes.extend(surface.shapes(rect, ppp)),
    }
    shapes
}

/// A `width` line `inset` inside `rect`, whose corners have `radius`.
fn line_inside(rect: Rect, radius: CornerRadius, inset: f32, width: f32, color: Color32) -> Shape {
    let radius = radius - inset.round() as u8;
    Shape::rect_stroke(rect.shrink(inset), radius, Stroke::new(width, color), StrokeKind::Inside)
}

fn border_color(surface: &SurfaceStyle) -> Color32 {
    match surface.edge {
        SurfaceEdge::Border(stroke) => stroke.color,
        _ => Color32::TRANSPARENT,
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Text field", "Search field"];

#[derive(StoryArgs, Debug, PartialEq)]
struct StatesArgs {
    size: SizeArg,
    placeholder: String,
    #[arg(name = "edit menu")]
    edit_menu: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SearchArgs {
    size: SizeArg,
    placeholder: String,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "controls/text-field/states",
            Level::Controls,
            "Text field",
            "States",
            StatesArgs { size: SizeArg::Md, placeholder: "Layer name".into(), edit_menu: true },
            show_states
        ),
        typed_story!("controls/text-field/event-log", Level::Controls, "Text field", "Event log", PolicyArgs::default(), show_event_log),
        typed_story!(
            "controls/search-field/states",
            Level::Controls,
            "Search field",
            "States",
            SearchArgs { size: SizeArg::Md, placeholder: "Search".into() },
            show_search_states
        ),
        typed_story!(
            "controls/search-field/filter",
            Level::Controls,
            "Search field",
            "Filter",
            SearchArgs { size: SizeArg::Md, placeholder: "Search".into() },
            show_filter
        ),
    ]
}

/// A host's text and its value when the gesture under way began, which a cancel puts back.
#[derive(Clone, Debug, Default)]
pub(super) struct Held {
    pub text: String,
    start: String,
}

impl Held {
    pub(super) fn new(text: &str) -> Self {
        Self { text: text.to_owned(), start: text.to_owned() }
    }

    pub(super) fn apply(&mut self, edit: EditResponse<String>) -> egui::Response {
        match edit.event {
            Some(EditEvent::Begin { .. }) => self.start.clone_from(&self.text),
            Some(EditEvent::Cancel { reason, .. }) if reason != CancelReason::External => self.text.clone_from(&self.start),
            Some(event) => {
                if let Some(value) = event.value() {
                    self.text.clone_from(value);
                }
            }
            None => {}
        }
        edit.response
    }
}

const STATE_ROWS: [(&str, &str); 6] = [
    ("Empty", ""),
    ("Filled", "Background"),
    ("Long text", "A layer name long enough to scroll inside the field while the caret moves along it"),
    ("Invalid", "Layer/1"),
    ("Password", "correct horse"),
    ("Leading icon", "Layers"),
];

/// Empty, filled, long, invalid, password and leading-icon fields, a multiline one, then two
/// disabled ones. A secondary click opens the edit menu unless the argument turns it off.
fn show_states(ui: &mut Ui, args: &mut StatesArgs) {
    let size = args.size.control();
    let placeholder = args.placeholder.clone();
    let edit_menu = args.edit_menu;
    let initial: [Held; STATE_ROWS.len() + 1] =
        std::array::from_fn(|i| Held::new(STATE_ROWS.get(i).map_or("Shot list\nGolden hour, wide\nDetail of the hands", |(_, text)| text)));
    with_values(ui, "text-field-states", initial, |ui, values| {
        variant_list(ui, "text-field-states", |form| {
            let Some((notes, values)) = values.split_last_mut() else { return };
            for (i, ((name, _), held)) in STATE_ROWS.iter().zip(values.iter_mut()).enumerate() {
                variant_row(form, name, |ui| {
                    let field = TextField::new(("text-field-state", i), *name, &held.text).placeholder(&placeholder).size(size).edit_menu(edit_menu);
                    let field = match i {
                        3 => field.invalid(true),
                        4 => field.password(true),
                        5 => field.leading(Icon::Layers),
                        _ => field,
                    };
                    held.apply(field.show(ui))
                });
            }
            variant_row(form, "Multiline", |ui| {
                let field = TextField::new("text-field-multiline", "Notes", &notes.text).multiline(3).size(size).edit_menu(edit_menu);
                notes.apply(field.show(ui))
            });
            for (i, text) in ["", "Locked layer"].into_iter().enumerate() {
                let name = if text.is_empty() { "Disabled, empty" } else { "Disabled, filled" };
                variant_row(form, name, |ui| {
                    TextField::new(("text-field-disabled", i), name, text).placeholder(&placeholder).size(size).enabled(false).show(ui).response
                });
            }
        });
    });
}

/// Empty, filled and disabled search fields.
fn show_search_states(ui: &mut Ui, args: &mut SearchArgs) {
    let size = args.size.control();
    let placeholder = args.placeholder.clone();
    with_values(ui, "search-field-states", [Held::new(""), Held::new("brush")], |ui, values| {
        variant_list(ui, "search-field-states", |form| {
            for (i, (name, held)) in ["Empty", "With text"].into_iter().zip(values.iter_mut()).enumerate() {
                variant_row(form, name, |ui| {
                    held.apply(SearchField::new(("search-field-state", i), name, &held.text, "Clear").placeholder(&placeholder).size(size).show(ui))
                });
            }
            variant_row(form, "Disabled", |ui| {
                SearchField::new("search-field-disabled", "Disabled", "brush", "Clear")
                    .placeholder(&placeholder)
                    .size(size)
                    .enabled(false)
                    .show(ui)
                    .response
            });
        });
    });
}

const NAMES: [&str; 12] =
    ["Background", "Sky", "Clouds", "Mountains", "Lake", "Reflection", "Trees", "Path", "Hiker", "Backpack", "Sun flare", "Vignette"];

/// A search field filtering a list of layer names as the user types.
fn show_filter(ui: &mut Ui, args: &mut SearchArgs) {
    let size = args.size.control();
    let placeholder = args.placeholder.clone();
    with_values(ui, "search-field-filter", [Held::new("")], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            let query = &mut values[0];
            query.apply(SearchField::new("search-field-filter", "Filter layers", &query.text, "Clear").placeholder(&placeholder).size(size).show(ui));
            let needle = query.text.to_lowercase();
            let shown: Vec<_> = NAMES.into_iter().filter(|name| name.to_lowercase().contains(&needle)).collect();
            if shown.is_empty() {
                ui.add(Text::new("No layers match").content(Content::Faint));
            }
            for name in shown {
                ui.add(Text::new(name));
            }
        });
    });
}

/// Events kept in the event log.
const LOG_LEN: usize = 20;

/// The event log's document: one text and an undo history of whole snapshots.
#[derive(Clone, Debug, Default)]
struct LogDoc {
    text: String,
    undo: Vec<String>,
}

impl UndoMarks for LogDoc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(text) = self.undo.get(mark) {
            self.text.clone_from(text);
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

#[derive(Clone, Debug, Default)]
struct LogState {
    doc: LogDoc,
    host: GestureHost<LogDoc>,
    field: Option<Id>,
    /// Newest first.
    log: VecDeque<String>,
}

impl LogState {
    fn log(&mut self, line: String) {
        self.log.push_front(line);
        self.log.truncate(LOG_LEN);
    }

    /// Undo outside the field: during a gesture it cancels the gesture instead of undoing a step.
    fn undo(&mut self, ui: &Ui) {
        match self.host.active() {
            Some(id) => {
                self.host.cancel(&mut self.doc, id);
                edit::cancel_all(ui.ctx(), CancelReason::UndoDuringGesture);
                self.log(format!("undo: cancel_all({:?})", CancelReason::UndoDuringGesture));
            }
            None => {
                if let Some(text) = self.doc.undo.pop() {
                    self.doc.text = text;
                    self.log("undo: one step".to_owned());
                }
            }
        }
    }
}

/// A field under a [`GestureHost`], its events newest first and the undo-step count. Cmd/Ctrl+Z
/// in the field is the field's own undo; outside it, it undoes a step of the document.
fn show_event_log(ui: &mut Ui, args: &mut PolicyArgs) {
    let policy = edit::policy(args);
    let state_id = ui.scope_id().with("text-field-event-log");
    let mut state = ui.data(|d| d.get_temp::<LogState>(state_id)).unwrap_or_default();
    let in_field = state.field.is_some_and(|id| ui.memory(|m| m.has_focus(id)));
    if !in_field && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
        state.undo(ui);
    }
    Stack::new(Gap::Stack).show(ui, |ui| {
        let text = state.doc.text.clone();
        variant_list(ui, "text-field-event-log", |form| {
            variant_row(form, "Name", |ui| {
                let edit = TextField::new("text-field-log", "Name", &text).placeholder("Type, then Enter, Esc or Tab").policy(policy).show(ui);
                state.field = Some(edit.response.id);
                if edit.submitted {
                    state.log("field: submitted".to_owned());
                }
                if edit.escaped {
                    state.log("field: escaped (the press goes on)".to_owned());
                }
                if let Some(event) = edit.event {
                    state.log(format!("field: {}", edit::describe(&event)));
                    state.host.handle(ui.ctx(), &mut state.doc, event, |doc, value| {
                        doc.undo.push(doc.text.clone());
                        doc.text = value;
                    });
                }
                edit.response
            });
        });
        if let Some(event) = state.host.end_frame(ui.ctx(), &mut state.doc) {
            let line = format!("host: {}", edit::describe(&event));
            state.log(line);
        }
        let status = format!("{:?}, {} undo steps", state.doc.text, state.doc.undo.len());
        ui.add(Text::new(status).content(Content::Muted));
        Surface::new(SurfaceKind::Sunken).show(ui, |ui| {
            ui.set_min_width(LOG_WIDTH);
            if state.log.is_empty() {
                ui.add(Text::new("No events yet").content(Content::Faint));
            }
            for line in &state.log {
                ui.add(Text::new(line.as_str()).role(TypeRole::Mono));
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, state));
}

const LOG_WIDTH: f32 = 420.0;
