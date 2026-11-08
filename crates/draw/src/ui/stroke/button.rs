//! [`StrokeButton`]: the field that shows the stroke's style and opens the editor in a popover.

use craft_behavior::{EditEvent, GestureId, HitLevel, HitPriority, Mixed, PartKey, Sense};
use craft_ui::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, Overlay, OverlayUi, Overlays as _, PaintUi, PartsCx,
    Pointer as _, Props as _, Role, Show as _, WordSetCx as _,
};
use craft_ui::display::Text;
use craft_ui::list::ThumbAspect;
use craft_ui::pickers::SelectFace;
use craft_ui::runtime::env::Label;
use craft_ui::runtime::memory::Remember as _;
use craft_ui::style::{ComponentStyle, FocusLook, StyleCx, SurfaceLook, TypeRole};
use egui::Vec2;
use std::sync::Arc;

use super::{LineStyle, StrokeEditor, StrokePreview, StrokeSections, StrokeStyle, StrokeWords};

/// The face under the whole field.
const FACE: PartKey = PartKey::new("face");

/// The width of the face's line sample as a multiple of its height.
const SAMPLE_ASPECT: (u16, u16) = (3, 1);

/// The style of a stroke button: the field well and the focus edge; the face carries the padding.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct StrokeButtonStyle {
    /// The field well.
    #[style(surface)]
    surface: SurfaceLook,
    /// No padding of its own: the face carries the field's.
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
    #[style(focus)]
    focus: FocusLook,
}

/// What the popover was opened with, so a change of target closes it.
#[derive(Clone, Debug, Default, PartialEq)]
struct Opened(Option<Option<u64>>);

/// The stroke style's field in an options bar or the shape Properties (PhotoCraft's `shape_stroke_ui::button`) and the
/// [`StrokeEditor`] in a popover under it.
///
/// - The field shows a line sample, the style's name (Solid, Dashed, Dotted, Custom; "No Color" while
///   [`stroked`](Self::stroked) is false) and a chevron. Its accessible name is the title, "Stroke Options", with the
///   style's name as its value.
/// - A click, Enter or Space opens the popover below the field and focus moves into it; Escape closes it and returns
///   focus to the field; a press outside closes it and reaches nothing under it. It closes when the
///   [`target`](Self::target) changes and when the field is disabled.
/// - `open` follows the popover: the button writes whether it is open.
/// - The editor opens from the value, or from [`StrokeStyle::default`] when the value fails
///   [`StrokeStyle::validate`]. An edit is the button's edit of the value.
///
/// ```ignore
/// let out = StrokeButton::new("Stroke", Bind::bound(&mut style), Bind::bound(&mut open)).closed(true).target(layer).show(ui);
/// ```
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
#[component(overlay)]
pub struct StrokeButton<'a> {
    #[common]
    common: Common,
    value: Bind<'a, StrokeStyle>,
    open: Bind<'a, bool>,
    /// Whether the path is closed, as [`StrokeEditor::closed`].
    #[prop]
    closed: bool,
    /// Whether the path has corners, as [`StrokeEditor::corners`].
    #[prop]
    corners: bool,
    /// The editor's sections.
    #[prop]
    sections: StrokeSections,
    /// The host's key of what the stroke belongs to (a layer); the popover closes when it changes.
    #[prop]
    target: Option<u64>,
    /// Whether the shape has a stroke (PhotoCraft: a width above 0). Off shows "No Color".
    #[prop]
    stroked: bool,
}

impl<'a> StrokeButton<'a> {
    /// A button named `label` over `value`; `open` follows whether the popover is open.
    pub fn new(label: impl Into<Label>, value: Bind<'a, StrokeStyle>, open: Bind<'a, bool>) -> Self {
        StrokeButton {
            common: Common::new(),
            value,
            open,
            closed: true,
            corners: true,
            sections: StrokeSections::POPOVER,
            target: None,
            stroked: true,
        }
        .label(label)
    }

    /// The name the face shows for `style`.
    fn name(&self, words: &StrokeWords, style: Option<&StrokeStyle>) -> Label {
        let Some(style) = style.filter(|_| self.stroked) else { return words.no_color.clone() };
        match style.line_style() {
            LineStyle::Solid => words.solid.clone(),
            LineStyle::Dashed => words.dashed.clone(),
            LineStyle::Dotted => words.dotted.clone(),
            LineStyle::Custom => words.custom.clone(),
        }
    }

    /// The face of a field showing `text`, with the sample of `style` when the shape has a stroke. The measured
    /// candidates and the painted face are both built here.
    fn face_of(&self, text: Label, style: Option<StrokeStyle>) -> SelectFace<'static> {
        let face = SelectFace::new(text);
        match style.filter(|_| self.stroked) {
            Some(style) => {
                face.thumb(ThumbAspect::new(SAMPLE_ASPECT.0, SAMPLE_ASPECT.1), move |cx, rect| StrokePreview::sized(&style, rect).paint(cx, rect))
            }
            None => face.placeholder(),
        }
    }
}

impl Component for StrokeButton<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = StrokeButtonStyle;
    type Output = EditResponse<StrokeStyle>;

    fn style(cx: &StyleCx<'_>, _: StrokeButtonStyleKey) -> StrokeButtonStyle {
        StrokeButtonStyle { surface: cx.field(), metrics: craft_behavior::Metrics::new(), focus: cx.focus_look() }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    /// The widest face that can show, so the field keeps its width whichever name is shown.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let words = cx.words::<StrokeWords>();
        let sample = StrokeStyle::default();
        let names = [LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted, LineStyle::Custom];
        let mut size = cx.face(self.face_of(words.no_color.clone(), None));
        for line in names {
            let style = sample.with_line_style(line);
            size = size.max(cx.face(self.face_of(self.name(&words, Some(&style)), Some(style))));
        }
        size
    }

    fn parts(&mut self, rect: egui::Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(FACE, rect)
            .sense(Sense::CLICK | Sense::HOVER | Sense::TAKE_FOCUS)
            .priority(HitPriority::new(HitLevel::Body, 0))
            .pointer_only("Enter and Space open the stroke options")
            .toggles("editor")
            .on_press();
    }

    fn bind(&mut self) -> Option<BindMut<'_, StrokeStyle>> {
        Some(self.value.reborrow())
    }

    /// Enter and Space toggle the popover (a click does through the face's `toggles`). A change of target closes it.
    /// `open` is written when it differs from the popover.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("editor");
        if cx.activation().is_some_and(|activation| activation.by_key) {
            if popup.is_open(cx) {
                popup.close(cx);
            } else {
                popup.open(cx);
            }
        }
        let is_open = popup.is_open(cx);
        if is_open {
            let opened = cx.memory::<Opened>().clone();
            match opened.0 {
                Some(target) if target != self.target => popup.close(cx),
                Some(_) => {}
                None => *cx.memory::<Opened>() = Opened(Some(self.target)),
            }
        } else {
            *cx.memory::<Opened>() = Opened(None);
        }
        if let Some(Mixed::Same(held)) = self.open.reborrow().read(cx)
            && held != is_open
        {
            let name: Arc<str> = cx.label().as_str().into();
            let _ = self.open.reborrow().write(cx, &EditEvent::set(GestureId::default(), Mixed::Same(held), is_open, name, None));
        }
    }

    /// The popover: the editor over the value, from the title.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("editor");
        let words = cx.words::<StrokeWords>();
        let style = cx.value().filter(|style| style.validate().is_ok()).cloned().unwrap_or_default();
        let (sections, closed, corners) = (self.sections, self.closed, self.corners);
        let out = ui.show(popup, Overlay::popover(), |ui, _| {
            Text::new(words.title.clone()).role(TypeRole::BodyStrong).show(ui);
            StrokeEditor::new(words.title.clone(), Bind::given(style.clone())).sections(sections).closed(closed).corners(corners).show(ui)
        });
        let Some(out) = out else { return };
        match out.event {
            Some(EditEvent::Set { after, .. }) => cx.set(after),
            Some(EditEvent::Begin { .. }) => cx.begin(),
            Some(EditEvent::Update { value, .. }) => cx.propose(value),
            Some(EditEvent::Commit { after, .. }) => {
                cx.propose(after);
                cx.commit();
            }
            Some(EditEvent::Cancel { reason, .. }) => cx.cancel(reason),
            _ => {}
        }
    }

    /// The style's name as the value.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let words = cx.words::<StrokeWords>();
        access.value(self.name(&words, cx.value()).as_str().to_owned());
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let words = cx.words::<StrokeWords>();
        let style = cx.value().cloned();
        ui.face(FACE, self.face_of(self.name(&words, style.as_ref()), style));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<StrokeStyle> {
        cx.edit_response()
    }
}
