//! `NavigatorZoomRow`: the Navigator's percent field, Zoom Out, slider and Zoom In.

use crate::buttons::IconButton;
use crate::canvas::{CanvasWords, ViewCommand};
use crate::contract::{Bind, NoValue, Outcome, Props as _, Show as _, ShowContainer as _};
use crate::layout_components::Row;
use crate::runtime::env::{Env, Label, UnitKind};
use crate::text::NumberField;
use crate::value_drag::{Slider, SliderScale};
use craft_behavior::canvas::zoom;
use craft_behavior::kurbo::Size;
use craft_behavior::{CanvasView, EditEvent, IconId, Mixed, NumberCodec, ZoomLadder};
use egui::{Response, Ui};

/// What the zoom row reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct NavigatorZoomRowOutput {
    /// The row's response.
    pub response: Response,
    /// The view change of this pass, for a binding that does not write through: a typed percent or a slider drag as
    /// its gesture.
    pub view: Option<EditEvent<CanvasView>>,
}

impl Outcome for NavigatorZoomRowOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The Navigator's zoom row: a percent field, Zoom Out, a slider over the whole zoom range and Zoom In, in reading
/// order.
///
/// - The field shows the zoom in percent, its range the document's zoom range widened to include the current zoom, so
///   the field never pulls the zoom back. A typed value applies without animation; Escape in the field restores the
///   zoom from before the edit.
/// - The field shows up to two decimals, and its arrows and wheel step the zoom ladder.
/// - Zoom Out and Zoom In are the [`ViewCommand::ZoomOut`] and [`ViewCommand::ZoomIn`] commands: label, chord, enabled
///   state and action come from the active canvas, which steps its view and reports the ease under Animated Zoom in
///   [`CanvasOutput`](crate::canvas::CanvasOutput).
/// - The slider is logarithmic over the same range, so equal distances are equal ratios and it always holds the
///   current zoom. It follows the interface direction like every slider, so in right-to-left the value grows toward
///   the left, where Zoom In sits.
/// - Every zoom goes through [`ZoomLadder::clamp`] and keeps the view's centre.
#[must_use = "show the zoom row with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct NavigatorZoomRow<'a> {
    view: Bind<'a, CanvasView>,
    document: Size,
    /// Whether the field, the buttons and the slider take input.
    #[prop]
    enabled: bool,
}

impl<'a> NavigatorZoomRow<'a> {
    /// A zoom row that edits `view`, over a document of `document` units.
    pub fn new(view: Bind<'a, CanvasView>, document: Size) -> Self {
        NavigatorZoomRow { view, document, enabled: true }
    }

    /// Shows the row.
    pub fn show(self, ui: &mut Ui) -> NavigatorZoomRowOutput {
        let NavigatorZoomRow { mut view, document, enabled } = self;
        let env = Env::of(ui);
        let words = env.words::<CanvasWords>();
        let units = env.units();
        let pixels_per_point = env.scale.0;
        let zoom_label = Label::from(words.navigator_zoom.clone());

        let mut event: Option<EditEvent<CanvasView>> = None;
        let shown = Row::new().show(ui, |ui, cx| {
            let Some(Mixed::Same(current)) = view.reborrow().read(cx) else { return };
            let device = zoom::device(&current, pixels_per_point);
            let range = ZoomLadder::percent_range(device, document);
            let at = |device: f64| with_device(&current, device, document, pixels_per_point);
            let mut edits: Vec<EditEvent<CanvasView>> = Vec::new();

            let ladder = |percent: f64, dir: i32| ZoomLadder::step(percent / 100.0, dir, document) * 100.0;
            let codec = NumberCodec::new(units.reader(Some(UnitKind::Percent)), PERCENT_DECIMALS);
            let field = NumberField::new(zoom_label.clone(), Bind::given(Some(device * 100.0)))
                .codec(codec)
                .range(range.clone())
                .stepper(ladder)
                .enabled(enabled)
                .show(ui);
            if let Some(typed) = field.edit.event.clone() {
                edits.push(typed.map(|percent| percent.map_or(current, |percent| at(percent / 100.0))));
            }

            IconButton::command(IconId::new("minus"), ViewCommand::ZoomOut).enabled(enabled).show(ui);

            let low = ZoomLadder::min(document).min(device);
            let high = ZoomLadder::MAX.max(device);
            let slider = Slider::new(Bind::given(device), low..=high)
                .scale(SliderScale::Log)
                .label(zoom_label.clone())
                .value_text(units.zoom_percent(device * 100.0))
                .enabled(enabled)
                .show(ui);
            if let Some(dragged) = slider.event.clone() {
                edits.push(dragged.map(at));
            }

            IconButton::command(IconId::new("plus"), ViewCommand::ZoomIn).enabled(enabled).show(ui);

            for edit in edits {
                view.reborrow().write(cx, &edit);
                event = EditEvent::fold(event.take(), edit);
            }
        });
        NavigatorZoomRowOutput { response: shown.output, view: event }
    }
}

/// The most decimals the percent field shows: the zoom ladder has rungs such as 66.67 %.
const PERCENT_DECIMALS: usize = 2;

/// `view` at `device` zoom, clamped to the ladder, with its centre kept.
fn with_device(view: &CanvasView, device: f64, document: Size, pixels_per_point: f32) -> CanvasView {
    let mut next = *view;
    next.zoom = CanvasView::points_per_unit(ZoomLadder::clamp(device, document), pixels_per_point);
    next
}
