//! `NavigatorZoomRow`: the Navigator's percent field, Zoom Out, slider and Zoom In.

use crate::buttons::IconButton;
use crate::canvas::{NavigationOptions, ViewCommand, ViewEase};
use crate::contract::{Bind, NoValue, Outcome, Props as _, Show as _, ShowContainer as _, clicked};
use crate::layout_components::Row;
use crate::runtime::commands;
use crate::runtime::env::{Env, Label, UnitKind};
use crate::runtime::windows::os_of;
use crate::text::{NumberField, NumberStepper};
use crate::value_drag::{Slider, SliderScale};
use craft_behavior::canvas::zoom;
use craft_behavior::kurbo::Size;
use craft_behavior::{CanvasView, EditEvent, GestureId, IconId, Mixed, NumberCodec, ZoomLadder};
use egui::{Response, Ui};
use std::sync::Arc;

/// What the zoom row reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct NavigatorZoomRowOutput {
    /// The row's response.
    pub response: Response,
    /// The view change of this pass, for a binding that does not write through: a typed percent or a slider drag as
    /// its gesture, a button as one step.
    pub view: Option<EditEvent<CanvasView>>,
    /// How [`view`](Self::view) eases to its target: set by a Zoom In or Zoom Out button with Animated Zoom on, about
    /// the view's centre. The row has not written that target to the bound view; the app starts a [`ZoomAnimation`]
    /// from the view before the change and then writes the target.
    ///
    /// [`ZoomAnimation`]: crate::canvas::ZoomAnimation
    pub ease: Option<ViewEase>,
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
/// - Zoom Out and Zoom In take their label and chord from [`ViewCommand::ZoomOut`] and [`ViewCommand::ZoomIn`], step
///   the ladder about the view centre and are disabled at the ends of the range. With Animated Zoom on, the output
///   reports the ease and leaves the bound view alone for the app to ease.
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
    pub fn show(self, ui: &mut Ui) -> NavigatorZoomRowOutput {
        let NavigatorZoomRow { mut view, document, navigation } = self;
        let env = Env::of(ui);
        let words = env.words::<CanvasWords>();
        let units = env.units();
        let pixels_per_point = env.scale.0;
        let zoom_label = Label::from(words.navigator_zoom.clone());
        let name: Arc<str> = Arc::from(zoom_label.as_str());
        let platform = os_of(ui.ctx());
        let command_button = |ui: &mut Ui, icon: &'static str, command: ViewCommand, enabled: bool| {
            let resolved = commands::resolve(ui.ctx(), command);
            let mut button = IconButton::new(IconId::new(icon)).label(resolved.label.to_string()).enabled(enabled);
            if let Some(chord) = &resolved.chord {
                button = button.tooltip(format!("{} ({})", resolved.label, chord.display(platform)));
            }
            button.show(ui)
        };

        let mut event: Option<EditEvent<CanvasView>> = None;
        let mut ease: Option<ViewEase> = None;
        let shown = Row::new().show(ui, |ui, cx| {
            let Some(Mixed::Same(current)) = view.reborrow().read(cx) else { return };
            let device = zoom::device(&current, pixels_per_point);
            let range = ZoomLadder::percent_range(device, document);
            let at = |device: f64| with_device(&current, device, document, pixels_per_point);
            let mut edits: Vec<EditEvent<CanvasView>> = Vec::new();
            let mut steps: Vec<EditEvent<CanvasView>> = Vec::new();

            let ladder = |percent: f64, dir: i32| ZoomLadder::step(percent / 100.0, dir, document) * 100.0;
            let codec = NumberCodec::new(units.reader(Some(UnitKind::Percent)), PERCENT_DECIMALS);
            let field =
                NumberField::new(zoom_label.clone(), Bind::given(Some(device * 100.0))).codec(codec).range(range.clone()).stepper(ladder).show(ui);
            if let Some(typed) = field.edit.event.clone() {
                edits.push(typed.map(|percent| percent.map_or(current, |percent| at(percent / 100.0))));
            }

            let out = command_button(ui, "minus", ViewCommand::ZoomOut, device > ZoomLadder::min(document) * (1.0 + STEP_EPSILON));
            let step = |dir: i32| {
                let mut next = current;
                zoom::step(&mut next, dir, document, pixels_per_point)
                    .then(|| EditEvent::set(GestureId::default(), Mixed::Same(current), next, name.clone(), None))
            };
            if clicked(&out) {
                steps.extend(step(-1));
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

            let into = command_button(ui, "plus", ViewCommand::ZoomIn, device < ZoomLadder::MAX * (1.0 - STEP_EPSILON));
            if clicked(&into) {
                steps.extend(step(1));
            }

            // A button step that eases is not written: the app eases to it and then writes the target.
            if navigation.animated_zoom && !steps.is_empty() {
                ease = Some(ViewEase { anchor: Some((current.center, current.rect.center())) });
            } else {
                edits.append(&mut steps);
            }
            for edit in edits {
                view.reborrow().write(cx, &edit);
                event = EditEvent::fold(event.take(), edit);
            }
            for edit in steps {
                event = EditEvent::fold(event.take(), edit);
            }
        });
        NavigatorZoomRowOutput { response: shown.output, view: event, ease }
    }
}

/// The most decimals the percent field shows: the zoom ladder has rungs such as 66.67 %.
const PERCENT_DECIMALS: usize = 2;

/// How close to an end of the range a zoom must be for the button that goes further to be disabled.
const STEP_EPSILON: f64 = 1e-9;

/// `view` at `device` zoom, clamped to the ladder, with its centre kept.
fn with_device(view: &CanvasView, device: f64, document: Size, pixels_per_point: f32) -> CanvasView {
    let mut next = *view;
    next.zoom = CanvasView::points_per_unit(ZoomLadder::clamp(device, document), pixels_per_point);
    next
}
