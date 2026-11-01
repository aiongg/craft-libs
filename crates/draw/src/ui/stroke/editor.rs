//! [`StrokeEditor`]: the stroke editor's body, the one a popover or a docked panel hosts.

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::{StrokeChoiceRow, StrokeEditorStyle, StrokeField, StrokeLabels, StrokeMixed, StrokeSections, StrokeStyle};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;

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
///   per pair ([`StrokeStyle::dash_pairs`]) of Dash and Gap number fields (0–10000, suffix `×` or
///   the document unit, `stroke_editor.field_width` wide); "Add pair" (enabled under 32 entries:
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
    #[expect(dead_code, reason = "skeleton: read by task PE-B3")]
    id_salt: IdSalt,
    #[expect(dead_code, reason = "skeleton: read by task PE-B3")]
    value: &'a StrokeStyle,
    #[expect(dead_code, reason = "skeleton: read by task PE-B3")]
    numbers: &'a LocaleNumbers,
    sections: StrokeSections,
    closed: bool,
    corners: bool,
    mixed: StrokeMixed,
    rows: &'a [StrokeChoiceRow<'a>],
    labels: Option<StrokeLabels>,
    look: Option<StrokeEditorStyle>,
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

    /// Adds the editor to `ui`, `stroke_editor.width` wide.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-B3)")]
    pub fn show(self, ui: &mut Ui) -> StrokeEditorResponse {
        // pc: crates/ui-egui/src/tools/shape_stroke_ui/editor.rs:128-248 (the popup body: presets
        // :139-148, grid :151-190 with the enable rules :152, :165, :172, :179, opacity :184-188,
        // open-path note :191-193, dashes :195-228 with Add pair :213-219 and Remove pair :220-222,
        // sample :229-233, error :234-236, valid-only reports :239-248); :262-294 `choice`,
        // `number` (accessible names). vectorcraft: crates/ui-egui/src/panels/stroke.rs:150-337
        // (icon buttons, `StrokeSections::ICONS`).
        todo!()
    }
}
