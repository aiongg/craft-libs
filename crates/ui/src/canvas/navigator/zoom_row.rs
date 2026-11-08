//! `NavigatorZoomRow`: the Navigator's percent field, Zoom Out, slider and Zoom In.

use crate::buttons::IconButton;
use crate::canvas::{CanvasWords, NavigationOptions};
use crate::contract::{Bind, NoValue, Outcome, Props as _, Show as _, ShowContainer as _, clicked};
use crate::layout_components::Row;
use crate::runtime::env::{Env, Label, UnitKind};
use crate::text::NumberField;
use crate::value_drag::{Slider, SliderScale};
use craft_behavior::canvas::zoom;
use craft_behavior::kurbo::Size;
use craft_behavior::{CanvasView, EditEvent, GestureId, IconId, Mixed, ZoomLadder};
use egui::{Response, Ui};
use std::sync::Arc;

/// What the zoom row reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct NavigatorZoomOutput {
    /// The row's response.
    pub response: Response,
    /// The view change of this pass, for a binding that does not write through: a typed percent or a slider drag as
    /// its gesture, a button as one step.
    pub view: Option<EditEvent<CanvasView>>,
    /// The change was a Zoom In or Zoom Out button with Animated Zoom on: the app starts a [`ZoomAnimation`] from the
    /// view before the change.
    ///
    /// [`ZoomAnimation`]: crate::canvas::ZoomAnimation
    pub animate_zoom: bool,
}

impl Outcome for NavigatorZoomOutput {
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
/// - Zoom Out and Zoom In step the ladder about the view centre and are disabled at the ends of the range. With
///   Animated Zoom on, the output asks the app to ease the change.
/// - The slider is logarithmic over the same range, so equal distances are equal ratios and it always holds the
///   current zoom. It follows the interface direction like every slider, so in right-to-left the value grows toward
///   the left, where Zoom In sits.
/// - Every zoom goes through [`ZoomLadder::clamp`] and keeps the view's centre.
#[must_use = "show the zoom row with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct NavigatorZoomRow<'a> {
    view: Bind<'a, CanvasView>,
    document: Size,
    /// The app's navigation options.
    #[prop]
    navigation: NavigationOptions,
}

impl<'a> NavigatorZoomRow<'a> {
    /// A zoom row that edits `view`, over a document of `document` units.
    pub fn new(view: Bind<'a, CanvasView>, document: Size) -> Self {
        NavigatorZoomRow { view, document, navigation: NavigationOptions::default() }
    }

    /// Shows the row.
    pub fn show(self, ui: &mut Ui) -> NavigatorZoomOutput {
        let NavigatorZoomRow { mut view, document, navigation } = self;
        let env = Env::of(ui);
        let words = env.words::<CanvasWords>();
        let units = env.units();
        let pixels_per_point = env.scale.0;
        let zoom_label = Label::from(words.navigator_zoom.clone());
        let name: Arc<str> = Arc::from(zoom_label.as_str());

        let mut event: Option<EditEvent<CanvasView>> = None;
        let mut animate = false;
        let shown = Row::new().show(ui, |ui, cx| {
            let Some(Mixed::Same(current)) = view.reborrow().read(cx) else { return };
            let device = zoom::device(&current, pixels_per_point);
            let range = ZoomLadder::percent_range(device, document);
            let at = |device: f64| with_device(&current, device, document, pixels_per_point);
            let mut edits: Vec<EditEvent<CanvasView>> = Vec::new();

            let field =
                NumberField::new(zoom_label.clone(), Bind::given(Some(device * 100.0))).unit_kind(UnitKind::Percent).range(range.clone()).show(ui);
            if let Some(typed) = field.edit.event.clone() {
                edits.push(typed.map(|percent| percent.map_or(current, |percent| at(percent / 100.0))));
            }

            let out = IconButton::new(IconId::new("minus"))
                .label(words.navigator_zoom_out.clone())
                .enabled(device > ZoomLadder::min(document) * (1.0 + STEP_EPSILON))
                .show(ui);
            let step = |dir: i32| {
                let mut next = current;
                zoom::step(&mut next, dir, document, pixels_per_point)
                    .then(|| EditEvent::set(GestureId::default(), Mixed::Same(current), next, name.clone(), None))
            };
            if clicked(&out) {
                let stepped = step(-1);
                animate |= navigation.animated_zoom && stepped.is_some();
                edits.extend(stepped);
            }

            let low = ZoomLadder::min(document).min(device);
            let high = ZoomLadder::MAX.max(device);
            let slider = Slider::new(Bind::given(device), low..=high)
                .scale(SliderScale::Log)
                .label(zoom_label.clone())
                .value_text(units.percent(device * 100.0))
                .show(ui);
            if let Some(dragged) = slider.event.clone() {
                edits.push(dragged.map(at));
            }

            let into = IconButton::new(IconId::new("plus"))
                .label(words.navigator_zoom_in.clone())
                .enabled(device < ZoomLadder::MAX * (1.0 - STEP_EPSILON))
                .show(ui);
            if clicked(&into) {
                let stepped = step(1);
                animate |= navigation.animated_zoom && stepped.is_some();
                edits.extend(stepped);
            }

            for edit in edits {
                view.reborrow().write(cx, &edit);
                event = EditEvent::fold(event.take(), edit);
            }
        });
        NavigatorZoomOutput { response: shown.output, view: event, animate_zoom: animate }
    }
}

/// How close to an end of the range a zoom must be for the button that goes further to be disabled.
const STEP_EPSILON: f64 = 1e-9;

/// `view` at `device` zoom, clamped to the ladder, with its centre kept.
fn with_device(view: &CanvasView, device: f64, document: Size, pixels_per_point: f32) -> CanvasView {
    let mut next = *view;
    next.zoom = CanvasView::points_per_unit(ZoomLadder::clamp(device, document), pixels_per_point);
    next
}
