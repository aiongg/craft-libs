//! [`OptionsBar`]: the bar of the active tool's options.

use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, Id, IdSalt, Key, KeyboardShortcut, Modifiers, Rect, Response, ScrollArea, Sense, Ui, UiBuilder, Vec2, vec2};

use crate::composites::measure;
use crate::controls::{IconButton, TooltipExt as _};
use crate::direction;
use crate::icons::{self, Icon, IconSize};
use crate::primitives::{Content, Divider, FocusRing};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// An options bar's sizes and colours, from [`OptionsBar::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct OptionsBarStyle {
    /// Height of a one-row bar: `options_bar.height`, or `height_narrow` for the narrow bar.
    pub height: f32,
    /// Between two rows of a bar with more than one: `options_bar.row_gap`.
    pub row_gap: f32,
    /// Space at the bar's ends: `options_bar.pad`.
    pub pad: f32,
    /// Between items, and between a caption and its control: `options_bar.gap`.
    pub gap: f32,
    /// Length of a divider: `options_bar.divider`.
    pub divider: f32,
    /// Width of the box a divider is drawn in: the gap and the line's own point.
    pub divider_box: f32,
    /// Narrow, medium and wide field widths: `options_bar.field_sm`, `field_md`, `field_lg`.
    pub fields: [f32; 3],
    /// Width of the floating scroll bar: `options_bar.scroll_bar`.
    pub scroll_bar: f32,
    /// Side of the Home, tool icon, commit and cancel buttons: `options_bar.button`. It is also the
    /// height of a row.
    pub button: f32,
    /// Fill of the bar: `surface.chrome`.
    pub fill: Color32,
    /// The chevron beside the tool icon: `text.faint`.
    pub chevron: Color32,
}

/// What the bar's trailing buttons reported in this frame, from [`OptionsBar::commit_cancel`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommitCancel {
    /// Commit was clicked.
    pub commit: bool,
    /// Cancel was clicked.
    pub cancel: bool,
}

type Slot<'a> = Option<Box<dyn FnOnce(&mut Ui) + 'a>>;
type Row<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The slots of a bar, filled by the closure given to [`OptionsBar::show`].
#[derive(Default)]
pub struct OptionsBarSlots<'a> {
    leading: Slot<'a>,
    rows: Vec<Row<'a>>,
    trailing: Slot<'a>,
}

impl<'a> OptionsBarSlots<'a> {
    /// The home button and the tool icon. They scroll with the body, as in PhotoCraft.
    pub fn leading(&mut self, add: impl FnOnce(&mut Ui) + 'a) {
        self.leading = Some(Box::new(add));
    }

    /// The tool's options on a one-row bar: cells, dividers and groups, in reading order. The same
    /// as [`row`](Self::row) called once.
    pub fn body(&mut self, add: impl FnOnce(&mut Ui) + 'a) {
        self.row(add);
    }

    /// One row of the tool's options. Call it once per row, top to bottom; the bar is as many rows
    /// high as it was called. Each row scrolls sideways on its own when it is wider than the bar.
    pub fn row(&mut self, add: impl FnOnce(&mut Ui) + 'a) {
        self.rows.push(Box::new(add));
    }

    /// Controls pinned at the reading end, outside the scrolling body: commit and cancel while a
    /// session is active. Leave it out when there is nothing to pin, and the body takes the bar.
    pub fn trailing(&mut self, add: impl FnOnce(&mut Ui) + 'a) {
        self.trailing = Some(Box::new(add));
    }
}

/// The bar of the active tool's options, as in Photoshop and PhotoCraft: a bar in `surface.chrome`
/// with a leading slot (home, tool icon), a body that scrolls sideways when it is wider than the
/// bar, and a trailing slot pinned at the reading end.
///
/// ```
/// use craft_ui::icons::Icon;
/// use craft_ui::patterns::{CellWidth, OptionsBar};
/// # egui::__run_test_ui(|ui| {
/// let mut home = false;
/// OptionsBar::new("options").label("Tool options").show(ui, |bar| {
///     bar.leading(|ui| {
///         OptionsBar::home(ui, &mut home, "Home");
///         OptionsBar::tool_icon(ui, Icon::Brush, "Brush");
///     });
///     bar.body(|ui| {
///         OptionsBar::cell(ui, "Mode:", CellWidth::Content, |ui, _caption| {
///             ui.label("Normal");
///         });
///     });
/// });
/// # });
/// ```
///
/// - The host passes whether the user prefers the narrow bar ([`narrow`](OptionsBar::narrow)); it
///   selects the shorter height and nothing else. A theme or Size never changes the layout.
/// - The scroll area has no overflow menu. When the keyboard focus moves to a control that is out
///   of view, the body scrolls it to the middle; a press or release of the pointer in the same
///   pass is left alone, because reading a response then would undo the focus a popup just asked
///   for.
/// - Dividers are the host's: call [`divider`](OptionsBar::divider) where a group ends and only when
///   the next group will draw.
/// - **Keyboard and AccessKit:** the bar is a `Toolbar` named by [`label`](OptionsBar::label). Tab
///   visits the leading slot, the body and the trailing slot in that order. A disabled
///   [`group`](OptionsBar::group) is a `Group` whose controls are disabled and keep their
///   tooltips. Dividers and the chevron are not in the tree.
pub struct OptionsBar {
    id_salt: IdSalt,
    narrow: bool,
    label: String,
}

impl OptionsBar {
    /// A bar; `id_salt` keeps its scroll position and slot widths.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), narrow: false, label: "Tool options".into() }
    }

    /// Whether the user prefers the narrow bar (Photoshop's "Enable Narrow Options Bar").
    pub fn narrow(mut self, narrow: bool) -> Self {
        self.narrow = narrow;
        self
    }

    /// The bar's accessible name, translated by the host (default "Tool options").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn style(tokens: &UiTokens, narrow: bool) -> OptionsBarStyle {
        let o = &tokens.component.options_bar;
        OptionsBarStyle {
            height: if narrow { o.height_narrow } else { o.height },
            row_gap: o.row_gap,
            pad: o.pad,
            gap: o.gap,
            divider: o.divider,
            divider_box: o.gap + 1.0,
            fields: [o.field_sm, o.field_md, o.field_lg],
            scroll_bar: o.scroll_bar,
            button: o.button,
            fill: tokens.color.surface.chrome.to_egui(),
            chevron: Content::Faint.color(tokens).to_egui(),
        }
    }

    /// Shows the bar across the available width. `add` fills the slots; they are drawn in the
    /// order leading, body, trailing whatever the order they are given in.
    pub fn show<'a>(self, ui: &mut Ui, add: impl FnOnce(&mut OptionsBarSlots<'a>)) -> Response {
        let tokens = Tokens::of(ui);
        let style = OptionsBar::style(&tokens, self.narrow);
        let mut slots = OptionsBarSlots::default();
        add(&mut slots);
        let rows = slots.rows.len().max(1);
        let height = OptionsBar::height_of(&style, rows);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, style.fill);
        let inner = Rect::from_x_y_ranges(rect.x_range().shrink(style.pad), rect.y_range());
        let id = ui.make_persistent_id(self.id_salt);
        let builder = UiBuilder::new()
            .id_salt(id)
            .max_rect(inner)
            .layout(direction::row(ui))
            .accessibility_role(Role::Toolbar)
            .accessibility_label(self.label.as_str());
        let mut bar = ui.new_child(builder);
        let trailing_id = id.with("trailing");
        let reserved = match slots.trailing {
            Some(_) => measure::trailing_width(ui.ctx(), trailing_id).map_or(0.0, |w| w + style.gap),
            None => 0.0,
        };
        let x_range = if direction::is_rtl(ui) { inner.left() + reserved..=inner.right() } else { inner.left()..=inner.right() - reserved };
        // A row's band is the one-row bar's height around the row, so the bands of neighbouring rows overlap
        // by the padding and a single row fills the bar.
        let band = style.height;
        let pitch = style.button + style.row_gap;
        let mut leading = slots.leading;
        let mut body_rows = slots.rows.into_iter();
        for index in 0..rows {
            let top = inner.top() + index as f32 * pitch;
            let scroll = Rect::from_x_y_ranges(x_range.clone(), top..=top + band);
            let mut area = bar.new_child(UiBuilder::new().id_salt(("row", index)).max_rect(scroll).layout(direction::row(ui)));
            area.spacing_mut().scroll = egui::style::ScrollStyle { bar_width: style.scroll_bar, ..egui::style::ScrollStyle::floating() };
            let leading = leading.take();
            let row = body_rows.next();
            let scroll_id = if index == 0 { IdSalt::new("tool-options") } else { IdSalt::new(("tool-options", index)) };
            ScrollArea::horizontal().id_salt(scroll_id).auto_shrink([false, false]).show(&mut area, |ui| {
                ui.spacing_mut().item_spacing.x = style.gap;
                ui.with_layout(direction::row(ui), |ui| {
                    let content = |ui: &mut Ui| {
                        if let Some(leading) = leading {
                            leading(ui);
                        }
                        if let Some(row) = row {
                            row(ui);
                        }
                        // Font ink can extend past its advance at fractional positions; keep it in the content.
                        ui.add_space(2.0);
                    };
                    if rows > 1 {
                        ui.scope_builder(UiBuilder::new().accessibility_role(Role::Group).layout(direction::row(ui)), content);
                    } else {
                        content(ui);
                    }
                });
                reveal_focused_control(ui, FocusRing::style(&Tokens::of(ui)).outset());
            });
        }
        if let Some(trailing) = slots.trailing {
            measure::trailing_row(&mut bar, trailing_id, inner, Sense::hover(), style.gap, 0.0, trailing);
        }
        response
    }

    /// The height of a bar of `rows` rows: the one-row height plus a row and a gap for each further row.
    pub fn height_of(style: &OptionsBarStyle, rows: usize) -> f32 {
        style.height + rows.saturating_sub(1) as f32 * (style.button + style.row_gap)
    }

    /// The Home button: a toggle in the leading slot, followed by a divider. The host decides when
    /// it can toggle (disable it in an `add_enabled_ui` when it cannot).
    pub fn home(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
        let response = Self::square_button(ui, IconButton::new(Icon::Home, label).toggle(on));
        Self::divider(ui);
        response
    }

    /// The active tool's icon, its chevron and a divider. It has no click action; the host attaches
    /// a [`ContextMenu`](crate::menu::ContextMenu) to the returned response (Reset Tool, Reset All
    /// Tools), which opens by secondary click, the Menu key and Shift+F10.
    pub fn tool_icon(ui: &mut Ui, icon: Icon, label: &str) -> Response {
        let tokens = Tokens::of(ui);
        let style = OptionsBar::style(&tokens, false);
        let response = Self::square_button(ui, IconButton::new(icon, label));
        let (rect, _) = ui.allocate_exact_size(vec2(10.0, 20.0), Sense::hover());
        icons::paint_in(ui, rect, Icon::ChevronDown, icons::active(ui.ctx()), 10.0, IconSize::Sm.stroke(), style.chevron);
        Self::divider(ui);
        response
    }

    /// A divider between groups: a line `options_bar.divider` long in `line.divider`, in a box
    /// the width of a gap and a point.
    pub fn divider(ui: &mut Ui) {
        let tokens = Tokens::of(ui);
        let style = OptionsBar::style(&tokens, false);
        let (rect, _) = ui.allocate_exact_size(vec2(style.divider_box, style.divider), Sense::hover());
        Divider::paint(ui.painter(), rect.center_top(), rect.center_bottom(), &tokens);
    }

    /// A group of controls that can be disabled together: they draw disabled and keep their tooltips.
    pub fn group<R>(ui: &mut Ui, enabled: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
        ui.scope_builder(UiBuilder::new().accessibility_role(Role::Group), |ui| ui.add_enabled_ui(enabled, add).inner).inner
    }

    /// The commit and cancel buttons of a session, for the trailing slot: cancel, then commit at the
    /// reading end. `commit` and `cancel` are the translated names ("Commit transform"); the tooltips
    /// add Enter and Esc, which the host's session handles as keys.
    pub fn commit_cancel(ui: &mut Ui, commit: &str, cancel: &str) -> CommitCancel {
        let cancel_clicked = Self::square_button(ui, IconButton::new(Icon::Ban, cancel).show_tooltip(false))
            .tooltip_with(cancel, &KeyboardShortcut::new(Modifiers::NONE, Key::Escape))
            .clicked();
        let commit_clicked = Self::square_button(ui, IconButton::new(Icon::Check, commit).show_tooltip(false))
            .tooltip_with(commit, &KeyboardShortcut::new(Modifiers::NONE, Key::Enter))
            .clicked();
        CommitCancel { commit: commit_clicked, cancel: cancel_clicked }
    }

    fn square_button(ui: &mut Ui, button: IconButton<'_>) -> Response {
        let side = Tokens::of(ui).component.options_bar.button;
        let (_, square) = ui.allocate_space(Vec2::splat(side));
        button.show_at(ui, square)
    }
}

/// Scrolls the focused control into the middle of the enclosing scroll area when the focus has
/// just moved there. egui does not scroll a Tab-focused widget into view.
fn reveal_focused_control(ui: &Ui, outset: f32) {
    // Reading a response reprocesses pointer events, which would undo the focus a popup requested
    // during its opening click.
    if ui.input(|input| input.pointer.any_pressed() || input.pointer.any_released()) {
        return;
    }
    let focused = ui.memory(|memory| memory.focused());
    let key = ui.scope_id().with("scroll-focused-control");
    let changed = ui.data_mut(|data| {
        let previous = data.get_temp::<Option<Id>>(key).flatten();
        data.insert_temp(key, focused);
        previous != focused
    });
    if changed
        && let Some(response) = focused.and_then(|id| ui.ctx().read_response(id))
        && response.layer_id == ui.layer_id()
        && ui.min_rect().contains_rect(response.rect)
    {
        ui.scroll_to_rect(response.rect.expand(outset), Some(egui::Align::Center));
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["OptionsBar"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "patterns/options-bar/brush-and-transform",
        level: Level::Patterns,
        component: "OptionsBar",
        name: "Brush and transform",
        args: || {
            Args::new()
                .with_bool("narrow", false)
                .with_bool("session", true)
                .with_bool("disabled", false)
                .with_bool("two_rows", false)
                .with_number("width", 640.0, 160.0, 1200.0)
        },
        show: super::options_cell::show_story,
    }]
}
