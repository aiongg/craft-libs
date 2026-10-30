//! [`PropertyRow`], [`PropertyPair`] and [`PropertyGroup`]: the rows of an inspector, a short
//! label and a control that fills the rest of the width.

use std::sync::Arc;

use craft_theme::{ControlSize, TypeRole};
use egui::text::LayoutJob;
use egui::{
    Align, AsIdSalt, Color32, Context, Galley, Id, IdSalt, InnerResponse, Label, LayerId, Layout, Rect, Response, Sense, Ui, UiBuilder, pos2, vec2,
};

use super::field::{FieldControl, name_control, one_line};
use super::measure;
use crate::controls::{IconButton, NumberField, Swatch};
use crate::direction;
use crate::edit::{EditEvent, EditResponse};
use crate::icons::Icon;
use crate::localized::LocaleNumbers;
use crate::primitives::Content;
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};

/// A property row's widths, gap and label look, from [`PropertyRow::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PropertyStyle {
    /// Width of a glyph label's column: `property.glyph_width`.
    pub glyph_width: f32,
    /// Width of a word label's column: `property.word_width`.
    pub word_width: f32,
    /// The least width a control is drawn at: `property.min_control`.
    pub min_control: f32,
    /// Between the two columns of a pair: `property.column_gap`.
    pub column_gap: f32,
    /// Width of the slot between a pair's columns: `property.link_width`.
    pub link_width: f32,
    /// Between a label and its control: `field.label_gap`.
    pub label_gap: f32,
    /// A label is centred on a band this high from the row's top: the control height of the
    /// row's size (`field.height`).
    pub label_band: f32,
    /// The label's type role: `label`.
    pub label_role: TypeRole,
    /// The label: `text.muted`.
    pub label: Color32,
}

/// How a label sits in its column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelKind {
    /// One or two characters (W, H, X, Y) in a narrow column. The labels of a [`PropertyGroup`]
    /// share one centre: the widest is flush against the control and narrower ones are centred on
    /// it. The control keeps its own accessible name ("Width" beside W).
    Glyph,
    /// A word (Opacity, Angle) at the reading start of a wider column. While visible it is the
    /// control's accessible name (`labelled_by`).
    Word,
}

/// The label of a property row.
#[derive(Clone, Debug)]
pub struct PropertyLabel {
    text: String,
    kind: LabelKind,
    redundant: bool,
    scrubs: bool,
}

impl PropertyLabel {
    /// A glyph label.
    pub fn glyph(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: LabelKind::Glyph, redundant: false, scrubs: false }
    }

    /// A word label.
    pub fn word(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: LabelKind::Word, redundant: false, scrubs: false }
    }

    /// The control says the same elsewhere (its unit, its icon, the panel's title), so the label
    /// is hidden when the control would otherwise go under its minimum width. The control keeps
    /// its own accessible name then.
    pub fn redundant(mut self) -> Self {
        self.redundant = true;
        self
    }

    /// The label scrubs a [`NumberField`]: the row hands the label's response to the control
    /// closure as [`RowLabel::scrub`].
    pub fn scrubs(mut self) -> Self {
        self.scrubs = true;
        self
    }
}

/// What the control closure of a row gets about the row's label.
#[derive(Clone, Copy, Debug)]
pub struct RowLabel<'a> {
    response: Option<&'a Response>,
    scrubs: bool,
}

impl<'a> RowLabel<'a> {
    pub(super) fn new(response: Option<&'a Response>, scrubs: bool) -> Self {
        Self { response, scrubs }
    }

    /// The visible label's response; `None` while the label is hidden.
    pub fn response(&self) -> Option<&'a Response> {
        self.response
    }

    /// The label's response when the row [`scrubs`](PropertyLabel::scrubs) and the label is
    /// visible, for [`NumberField::scrub_by`]:
    ///
    /// ```
    /// # use craft_ui::composites::PropertyRow;
    /// # use craft_ui::controls::NumberField;
    /// # use craft_ui::localized::LocaleNumbers;
    /// # egui::__run_test_ui(|ui| { let numbers = LocaleNumbers::from_tag("en");
    /// PropertyRow::word("Angle").scrubs().show(ui, |ui, label| {
    ///     let field = NumberField::new("angle", "Angle", 0.0, &numbers);
    ///     match label.scrub() {
    ///         Some(label) => field.scrub_by(label),
    ///         None => field,
    ///     }
    ///     .show(ui)
    /// });
    /// # });
    /// ```
    pub fn scrub(&self) -> Option<&'a Response> {
        self.response.filter(|_| self.scrubs)
    }
}

/// A row of an inspector: a short label and a control that fills the rest of the width.
///
/// ```
/// use craft_ui::composites::{PropertyGroup, PropertyRow};
/// use craft_ui::controls::NumberField;
/// use craft_ui::localized::LocaleNumbers;
/// # egui::__run_test_ui(|ui| { let numbers = LocaleNumbers::from_tag("en");
/// PropertyGroup::new("transform").show(ui, |ui| {
///     PropertyRow::glyph("X").show(ui, |ui, _| NumberField::new("x", "X", 10.0, &numbers).show(ui));
///     PropertyRow::glyph("Y").show(ui, |ui, _| NumberField::new("y", "Y", 20.0, &numbers).show(ui));
/// });
/// # });
/// ```
///
/// - **Labels** are glyph ([`PropertyLabel::glyph`]: W, H, X, Y) or word ([`PropertyLabel::word`]:
///   Opacity, Angle). A glyph column is `property.glyph_width` wide, or as wide as the widest
///   glyph label of the [`PropertyGroup`] when that is wider; a word column is
///   `property.word_width`. The label is centred on the first `field.height` of the row and
///   `field.label_gap` from the control. The text is the app's string as given.
/// - **The control** fills the rest of the row, never under `property.min_control`: the closure
///   gets a `Ui` whose width is the control's, so a `NumberField`, `TextField` or `Select` fills
///   it.
/// - **Narrow rows.** An essential label gives up width first: it is cut with an ellipsis and
///   shows its full text as a tooltip, so the control keeps its minimum. A label marked
///   [`redundant`](PropertyRow::redundant) hides when it would not fit with the control at its
///   minimum; the control then keeps its own accessible name.
/// - **Scrubbing.** [`scrubs`](PropertyRow::scrubs) hands the label's response to the closure
///   ([`RowLabel::scrub`]) for `NumberField::scrub_by`.
/// - **Mixed values.** The row shows nothing for them: the host passes `mixed` to the control
///   (`NumberField::mixed`, `Swatch::mixed`, `CheckState::Mixed`).
/// - **AccessKit:** a word label names its control while it is visible: the control is
///   `labelled_by` it, as in PhotoCraft's word rows. A glyph label (W, X) is too short to name
///   anything, so the control keeps its own name ("Width"). A hidden label changes nothing.
///
/// Rows are separated by the `Ui`'s item spacing. [`PropertyRow::pair`] puts two columns in a
/// row.
#[derive(Clone, Debug)]
#[must_use = "show the row with `show`"]
pub struct PropertyRow {
    label: PropertyLabel,
    size: ControlSize,
}

impl PropertyRow {
    /// A row with `label`.
    pub fn new(label: PropertyLabel) -> Self {
        Self { label, size: ControlSize::Md }
    }

    /// A row with a glyph label.
    pub fn glyph(text: impl Into<String>) -> Self {
        Self::new(PropertyLabel::glyph(text))
    }

    /// A row with a word label.
    pub fn word(text: impl Into<String>) -> Self {
        Self::new(PropertyLabel::word(text))
    }

    /// Two columns, each with its label and control, of equal width.
    pub fn pair(first: PropertyLabel, second: PropertyLabel) -> PropertyPair {
        PropertyPair { first, second, size: ControlSize::Md }
    }

    /// See [`PropertyLabel::redundant`].
    pub fn redundant(mut self) -> Self {
        self.label = self.label.redundant();
        self
    }

    /// See [`PropertyLabel::scrubs`].
    pub fn scrubs(mut self) -> Self {
        self.label = self.label.scrubs();
        self
    }

    /// The size of the row's controls, which the label is centred on (`Md` by default).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// The style of a row of `size` in `tokens`.
    pub fn style(tokens: &UiTokens, size: ControlSize) -> PropertyStyle {
        let (p, f) = (&tokens.component.property, &tokens.component.field);
        PropertyStyle {
            glyph_width: p.glyph_width,
            word_width: p.word_width,
            min_control: p.min_control,
            column_gap: p.column_gap,
            link_width: p.link_width,
            label_gap: f.label_gap,
            label_band: f.height.get(size),
            label_role: TypeRole::Label,
            label: Content::Muted.color(tokens).to_egui(),
        }
    }

    /// Shows the row. The response covers the row; `add_control` draws the control, whose
    /// response must carry its AccessKit node ([`FieldControl`]).
    pub fn show<C: FieldControl>(self, ui: &mut Ui, add_control: impl FnOnce(&mut Ui, &RowLabel) -> C) -> InnerResponse<C> {
        let setup = Setup::new(ui, self.size);
        let rect = ui.available_rect_before_wrap();
        let labels = [Measured::new(ui, &setup, self.label)];
        let widest = widest(ui, &labels);
        let (control, bottom) = setup.cell(ui, rect, &labels[0], widest, add_control);
        let row = Rect::from_min_max(rect.min, pos2(rect.right(), bottom));
        InnerResponse::new(control, ui.allocate_rect(row, Sense::hover()))
    }
}

/// Two property columns in one row: from [`PropertyRow::pair`].
#[derive(Clone, Debug)]
#[must_use = "show the pair with `show`"]
pub struct PropertyPair {
    first: PropertyLabel,
    second: PropertyLabel,
    size: ControlSize,
}

impl PropertyPair {
    /// See [`PropertyRow::size`].
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Shows the two columns, `property.column_gap` apart. Each has the width the other has, less
    /// what its label takes (equal for labels of one kind). The response covers the row.
    pub fn show<A: FieldControl, B: FieldControl>(
        self,
        ui: &mut Ui,
        first: impl FnOnce(&mut Ui, &RowLabel) -> A,
        second: impl FnOnce(&mut Ui, &RowLabel) -> B,
    ) -> InnerResponse<(A, B)> {
        let pair = Pair::new(ui, self);
        let gap = pair.setup.style.column_gap;
        let width = pair.column_width(gap);
        let (a, a_bottom) = pair.setup.cell(ui, pair.at(0.0, width), &pair.labels[0], pair.widest, first);
        let (b, b_bottom) = pair.setup.cell(ui, pair.at(width + gap, width), &pair.labels[1], pair.widest, second);
        InnerResponse::new((a, b), pair.finish(ui, a_bottom.max(b_bottom)))
    }

    /// As [`show`](PropertyPair::show) with a slot of `property.link_width` between the columns,
    /// `property.column_gap` from each, for a toggle that links the two values (W and H). `link`
    /// draws in it, centred on the first `field.height` of the row.
    pub fn show_linked<A: FieldControl, L, B: FieldControl>(
        self,
        ui: &mut Ui,
        first: impl FnOnce(&mut Ui, &RowLabel) -> A,
        link: impl FnOnce(&mut Ui) -> L,
        second: impl FnOnce(&mut Ui, &RowLabel) -> B,
    ) -> InnerResponse<(A, L, B)> {
        let pair = Pair::new(ui, self);
        let style = &pair.setup.style;
        let middle = style.link_width + 2.0 * style.column_gap;
        let width = pair.column_width(middle);
        let (a, a_bottom) = pair.setup.cell(ui, pair.at(0.0, width), &pair.labels[0], pair.widest, first);
        let slot = pair.at(width + style.column_gap, style.link_width);
        let band = Rect::from_min_size(slot.min, vec2(slot.width(), style.label_band));
        let l = ui.scope_builder(UiBuilder::new().max_rect(band).layout(Layout::left_to_right(Align::Center)), link).inner;
        let (b, b_bottom) = pair.setup.cell(ui, pair.at(width + middle, width), &pair.labels[1], pair.widest, second);
        InnerResponse::new((a, l, b), pair.finish(ui, a_bottom.max(b_bottom)))
    }
}

/// A pair row being laid out: its labels measured, its rect taken.
struct Pair {
    setup: Setup,
    rect: Rect,
    labels: [Measured; 2],
    widest: f32,
}

impl Pair {
    fn new(ui: &Ui, pair: PropertyPair) -> Self {
        let setup = Setup::new(ui, pair.size);
        let rect = ui.available_rect_before_wrap();
        let labels = [Measured::new(ui, &setup, pair.first), Measured::new(ui, &setup, pair.second)];
        let widest = widest(ui, &labels);
        Self { setup, rect, labels, widest }
    }

    /// The width of each column when `middle` separates them.
    fn column_width(&self, middle: f32) -> f32 {
        ((self.rect.width() - middle) / 2.0).max(0.0)
    }

    /// The part of the row `width` wide from `offset` after its reading start.
    fn at(&self, offset: f32, width: f32) -> Rect {
        let rect = self.rect;
        let (x0, x1) = if self.setup.rtl {
            (rect.right() - offset - width, rect.right() - offset)
        } else {
            (rect.left() + offset, rect.left() + offset + width)
        };
        Rect::from_min_max(pos2(x0, rect.top()), pos2(x1, rect.bottom()))
    }

    /// Allocates the row down to the cells' `bottom`.
    fn finish(&self, ui: &mut Ui, bottom: f32) -> Response {
        let bottom = bottom.max(self.rect.top() + self.setup.style.label_band);
        ui.allocate_rect(Rect::from_min_max(self.rect.min, pos2(self.rect.right(), bottom)), Sense::hover())
    }
}

/// A label measured on its own line.
struct Measured {
    label: PropertyLabel,
    galley: Arc<Galley>,
}

impl Measured {
    fn new(ui: &Ui, setup: &Setup, label: PropertyLabel) -> Self {
        let galley = one_line(ui, label.text.clone(), setup.format.clone());
        Self { label, galley }
    }
}

/// The widest glyph label of the rows being laid out together: the group's when in one,
/// otherwise the rows' own. Every glyph label is reported to the group.
fn widest(ui: &Ui, labels: &[Measured]) -> f32 {
    let mut widest = 0.0f32;
    for m in labels.iter().filter(|m| m.label.kind == LabelKind::Glyph) {
        let own = m.galley.size().x;
        widest = widest.max(own).max(measure::column(ui, GLYPH_TAG, own).0);
    }
    widest
}

const GLYPH_TAG: &str = "PropertyGroup";

/// What every cell of a row shares.
struct Setup {
    style: PropertyStyle,
    format: egui::text::TextFormat,
    rtl: bool,
}

impl Setup {
    fn new(ui: &Ui, size: ControlSize) -> Self {
        let tokens = Tokens::of(ui);
        let mut style = PropertyRow::style(&tokens, size);
        if let Some(over) = overrides(ui) {
            style.glyph_width = over.glyph.unwrap_or(style.glyph_width);
            style.word_width = over.word.unwrap_or(style.word_width);
        }
        let format = role_format(&tokens, style.label_role, style.label);
        Self { style, format, rtl: direction::is_rtl(ui) }
    }

    /// Draws one label and control in `rect` (as wide as the cell, from its reading start) and
    /// returns the control and the cell's bottom.
    fn cell<C: FieldControl>(
        &self,
        ui: &mut Ui,
        rect: Rect,
        measured: &Measured,
        widest: f32,
        add_control: impl FnOnce(&mut Ui, &RowLabel) -> C,
    ) -> (C, f32) {
        let style = &self.style;
        let (label, galley) = (&measured.label, &measured.galley);
        let own = galley.size().x;
        let natural = match label.kind {
            LabelKind::Glyph => style.glyph_width.max(widest),
            LabelKind::Word => style.word_width,
        };
        let room = rect.width() - style.label_gap - style.min_control;
        let column = if natural <= room {
            natural
        } else if label.redundant {
            0.0
        } else {
            room.max(0.0)
        };
        let shown = column >= 1.0;
        let used = if shown { column + style.label_gap } else { 0.0 };

        // Drawn before the control, which scrubbing needs, and so listed first in AccessKit.
        let mut bottom = rect.top() + style.label_band;
        let label_response = shown.then(|| {
            let off = match label.kind {
                LabelKind::Glyph => (column - widest / 2.0 - own / 2.0).max(0.0),
                LabelKind::Word => 0.0,
            };
            let width = own.min(column - off);
            // Slack so that rounding doesn't cut a label that fits.
            let band = if width >= own { width + 1.0 } else { width };
            let (x0, x1) = if self.rtl { (rect.right() - off - band, rect.right() - off) } else { (rect.left() + off, rect.left() + off + band) };
            let band = Rect::from_min_max(pos2(x0, rect.top()), pos2(x1, rect.top() + style.label_band));
            let job = LayoutJob::single_section(label.text.clone(), self.format.clone());
            ui.scope_builder(UiBuilder::new().max_rect(band).layout(direction::row(ui)), |ui| ui.add(Label::new(job).truncate().selectable(false)))
                .inner
        });

        let width = (rect.width() - used).max(style.min_control);
        let (x0, x1) = if self.rtl { (rect.right() - used - width, rect.right() - used) } else { (rect.left() + used, rect.left() + used + width) };
        let control_rect = Rect::from_min_max(pos2(x0, rect.top()), pos2(x1, rect.bottom()));
        let mut child =
            ui.new_child(UiBuilder::new().max_rect(control_rect).layout(Layout::top_down(direction::start_align(ui)).with_cross_justify(true)));
        let control = add_control(&mut child, &RowLabel::new(label_response.as_ref(), label.scrubs));
        bottom = bottom.max(child.min_rect().bottom());

        if let Some(label) = label_response.as_ref().filter(|_| label.kind == LabelKind::Word) {
            name_control(ui.ctx(), control.control(), label);
        }
        (control, bottom)
    }
}

/// The inspector section's overrides of the label columns.
#[derive(Clone, Copy, Debug)]
struct Overrides {
    layer: LayerId,
    glyph: Option<f32>,
    word: Option<f32>,
}

fn overrides_id(ctx: &Context) -> Id {
    Id::unique("craft-ui-property-overrides").with(ctx.viewport_id())
}

fn overrides(ui: &Ui) -> Option<Overrides> {
    let layer = ui.layer_id();
    ui.data(|d| d.get_temp::<Vec<Overrides>>(overrides_id(ui.ctx())).and_then(|v| v.last().copied().filter(|o| o.layer == layer)))
}

/// A section of an inspector: its glyph labels share a centre, and the host may set its label
/// widths:
///
/// ```
/// use craft_ui::composites::{PropertyGroup, PropertyRow};
/// use craft_ui::controls::NumberField;
/// use craft_ui::localized::LocaleNumbers;
/// # egui::__run_test_ui(|ui| { let numbers = LocaleNumbers::from_tag("en");
/// PropertyGroup::new("appearance").word_width(96.0).show(ui, |ui| {
///     PropertyRow::word("Opacity").show(ui, |ui, _| NumberField::new("opacity", "Opacity", 100.0, &numbers).show(ui));
/// });
/// # });
/// ```
///
/// The group measures its glyph labels as its rows draw them and uses the widest on the next
/// pass; when it changes it asks egui to discard the pass (as [`FieldGroup`](super::FieldGroup)
/// does), so the labels never show at a stale centre. Rows find their group through the `Ui`
/// tree on the group's layer. Groups don't nest: a row belongs to the innermost one. Two groups
/// under one parent need different ids (a debug build fails an assertion when they share one).
#[derive(Clone, Copy, Debug)]
pub struct PropertyGroup {
    id_salt: IdSalt,
    glyph: Option<f32>,
    word: Option<f32>,
}

impl PropertyGroup {
    /// A group; `id_salt` keeps its measured width when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), glyph: None, word: None }
    }

    /// The glyph column's width in this section, in place of `property.glyph_width`. A wider
    /// glyph label still widens it.
    pub fn glyph_width(mut self, width: f32) -> Self {
        self.glyph = Some(width);
        self
    }

    /// The word column's width in this section, in place of `property.word_width`.
    pub fn word_width(mut self, width: f32) -> Self {
        self.word = Some(width);
        self
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let ctx = ui.ctx().clone();
        let id = overrides_id(&ctx);
        let layer = ui.layer_id();
        ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Overrides>>(id).push(Overrides { layer, glyph: self.glyph, word: self.word }));
        let row_gap = Tokens::of(ui).space.gap.row;
        let inner = measure::group(ui, GLYPH_TAG, self.id_salt, |ui| {
            ui.spacing_mut().item_spacing.y = row_gap;
            add_contents(ui)
        });
        ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Overrides>>(id).pop());
        inner
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PropertyRow"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_bool("mixed", false).with_number("width", 260.0, 60.0, 400.0);
    vec![Story::new("composites/property-row/inspector", Level::Composites, "PropertyRow", "Inspector", args, show_inspector)]
}

/// The values of the inspector story.
#[derive(Clone, Copy, Debug)]
struct Values {
    w: f64,
    h: f64,
    x: f64,
    y: f64,
    opacity: f64,
    angle: f64,
    linked: bool,
}

fn apply(edit: &EditResponse<f64>, value: &mut f64) {
    if let Some(EditEvent::Update { value: v, .. } | EditEvent::Set { value: v, .. } | EditEvent::Commit { value: v, .. }) = edit.event {
        *value = v;
    }
}

/// A Transform section (W and H with a link toggle, X and Y; glyph labels) and an Appearance
/// section (Opacity and Angle; word labels that scrub), in a panel `width` wide. `mixed` shows
/// every value as mixed.
pub(super) fn show_inspector(ui: &mut Ui, args: &mut Args) {
    let (mixed, width) = (args.bool("mixed"), args.number("width") as f32);
    let state_id = ui.scope_id().with("property-row-inspector");
    let mut v = ui.data(|d| d.get_temp::<Values>(state_id)).unwrap_or(Values {
        w: 640.0,
        h: 480.0,
        x: 12.0,
        y: 8.0,
        opacity: 100.0,
        angle: 0.0,
        linked: true,
    });
    let numbers = LocaleNumbers::from_tag("en");
    let number = |id: &'static str, label: &'static str, value: f64| NumberField::new(id, label, value, &numbers).mixed(mixed);
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        PropertyGroup::new("property-row-transform").show(ui, |ui| {
            let (w, link, h) = PropertyRow::pair(PropertyLabel::glyph("W"), PropertyLabel::glyph("H").redundant())
                .show_linked(
                    ui,
                    |ui, _| number("w", "Width", v.w).show(ui),
                    |ui| {
                        ui.add(
                            IconButton::new(if v.linked { Icon::Link } else { Icon::Unlink }, "Link width and height")
                                .size(ControlSize::Sm)
                                .toggle(&mut v.linked),
                        )
                    },
                    |ui, _| number("h", "Height", v.h).show(ui),
                )
                .inner;
            let _ = link;
            apply(&w, &mut v.w);
            apply(&h, &mut v.h);
            let (x, y) = PropertyRow::pair(PropertyLabel::glyph("X"), PropertyLabel::glyph("Y"))
                .show(ui, |ui, _| number("x", "X", v.x).show(ui), |ui, _| number("y", "Y", v.y).show(ui))
                .inner;
            apply(&x, &mut v.x);
            apply(&y, &mut v.y);
        });
        ui.add_space(ui.spacing().item_spacing.y * 2.0);
        PropertyGroup::new("property-row-appearance").show(ui, |ui| {
            for (label, id, value) in [("Opacity", "opacity", 0usize), ("Angle", "angle", 1)] {
                let edit = PropertyRow::word(label)
                    .scrubs()
                    .show(ui, |ui, row| {
                        let field = number(id, label, if value == 0 { v.opacity } else { v.angle });
                        match row.scrub() {
                            Some(l) => field.scrub_by(l),
                            None => field,
                        }
                        .show(ui)
                    })
                    .inner;
                apply(&edit, if value == 0 { &mut v.opacity } else { &mut v.angle });
            }
            let _ = PropertyRow::word("Fill")
                .redundant()
                .show(ui, |ui, _| ui.add(Swatch::new("fill", "Fill", egui::Color32::from_rgb(200, 80, 60)).mixed(mixed)));
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, v));
}
