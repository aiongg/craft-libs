//! [`StrokeEditor`]: the stroke editor's body, the one a popover or a docked panel hosts.

use std::ops::RangeInclusive;

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::Role;
use egui::text::{LayoutJob, TextFormat};
use egui::{AsIdSalt, Id, IdSalt, Modifiers, Response, Sense, Ui, UiBuilder, vec2};

use super::{
    DashUnit, LineCap, LineJoin, LineStyle, StrokeAlign, StrokeChoiceRow, StrokeEditorStyle, StrokeField, StrokeLabels, StrokeMixed, StrokePreview,
    StrokeSections, StrokeStyle,
};
use craft_ui::composites::{Form, FormGrid};
use craft_ui::controls::mixed::ApplyRowLabels;
use craft_ui::controls::{Button, CheckState, Checkbox, NumberField, RadioOption, RadioSet, Segmented, Select};
use craft_ui::direction;
use craft_ui::edit::{CancelReason, EditEvent, EditResponse};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit, UnitKind, Units};
use craft_ui::primitives::{Content, Divider, Interactive, SurfaceKind, Text};
use craft_ui::theme::{Tokens, TokensOf as _, role_format};

/// What [`StrokeEditor::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct StrokeEditorResponse {
    /// The editor's whole area.
    pub response: Response,
    /// A valid change, the value the whole new style: a preset row, a select, a segmented
    /// button, the check, Add pair and Remove pair are `Set`s; a number field's typed value is a
    /// `Set` and its scrub `Begin`, `Update`, `Commit` (Esc cancels). A style that fails
    /// [`StrokeStyle::validate`] is never reported.
    pub event: Option<EditEvent<StrokeStyle>>,
    /// The control the event came from, so the host coalesces one field's edits into one undo
    /// step (PhotoCraft's coalesce key per gesture).
    pub field: Option<StrokeField>,
    /// A pick in a [`StrokeChoiceRow`]: the row's id and the choice's id.
    pub choice: Option<(String, String)>,
}

/// The stroke editor's body (PhotoCraft's Stroke Options popover body; VectorCraft's and
/// DesignCraft's stroke panels with [`StrokeSections::PANEL`]). It edits the host's
/// [`StrokeStyle`] and reports every valid change at once; there is no OK button (lead call 21).
///
/// ```no_run
/// use craft_draw::ui::stroke::{StrokeEditor, StrokeSections, StrokeStyle};
/// use craft_ui::localized::LocaleNumbers;
/// # fn demo(ui: &mut egui::Ui, style: &mut StrokeStyle, numbers: &LocaleNumbers) {
/// let r = StrokeEditor::new("stroke-panel", style, numbers).sections(StrokeSections::PANEL).show(ui);
/// if let Some(value) = r.event.as_ref().and_then(|e| e.value()) {
///     *style = value.clone();
/// }
/// # }
/// ```
///
/// - Presets ([`StrokeSections::PRESETS`]): Solid, Dashed (`[4, 2]`), Dotted (`[0, 2]`, round
///   caps) as radio rows `stroke_editor.preset_height` high, each with a line sample drawn at
///   opacity 1; the row matching [`StrokeStyle::line_style`] is selected, none for Custom.
///   Picking one applies [`StrokeStyle::with_line_style`].
/// - Rows, a two-column grid (label column at the reading start): Alignment (Inside, Center,
///   Outside; enabled only when [`closed`](Self::closed)); Caps (Butt / Flat, Round, Square;
///   disabled for a solid closed path); Corners (Miter, Round, Bevel; enabled only with
///   [`corners`](Self::corners)); Miter limit (1–500, suffix `×`; enabled only for Miter
///   corners); Opacity ([`StrokeSections::OPACITY`], 0–100 %, stored 0..=1). Choices are
///   `Select`s `stroke_editor.choice_width` wide, or `Segmented` icon buttons with
///   [`StrokeSections::ICONS`]. Under the rows of an open path: "Open paths use centered strokes".
/// - Dashes ([`StrokeSections::DASHES`]): a "Dashed line" check (setting it applies the Dashed
///   preset, clearing it Solid); while dashed, a hint by [`StrokeStyle::dash_unit`], then one row
///   per pair ([`StrokeStyle::dash_pairs`]) of Dash and Gap number fields (0–10000, suffix `×`, or
///   `pt` for document units, `stroke_editor.field_width` wide); "Add pair" (enabled under 32 entries:
///   appends `[4, 2]`, or `2` to complete an odd pattern) and "Remove pair" (enabled over 2
///   entries: drops the last two); "Dash offset" (±1,000,000).
/// - Sample ([`StrokeSections::SAMPLE`]): a `stroke_editor.sample_height` tall
///   [`StrokePreview`](super::StrokePreview) on `field.fill`, a rectangle when closed, a line
///   otherwise. While the style fails [`StrokeStyle::validate`], [`StrokeLabels::invalid`] in
///   `status.bad.fg` under it, and nothing is reported.
/// - [`mixed`](Self::mixed): a mixed field shows the mixed placeholder and lights no button or
///   preset; the first edit sets it for all. [`rows`](Self::rows): the app's choice rows after
///   Corners, each a `Select` with painted previews.
/// - Keyboard: Tab top to bottom through the controls; arrows inside radio and segmented sets;
///   Enter on a preset selects it. AccessKit: presets are radio buttons with their selected
///   state, numbers sliders with value and name, the sample a decorative image named by the
///   style's name. Right to left: the grid mirrors; samples draw left to right.
pub struct StrokeEditor<'a> {
    id_salt: IdSalt,
    value: &'a StrokeStyle,
    numbers: &'a LocaleNumbers,
    sections: StrokeSections,
    closed: bool,
    corners: bool,
    mixed: StrokeMixed,
    rows: &'a [StrokeChoiceRow<'a>],
    labels: Option<StrokeLabels>,
    look: Option<StrokeEditorStyle>,
    report_same_preset: bool,
    pending_key: Option<Id>,
}

impl<'a> StrokeEditor<'a> {
    /// The editor of the host's `value`, its numbers formatted with `numbers`.
    pub fn new(id_salt: impl AsIdSalt, value: &'a StrokeStyle, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            sections: StrokeSections::default(),
            closed: true,
            corners: true,
            mixed: StrokeMixed::default(),
            rows: &[],
            labels: None,
            look: None,
            report_same_preset: false,
            pending_key: None,
        }
    }

    /// Which parts show; [`StrokeSections::POPOVER`] by default.
    pub fn sections(mut self, sections: StrokeSections) -> Self {
        self.sections = sections;
        self
    }

    /// Whether the stroked path is closed: alignment applies, caps need dashes, and the tall
    /// sample is a rectangle. True by default.
    pub fn closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }

    /// Whether the path has corners for the Corners and Miter limit rows (an ellipse has none).
    /// True by default.
    pub fn corners(mut self, corners: bool) -> Self {
        self.corners = corners;
        self
    }

    /// The fields that differ across the host's selection.
    pub fn mixed(mut self, mixed: &StrokeMixed) -> Self {
        self.mixed = *mixed;
        self
    }

    /// The app's choice rows, after the Corners row.
    pub fn rows(mut self, rows: &'a [StrokeChoiceRow<'a>]) -> Self {
        self.rows = rows;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: StrokeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// A look of the host's in place of [`style`](Self::style)'s.
    pub fn look(mut self, style: StrokeEditorStyle) -> Self {
        self.look = Some(style);
        self
    }

    /// Picking the preset the style already has reports it too (the button while the stroke is
    /// off: the pick turns the stroke on).
    pub(super) fn report_same_preset(mut self, on: bool) -> Self {
        self.report_same_preset = on;
        self
    }

    /// Where the pending invalid edit is kept, so the host can drop it when it closes the editor.
    pub(super) fn pending_key(mut self, key: Id) -> Self {
        self.pending_key = Some(key);
        self
    }

    /// Adds the editor to `ui`, `stroke_editor.width` wide.
    pub fn show(self, ui: &mut Ui) -> StrokeEditorResponse {
        // pc: crates/ui-egui/src/tools/shape_stroke_ui/editor.rs:128-248 (the popup body: presets
        // :139-148, grid :151-190 with the enable rules :152, :165, :172, :179, opacity :184-188,
        // open-path note :191-193, dashes :195-228 with Add pair :213-219 and Remove pair :220-222,
        // sample :229-233, error :234-236, valid-only reports :239-248); :262-294 `choice`,
        // `number` (accessible names). vectorcraft: crates/ui-egui/src/panels/stroke.rs:150-337
        // (icon buttons, `StrokeSections::ICONS`).
        let tokens = Tokens::of(ui);
        let look = self.look.unwrap_or_else(|| Self::style(&tokens));
        let labels = self.labels.unwrap_or_else(|| StrokeLabels::of(ui));
        let mixed_word = ApplyRowLabels::of(ui).mixed;
        let select = Select::style(&tokens, ControlSize::Md);
        let width = body_width(ui, &look);
        let id = ui.make_persistent_id(self.id_salt);
        let key = self.pending_key.unwrap_or_else(|| id.with("pending"));
        let pending = ui.data(|d| d.get_temp::<Pending>(key)).filter(|p| p.base == *self.value);
        let mut body = Body {
            id,
            value: pending.map_or_else(|| self.value.clone(), |p| p.style),
            numbers: self.numbers,
            units: Units::of(ui),
            sections: self.sections,
            closed: self.closed,
            corners: self.corners,
            mixed: self.mixed,
            rows: self.rows,
            report_same_preset: self.report_same_preset,
            labels,
            mixed_word,
            look,
            error: role_format(&tokens, TypeRole::Caption, look.error),
            preview_height: select.field.icon,
            event: None,
            field: None,
            choice: None,
            invalid: None,
        };
        let builder = UiBuilder::new().id_salt(self.id_salt).layout(direction::column(ui));
        let response = ui
            .scope_builder(builder, |ui| {
                ui.set_width(width);
                body.show(ui);
            })
            .response;
        match body.invalid {
            Some(style) => ui.data_mut(|d| {
                d.insert_temp(key, Pending { base: self.value.clone(), style });
            }),
            None if body.event.is_some() => ui.data_mut(|d| {
                d.remove::<Pending>(key);
            }),
            None => {}
        }
        StrokeEditorResponse { response, event: body.event, field: body.field, choice: body.choice }
    }
}

/// The body's width: `stroke_editor.width`, less in a window narrower than that plus the margin.
pub(super) fn body_width(ui: &Ui, look: &StrokeEditorStyle) -> f32 {
    let room = ui.ctx().content_rect().width() - look.screen_margin;
    look.width.min(room).max(look.min_width)
}

/// `value`, or `None` while the field is mixed: nothing is lit.
fn unless<T>(mixed: bool, value: T) -> Option<T> {
    (!mixed).then_some(value)
}

fn wide(range: RangeInclusive<f32>) -> RangeInclusive<f64> {
    f64::from(*range.start())..=f64::from(*range.end())
}

/// The name of the style's line: a preset's, or Custom.
pub(super) fn line_style_name<'l>(labels: &'l StrokeLabels, style: &StrokeStyle) -> &'l str {
    match style.line_style() {
        LineStyle::Solid => &labels.solid,
        LineStyle::Dashed => &labels.dashed,
        LineStyle::Dotted => &labels.dotted,
        LineStyle::Custom => &labels.custom,
    }
}

/// One pass of the editor: what it draws from and what it found.
struct Body<'a> {
    id: Id,
    /// The host's value, or the pending invalid edit made on it.
    value: StrokeStyle,
    numbers: &'a LocaleNumbers,
    units: Units,
    sections: StrokeSections,
    closed: bool,
    corners: bool,
    mixed: StrokeMixed,
    rows: &'a [StrokeChoiceRow<'a>],
    report_same_preset: bool,
    labels: StrokeLabels,
    mixed_word: String,
    look: StrokeEditorStyle,
    /// The format of the error line: the caption role in `status.bad.fg`.
    error: TextFormat,
    /// The height of a choice preview in a select: the field's icon side.
    preview_height: f32,
    event: Option<EditEvent<StrokeStyle>>,
    field: Option<StrokeField>,
    choice: Option<(String, String)>,
    /// The last edit of the pass that fails [`StrokeStyle::validate`].
    invalid: Option<StrokeStyle>,
}

/// The invalid edit the user last made on `base`, kept until the host's value or a valid edit
/// replaces it (PhotoCraft's draft): the fields show it and the message names it.
#[derive(Clone)]
pub(super) struct Pending {
    base: StrokeStyle,
    style: StrokeStyle,
}

/// The event as the editor reports it: a style that fails [`StrokeStyle::validate`] is never
/// reported, so a gesture whose value turns invalid ends in a cancel.
fn checked(event: EditEvent<StrokeStyle>) -> Option<EditEvent<StrokeStyle>> {
    let valid = |style: &StrokeStyle| style.validate().is_ok();
    match event {
        EditEvent::Begin { id, value: Some(value), .. } if !valid(&value) => Some(EditEvent::begin(id)),
        EditEvent::Update { ref value, .. } | EditEvent::Set { ref value, .. } if !valid(value) => None,
        EditEvent::Commit { id, ref value, .. } if !valid(value) => Some(EditEvent::cancel(id, CancelReason::External)),
        event => Some(event),
    }
}

impl Body<'_> {
    fn show(&mut self, ui: &mut Ui) {
        if self.sections.contains(StrokeSections::PRESETS) {
            self.presets(ui);
            Divider::horizontal().show(ui);
        }
        self.grid(ui);
        if !self.closed {
            ui.add(Text::new(self.labels.open_paths.clone()).role(TypeRole::Caption).content(Content::Muted));
        }
        if self.sections.contains(StrokeSections::DASHES) {
            Divider::horizontal().show(ui);
            self.dashes(ui);
        }
        if self.sections.contains(StrokeSections::SAMPLE) {
            self.sample(ui);
        }
        if self.invalid.is_some() || self.value.validate().is_err() {
            ui.label(LayoutJob::single_section(self.labels.invalid.clone(), self.error.clone()));
        }
    }

    /// Takes the first event of the pass; a field's gesture is the only thing that reports twice.
    fn push(&mut self, field: StrokeField, event: Option<EditEvent<StrokeStyle>>) {
        if self.event.is_none()
            && let Some(event) = event
        {
            if let Some(style) = event.value().filter(|style| style.validate().is_err()) {
                self.invalid = Some(style.clone());
            }
            if let Some(event) = checked(event) {
                self.event = Some(event);
                self.field = Some(field);
            }
        }
    }

    fn set(&mut self, field: StrokeField, style: StrokeStyle) {
        self.push(field, Some(EditEvent::set(style, Modifiers::NONE)));
    }

    fn presets(&mut self, ui: &mut Ui) {
        let current = if self.mixed.dashes { LineStyle::Custom } else { self.value.line_style() };
        let mut picked = current;
        let mut radios = RadioSet::new((self.id, "presets"), self.labels.title.clone(), &mut picked)
            .item(LineStyle::Solid)
            .item(LineStyle::Dashed)
            .item(LineStyle::Dotted)
            .begin(ui);
        let look = self.look;
        let mut again = None;
        for (i, (preset, name)) in
            [(LineStyle::Solid, &self.labels.solid), (LineStyle::Dashed, &self.labels.dashed), (LineStyle::Dotted, &self.labels.dotted)]
                .into_iter()
                .enumerate()
        {
            // The rows show their pattern whatever the opacity is.
            let mut sample = self.value.with_line_style(preset);
            sample.opacity = 1.0;
            let row = RadioOption::new(name).content(|row| {
                let (_, rect) = row.allocate_space(vec2(row.available_width(), look.preset_height));
                let preview = StrokePreview::new(&sample, look.sample_stroke, look.sample_color).margin(look.sample_margin);
                row.painter().extend(preview.shapes(rect));
            });
            if radios.radio_with(ui, i, row).clicked() && preset == current {
                again = Some(preset);
            }
        }
        let edit = radios.end(ui);
        if let Some(&preset) = edit.committed() {
            self.set(StrokeField::Preset, self.value.with_line_style(preset));
        } else if let (Some(preset), true) = (again, self.report_same_preset) {
            self.set(StrokeField::Preset, self.value.with_line_style(preset));
        }
    }

    fn grid(&mut self, ui: &mut Ui) {
        let (style, mixed, labels) = (&self.value.clone(), self.mixed, self.labels.clone());
        let solid_closed = self.closed && style.dashes.is_empty() && !mixed.dashes;
        FormGrid::new((self.id, "grid")).show(ui, |form| {
            let align = [
                (StrokeAlign::Inside, Icon::StrokeAlignInside, &labels.inside),
                (StrokeAlign::Center, Icon::StrokeAlignCenter, &labels.center),
                (StrokeAlign::Outside, Icon::StrokeAlignOutside, &labels.outside),
            ];
            let cap = [
                (LineCap::Butt, Icon::StrokeCapButt, &labels.butt),
                (LineCap::Round, Icon::StrokeCapRound, &labels.round_cap),
                (LineCap::Square, Icon::StrokeCapSquare, &labels.square),
            ];
            let join = [
                (LineJoin::Miter, Icon::StrokeJoinMiter, &labels.miter),
                (LineJoin::Round, Icon::StrokeJoinRound, &labels.round_join),
                (LineJoin::Bevel, Icon::StrokeJoinBevel, &labels.bevel),
            ];
            if let Some(v) = self.choice(form, &labels.alignment, "align", unless(mixed.align, style.align), &align, self.closed) {
                self.set(StrokeField::Align, StrokeStyle { align: v, ..style.clone() });
            }
            if let Some(v) = self.choice(form, &labels.caps, "cap", unless(mixed.cap, style.cap), &cap, !solid_closed) {
                self.set(StrokeField::Cap, StrokeStyle { cap: v, ..style.clone() });
            }
            if let Some(v) = self.choice(form, &labels.corners, "join", unless(mixed.join, style.join), &join, self.corners) {
                self.set(StrokeField::Join, StrokeStyle { join: v, ..style.clone() });
            }
            self.app_rows(form);
            let miter = self.corners && (mixed.join || style.join == LineJoin::Miter);
            form.set_disabled(!miter);
            let edit = form
                .row(&labels.miter_limit, |ui, _| {
                    let unit = self.units.unit(UnitKind::Times);
                    self.number(
                        ui,
                        "miter-limit",
                        &labels.miter_limit,
                        f64::from(style.miter_limit),
                        wide(StrokeStyle::MITER_LIMIT_RANGE),
                        unit,
                        mixed.miter_limit,
                    )
                })
                .inner;
            form.set_disabled(false);
            self.number_event(StrokeField::MiterLimit, edit, |v| StrokeStyle { miter_limit: v as f32, ..style.clone() });
            if self.sections.contains(StrokeSections::OPACITY) {
                let edit = form
                    .row(&labels.opacity, |ui, _| {
                        let unit = self.units.unit(UnitKind::Percent);
                        self.number(ui, "opacity", &labels.opacity, f64::from(style.opacity) * 100.0, 0.0..=100.0, unit, mixed.opacity)
                    })
                    .inner;
                self.number_event(StrokeField::Opacity, edit, |v| StrokeStyle { opacity: v as f32 / 100.0, ..style.clone() });
            }
        });
    }

    /// A row of choices as a select, or as a segmented set with [`StrokeSections::ICONS`]; the
    /// pick, if any. `current` is `None` while the field is mixed: nothing is lit.
    fn choice<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        form: &mut Form<'_>,
        label: &str,
        salt: &'static str,
        current: Option<T>,
        items: &[(T, Icon, &String)],
        enabled: bool,
    ) -> Option<T> {
        form.set_disabled(!enabled);
        let id = (self.id, salt);
        let mut value = current;
        let mut picked = None;
        form.row(label, |ui, _| {
            if self.sections.contains(StrokeSections::ICONS) {
                let mut segmented = Segmented::new(id, label, &mut value);
                for (item, icon, name) in items {
                    segmented = segmented.icon_item(Some(*item), *icon, name.as_str());
                }
                let response = segmented.show(ui);
                if response.changed() {
                    picked = value;
                }
                response
            } else {
                let mut select = Select::new(id, label, &mut value).width(self.look.choice_width).placeholder(self.mixed_word.as_str());
                for (item, _, name) in items {
                    select = select.option(Some(*item), name.as_str());
                }
                let edit = select.show(ui);
                picked = edit.committed().copied().flatten();
                edit.response
            }
        });
        form.set_disabled(false);
        picked
    }

    /// The app's rows: each a select of the row's choices with painted previews.
    fn app_rows(&mut self, form: &mut Form<'_>) {
        let rows = self.rows;
        for row in rows {
            let mixed = row.selected.is_some_and(|m| m.is_mixed());
            let mut value: Option<String> = row.selected.and_then(|m| m.same()).map(str::to_owned);
            let mut picked = None;
            form.row(&row.label, |ui, _| {
                let mut select = Select::new((self.id, "row", &row.id), row.label.clone(), &mut value)
                    .width(self.look.choice_width)
                    .placeholder(if mixed { self.mixed_word.as_str() } else { "" });
                for choice in row.choices {
                    select = select.option(Some(choice.id.clone()), choice.label.clone());
                }
                if let Some(paint) = &row.paint {
                    let size = vec2(self.look.field_width, self.preview_height);
                    select = select.row_content(size, |painter, rect, id: &Option<String>| {
                        if let Some(choice) = row.choices.iter().find(|c| Some(&c.id) == id.as_ref()) {
                            paint(painter, rect, choice);
                        }
                    });
                }
                let edit = select.show(ui);
                picked = edit.committed().cloned().flatten();
                edit.response
            });
            if let Some(id) = picked
                && self.choice.is_none()
            {
                self.choice = Some((row.id.clone(), id));
            }
        }
    }

    /// A number field named `name`.
    #[expect(clippy::too_many_arguments, reason = "one call per field")]
    fn number(
        &self,
        ui: &mut Ui,
        salt: &'static str,
        name: &str,
        value: f64,
        range: RangeInclusive<f64>,
        unit: Unit,
        mixed: bool,
    ) -> EditResponse<f64> {
        NumberField::new((self.id, salt), name, value, self.numbers).range(range).unit(unit).width(self.look.field_width).mixed(mixed).show(ui)
    }

    /// Reports a number field's edit as the style `with` makes of its value.
    fn number_event(&mut self, field: StrokeField, edit: EditResponse<f64>, with: impl FnOnce(f64) -> StrokeStyle) {
        self.push(field, edit.event.map(|event| event.map(with)));
    }

    fn dash_unit(&self) -> Unit {
        match self.value.dash_unit {
            DashUnit::Widths => self.units.unit(UnitKind::Times),
            DashUnit::Document => self.units.unit(UnitKind::Points),
        }
    }

    fn dashes(&mut self, ui: &mut Ui) {
        let style = &self.value.clone();
        let mut state = if self.mixed.dashes {
            CheckState::Mixed
        } else if style.dashes.is_empty() {
            CheckState::Unchecked
        } else {
            CheckState::Checked
        };
        let edit = Checkbox::new(self.labels.dashed_line.clone(), &mut state).id_salt((self.id, "dashed")).show(ui);
        if let Some(&state) = edit.committed() {
            let preset = if state == CheckState::Checked { LineStyle::Dashed } else { LineStyle::Solid };
            self.set(StrokeField::Dashed, style.with_line_style(preset));
        }
        if self.mixed.dashes || style.dashes.is_empty() {
            return;
        }
        let hint = match style.dash_unit {
            DashUnit::Widths => &self.labels.dash_widths,
            DashUnit::Document => &self.labels.dash_document,
        };
        ui.add(Text::new(hint.clone()).role(TypeRole::Caption).content(Content::Muted));
        let (dash, gap) = (self.labels.dash.clone(), self.labels.gap.clone());
        for (pair, lengths) in style.dashes.chunks(2).enumerate() {
            direction::horizontal(ui, |ui| {
                for (k, &length) in lengths.iter().enumerate() {
                    let (index, name) = (pair * 2 + k, if k == 0 { &dash } else { &gap });
                    ui.add(Text::new(name.clone()).role(TypeRole::Label).content(Content::Muted));
                    let edit = NumberField::new((self.id, "dash", index), name.as_str(), f64::from(length), self.numbers)
                        .range(wide(StrokeStyle::DASH_RANGE))
                        .unit(self.dash_unit())
                        .width(self.look.field_width)
                        .show(ui);
                    let with = |v: f64| {
                        let mut next = style.clone();
                        next.dashes[index] = v as f32;
                        next
                    };
                    self.number_event(StrokeField::Dash(index), edit, with);
                }
            });
        }
        direction::horizontal(ui, |ui| {
            if ui.add(Button::new(self.labels.add_pair.clone()).enabled(style.dashes.len() < StrokeStyle::MAX_DASHES)).clicked() {
                let mut next = style.clone();
                if next.dashes.len() % 2 == 1 {
                    next.dashes.push(StrokeStyle::DASHED[1]);
                } else {
                    next.dashes.extend(StrokeStyle::DASHED);
                }
                self.set(StrokeField::AddPair, next);
            }
            if ui.add(Button::new(self.labels.remove_pair.clone()).enabled(style.dashes.len() > 2)).clicked() {
                let mut next = style.clone();
                next.dashes.truncate(style.dashes.len().saturating_sub(2));
                self.set(StrokeField::RemovePair, next);
            }
        });
        let name = self.labels.dash_offset.clone();
        let edit = FormGrid::new((self.id, "offset"))
            .show(ui, |form| {
                form.row(&name, |ui, _| {
                    let range = -f64::from(StrokeStyle::DASH_OFFSET_MAX)..=f64::from(StrokeStyle::DASH_OFFSET_MAX);
                    self.number(ui, "dash-offset", &name, f64::from(style.dash_offset), range, self.dash_unit(), self.mixed.dash_offset)
                })
                .inner
            })
            .inner;
        self.number_event(StrokeField::DashOffset, edit, |v| StrokeStyle { dash_offset: v as f32, ..style.clone() });
    }

    fn sample(&self, ui: &mut Ui) {
        let name = line_style_name(&self.labels, &self.value);
        let size = vec2(ui.available_width(), self.look.sample_height);
        let id = (self.id, "sample");
        let response = Interactive::salted(IdSalt::new(id), Role::Image, name)
            .sense(Sense::hover())
            .surface(SurfaceKind::Sunken)
            .state_layer(false)
            .focus_ring(false)
            .show(ui, size);
        if ui.is_rect_visible(response.rect) {
            let look = &self.look;
            let preview = StrokePreview::new(&self.value, look.sample_stroke, look.sample_color)
                .closed(self.closed)
                .inset(look.sample_inset)
                .margin(look.sample_margin);
            ui.painter().extend(preview.shapes(response.rect));
        }
    }
}
