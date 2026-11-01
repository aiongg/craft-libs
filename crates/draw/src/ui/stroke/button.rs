//! [`StrokeButton`]: the opener that shows the stroke's style and opens the editor in a popover.

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::{StrokeEditorStyle, StrokeField, StrokeLabels, StrokeSections, StrokeStyle};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;

/// What [`StrokeButton::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct StrokeButtonResponse {
    /// The opener.
    pub response: Response,
    /// The editor's valid change, as [`StrokeEditorResponse::event`](super::StrokeEditorResponse::event):
    /// the host makes one undo step per gesture ([`field`](Self::field) names it).
    pub event: Option<EditEvent<StrokeStyle>>,
    pub field: Option<StrokeField>,
}

/// The stroke style's opener in an options bar or the shape Properties (PhotoCraft's
/// `shape_stroke_ui::button`) and the [`StrokeEditor`](super::StrokeEditor) in a craft-ui
/// `Popover` under it.
///
/// ```no_run
/// use craft_draw::ui::stroke::{StrokeButton, StrokeStyle};
/// use craft_ui::localized::LocaleNumbers;
/// # fn demo(ui: &mut egui::Ui, style: &mut StrokeStyle, numbers: &LocaleNumbers, open: &mut bool, layer: u64) {
/// let r = StrokeButton::new("shape-stroke", style, numbers).closed(true).target(layer).show(ui, open);
/// if let Some(value) = r.event.as_ref().and_then(|e| e.value()) {
///     *style = value.clone(); // the host runs its "shape.edit" with it
/// }
/// # }
/// ```
///
/// - The opener: `stroke_editor.opener_width` by `stroke_editor.opener_height`, the style's name
///   (Solid, Dashed, Dotted, Custom; "No Color" while [`enabled`](Self::enabled) is false), a
///   line sample (a faint line crossed by a red slash while off) and a chevron. AccessKit: a
///   button named "Stroke Options" with the style's name as its value; the tooltip says "Stroke
///   Options".
/// - A click, Enter or Space toggles the host's `open`. The popover opens below the opener, focus
///   moves to its first control; Esc closes it and returns focus to the opener; a press outside
///   closes it and reaches nothing under it (`PopoverClose::swallow_outside(true)`, lead call 22,
///   as menus); a click on the opener closes it. It also closes when the
///   [`target`](Self::target) changes or the value changes from outside (Undo, another control),
///   and when the opener is disabled.
/// - The editor in it opens from the value, or from [`StrokeStyle::default`] when the value fails
///   [`StrokeStyle::validate`].
pub struct StrokeButton<'a> {
    #[expect(dead_code, reason = "skeleton: read by task PE-B3")]
    id_salt: IdSalt,
    #[expect(dead_code, reason = "skeleton: read by task PE-B3")]
    value: &'a StrokeStyle,
    #[expect(dead_code, reason = "skeleton: read by task PE-B3")]
    numbers: &'a LocaleNumbers,
    enabled: bool,
    closed: bool,
    corners: bool,
    target: Option<u64>,
    sections: StrokeSections,
    labels: Option<StrokeLabels>,
    look: Option<StrokeEditorStyle>,
}

impl<'a> StrokeButton<'a> {
    /// The opener of the host's `value`, its numbers formatted with `numbers`.
    pub fn new(id_salt: impl AsIdSalt, value: &'a StrokeStyle, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            enabled: true,
            closed: true,
            corners: true,
            target: None,
            sections: StrokeSections::POPOVER,
            labels: None,
            look: None,
        }
    }

    /// Whether the shape has a stroke (PhotoCraft: a width above 0). Off shows "No Color"; picking
    /// a preset then still reports. True by default.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// As [`StrokeEditor::closed`](super::StrokeEditor::closed); true by default.
    pub fn closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }

    /// As [`StrokeEditor::corners`](super::StrokeEditor::corners); true by default.
    pub fn corners(mut self, corners: bool) -> Self {
        self.corners = corners;
        self
    }

    /// The host's key of what the stroke belongs to (a layer); the popover closes when it
    /// changes. None by default (a tool's default stroke).
    pub fn target(mut self, target: u64) -> Self {
        self.target = Some(target);
        self
    }

    /// The editor's sections; [`StrokeSections::POPOVER`] by default.
    pub fn sections(mut self, sections: StrokeSections) -> Self {
        self.sections = sections;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: StrokeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// A look of the host's in place of [`StrokeEditor::style`](super::StrokeEditor::style)'s.
    pub fn look(mut self, style: StrokeEditorStyle) -> Self {
        self.look = Some(style);
        self
    }

    /// Adds the opener to `ui` and, while `open`, the popover. The host owns `open`.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-B3)")]
    pub fn show(self, ui: &mut Ui, open: &mut bool) -> StrokeButtonResponse {
        // pc: crates/ui-egui/src/tools/shape_stroke_ui/editor.rs:81-110 `button` (label :91,
        // opener 138 wide :92, toggle :94-104, close on target or value change :105-108),
        // :249-258 (Esc returns focus, a press outside closes), :37-77 `style_button`;
        // crates/ui-egui/src/tools/shape_stroke_ui.rs:164-186 `dismiss_from_canvas` (the closing
        // press reaches nothing). new: craft-ui `Popover` with `PopoverClose::swallow_outside`.
        todo!()
    }
}
