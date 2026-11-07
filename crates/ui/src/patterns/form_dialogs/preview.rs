//! `PreviewPane` and `PreviewZoom`: a picture in a well that the pointer pans and the zoom buttons scale, for the
//! preview of a filter dialog and of an export dialog.

use crate::buttons::IconButton;
use crate::contract::{
    Common, Component, Cx, Focus as _, Input, MeasureCx, NoValue, Outcome, PaintUi, PartsCx, Pointer as _, Props as _, Role, Show as _,
    ShowContainer as _,
};
use crate::display::{Progress, Text};
use crate::layout_components::Row;
use crate::runtime::access::Access;
use crate::runtime::env::{Env, Label};
use crate::runtime::focus::{ActivateKeys, KeepKeys, KeyUse};
use crate::style::{ComponentStyle, FocusLook, StyleCx};
use craft_behavior::{AccessAction, AccessData, CanvasTransform, CursorIcon, CursorPriority, IconId, Metrics, NavMove, PartKey, PressPhase, Sense};
use egui::load::SizedTexture;
use egui::{Rect, Response, Ui, Vec2, pos2};
use std::ops::RangeInclusive;

#[cfg(feature = "testing")]
mod stories;

/// The well: a drag on it pans the picture and a click centres the view on the point clicked.
const WELL: PartKey = PartKey::new("well");

/// The progress line of a pending pane and the message of a failed one.
const PROGRESS: &str = "progress";
const MESSAGE: &str = "message";

/// The zoom levels of the preview, as fractions of the picture's size.
const ZOOM_STEPS: [f32; 11] = [0.125, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0];

/// The zoom a pane allows until `zoom_range` says otherwise: the first and last of `ZOOM_STEPS`.
const ZOOM_RANGE: RangeInclusive<f32> = RangeInclusive::new(ZOOM_STEPS[0], ZOOM_STEPS[ZOOM_STEPS.len() - 1]);

/// Within this distance two zoom levels are the same level.
const ZOOM_EPSILON: f32 = 1e-4;

/// The share of the well an arrow key pans the view by.
const PAN_SHARE: f32 = 0.1;

craft_ui::words! {
    /// The words of the preview pane and its zoom row.
    pub struct PreviewWords {
        zoom_in: "formDialogs.zoomIn" = "Zoom in";
        zoom_out: "formDialogs.zoomOut" = "Zoom out";
        rendering: "formDialogs.previewRendering" = "Rendering preview";
    }
}

/// The style of a preview pane.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PreviewPaneStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
}

/// What a preview pane shows in its well.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum PreviewState {
    /// The host is rendering: the well with a progress line, `Some` of the fraction done or `None` while unknown.
    Pending(Option<f32>),
    /// The rendered picture.
    Ready(SizedTexture),
    /// The render failed: the well with this message. The label is resolved when the pane is drawn.
    Failed(Label),
}

/// What a preview pane reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct PreviewOutput {
    /// The pane's response.
    pub response: Response,
    /// The view after a pan or zoom of this pass; `None` when the view did not change. A pane that is not `Ready`
    /// reports none.
    pub view: Option<CanvasTransform>,
    /// Whether the view follows the well after this pass, when the pane was built with [`PreviewPane::fit`] and the
    /// user changed it: `Some(false)` with a `view` after a pan or zoom, `Some(true)` with the fit view after a
    /// double click that returns to fit.
    pub fit: Option<bool>,
    /// The view the pane drew this pass: the given view settled in the well, or the fit view while fitting. A zoom
    /// row steps from it.
    pub shown: CanvasTransform,
}

impl Outcome for PreviewOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A picture in a well, over a checkerboard where it is transparent. The view is a `CanvasTransform` whose canvas
/// origin is the centre of the picture and whose layer origin is the centre of the well: `scale` is the zoom (1 is
/// the picture's own size) and `offset` is how far the picture's centre lies from the well's centre. A picture
/// smaller than the well sits in the middle and one larger than it never leaves a gap at an edge.
///
/// - Drag pans, a click centres the view on the point clicked (unless `click_centres(false)`), the arrow keys pan, the
///   wheel pans and Ctrl or Cmd with the wheel zooms. A pane built with `fit` or `zoom_range` zooms about the pointer,
///   so the point under it stays; any other pane zooms about the well's centre.
/// - With `fit(true)` the zoom is the one that shows the whole picture in the well, recomputed when the well changes.
///   A pan or zoom leaves fit (`PreviewOutput::fit`) and a double click returns to it.
/// - The pane keeps no view: it shows the one given and reports the next in `PreviewOutput::view`. `PreviewZoom`
///   steps the same view with buttons. Panes that show the same picture area (a 2-up or 4-up dialog) take one view
///   and the dialog applies the first view any of them reports to all.
/// - A pane whose state is not `Ready` draws no picture and reports no view.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(content_painter)]
pub struct PreviewPane {
    #[common]
    common: Common,
    state: PreviewState,
    view: CanvasTransform,
    /// Whether the view follows the well: the zoom is the one that shows the whole picture in it, recomputed when the
    /// well changes, and the given view is ignored. A pan or zoom by the user leaves fit and reports it in
    /// `PreviewOutput::fit`; the caller passes `fit(false)` from then on. A double click returns to fit. Unset, the
    /// pane has no fit mode.
    #[prop]
    fit: Option<bool>,
    /// The zoom the wheel and fit stay inside; unset, 0.125 to 8.
    #[prop]
    zoom_range: Option<RangeInclusive<f32>>,
    /// Whether a click centres the view on the point clicked.
    #[prop]
    click_centres: bool,
    moved: Option<CanvasTransform>,
    refit: bool,
}

impl PreviewPane {
    /// A pane named `label` showing `texture` through `view`.
    pub fn new(label: impl Into<Label>, texture: SizedTexture, view: CanvasTransform) -> Self {
        Self::state(label, PreviewState::Ready(texture), view)
    }

    /// A pane named `label` showing `state` through `view`.
    pub fn state(label: impl Into<Label>, state: PreviewState, view: CanvasTransform) -> Self {
        PreviewPane { common: Common::new(), state, view, fit: None, zoom_range: None, click_centres: true, moved: None, refit: false }.label(label)
    }

    /// The zoom this pane stays inside.
    fn range(&self) -> RangeInclusive<f32> {
        self.zoom_range.clone().unwrap_or(ZOOM_RANGE)
    }

    /// Whether wheel zoom keeps the point under the pointer in place: panes with a fit mode or a zoom range of
    /// their own.
    fn zooms_about_pointer(&self) -> bool {
        self.fit.is_some() || self.zoom_range.is_some()
    }

    /// The view at the next zoom level above the view's, with the picture's centre kept in place.
    pub fn zoom_in(view: CanvasTransform) -> CanvasTransform {
        Self::zoom_in_within(view, &ZOOM_RANGE)
    }

    /// The view at the next zoom level below the view's, with the picture's centre kept in place.
    pub fn zoom_out(view: CanvasTransform) -> CanvasTransform {
        Self::zoom_out_within(view, &ZOOM_RANGE)
    }

    /// Whether `view` can zoom further in.
    pub fn can_zoom_in(view: CanvasTransform) -> bool {
        Self::can_zoom_in_within(view, &ZOOM_RANGE)
    }

    /// Whether `view` can zoom further out.
    pub fn can_zoom_out(view: CanvasTransform) -> bool {
        Self::can_zoom_out_within(view, &ZOOM_RANGE)
    }

    /// `zoom_in` with the zoom kept inside `range`: the levels are the base ones inside it and its two ends.
    pub fn zoom_in_within(view: CanvasTransform, range: &RangeInclusive<f32>) -> CanvasTransform {
        let next = Self::levels(range).into_iter().find(|level| *level > view.scale + ZOOM_EPSILON).unwrap_or(*range.end());
        Self::rescaled(view, next, range, Vec2::ZERO)
    }

    /// `zoom_out` with the zoom kept inside `range`.
    pub fn zoom_out_within(view: CanvasTransform, range: &RangeInclusive<f32>) -> CanvasTransform {
        let next = Self::levels(range).into_iter().rev().find(|level| *level < view.scale - ZOOM_EPSILON).unwrap_or(*range.start());
        Self::rescaled(view, next, range, Vec2::ZERO)
    }

    /// Whether `view` can zoom further in inside `range`.
    pub fn can_zoom_in_within(view: CanvasTransform, range: &RangeInclusive<f32>) -> bool {
        view.scale < *range.end() - ZOOM_EPSILON
    }

    /// Whether `view` can zoom further out inside `range`.
    pub fn can_zoom_out_within(view: CanvasTransform, range: &RangeInclusive<f32>) -> bool {
        view.scale > *range.start() + ZOOM_EPSILON
    }

    /// The view that shows the whole picture of `picture` points in a well of `well` points, with its zoom kept
    /// inside `range`.
    pub fn fit_view(picture: Vec2, well: Vec2, range: &RangeInclusive<f32>) -> CanvasTransform {
        let scale = if picture.x > 0.0 && picture.y > 0.0 { (well.x / picture.x).min(well.y / picture.y) } else { 1.0 };
        CanvasTransform::new(scale.clamp(*range.start(), *range.end()), Vec2::ZERO)
    }

    /// The zoom levels inside `range`, ascending: its ends and the base levels between them.
    fn levels(range: &RangeInclusive<f32>) -> Vec<f32> {
        let inside = ZOOM_STEPS.iter().copied().filter(|step| *step > *range.start() + ZOOM_EPSILON && *step < *range.end() - ZOOM_EPSILON);
        std::iter::once(*range.start()).chain(inside).chain(std::iter::once(*range.end())).collect()
    }

    /// `view` at zoom `scale` (kept inside `range`), the point `about` (from the well's centre) staying where it is.
    fn rescaled(view: CanvasTransform, scale: f32, range: &RangeInclusive<f32>, about: Vec2) -> CanvasTransform {
        let scale = scale.clamp(*range.start(), *range.end());
        let ratio = scale / view.scale;
        CanvasTransform::new(scale, about - (about - view.offset) * ratio)
    }

    /// `view` with its offset moved to where the picture of `picture` points shown in a well of `well` points allows:
    /// centred on an axis where the picture fits, else not further than the picture's edge at the well's edge.
    pub fn settled(view: CanvasTransform, picture: Vec2, well: Vec2) -> CanvasTransform {
        let slack = ((picture * view.scale - well) / 2.0).max(Vec2::ZERO);
        CanvasTransform::new(view.scale, view.offset.clamp(-slack, slack))
    }

    /// The picture of a `Ready` pane.
    fn picture(&self) -> Option<SizedTexture> {
        match &self.state {
            PreviewState::Ready(texture) => Some(*texture),
            _ => None,
        }
    }

    /// How far the picture's centre may lie from the well's centre on each axis: 0 where the picture fits.
    fn slack(&self, well: Vec2) -> Vec2 {
        match self.picture() {
            Some(texture) => ((texture.size * self.shown(well).scale - well) / 2.0).max(Vec2::ZERO),
            None => Vec2::ZERO,
        }
    }

    /// The view the pane shows in a well of `well` points: the given one settled in the well, the fit view while
    /// fitting, and the given one as it is where there is no picture.
    fn shown(&self, well: Vec2) -> CanvasTransform {
        let Some(texture) = self.picture() else { return self.view };
        let view = if self.fit == Some(true) { Self::fit_view(texture.size, well, &self.range()) } else { self.view };
        Self::settled(view, texture.size, well)
    }
}

impl Component for PreviewPane {
    const ROLE: Role = Role::GROUP;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_keep(KeepKeys::BlockArrows).with_activate(ActivateKeys::Neither);
    type Style = PreviewPaneStyle;
    type Output = PreviewOutput;

    fn style(cx: &StyleCx<'_>, _: PreviewPaneStyleKey) -> PreviewPaneStyle {
        PreviewPaneStyle {
            metrics: Metrics::new().min_size(Vec2::new(cx.tokens.size.plot.width, cx.tokens.size.plot.height)),
            focus: cx.focus_look(),
        }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn sense(&self) -> Sense {
        Sense::FOCUS | Sense::WHEEL
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let slack = self.slack(cx.rect.size());
        access.pans(slack.y > 0.0, slack.x > 0.0);
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let pannable = self.slack(rect.size()) != Vec2::ZERO;
        let part = cx.part(WELL, rect).sense(Sense::CLICK | Sense::DRAG).pointer_only("the arrow keys pan the view");
        if pannable {
            part.cursor(CursorIcon::Grab);
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(texture) = self.picture() else { return };
        let well = cx.rect;
        let shown = self.shown(well.size());
        let mut next = shown;
        match cx.part(WELL).press.phase {
            PressPhase::Drag { delta, .. } => {
                next.offset += delta;
                cx.cursor(CursorIcon::Grabbing, CursorPriority::Drag);
            }
            // The second press of a double click returns to fit, as the other double-click readers detect it.
            PressPhase::Down { start } if start.clicks >= 2 && self.fit.is_some() => self.refit = true,
            PressPhase::Release { start, pos, click: true, .. } if self.click_centres && !(start.clicks >= 2 && self.fit.is_some()) => {
                next.offset += well.center() - pos;
            }
            _ => {}
        }
        let well_size = well.size();
        let pan = |mv: NavMove| match mv {
            NavMove::Forward => Some(-Vec2::X * well_size * PAN_SHARE),
            NavMove::Back => Some(Vec2::X * well_size * PAN_SHARE),
            NavMove::Down => Some(-Vec2::Y * well_size * PAN_SHARE),
            NavMove::Up => Some(Vec2::Y * well_size * PAN_SHARE),
            _ => None,
        };
        while let Some((mv, _)) = cx.intents().take_move() {
            next.offset += pan(mv).unwrap_or(Vec2::ZERO);
        }
        // AccessKit's scroll actions are the arrows for assistive technology: the same pans.
        let rtl = cx.flow.is_rtl();
        while let Some(request) = cx.intents().take_scroll_access(false) {
            match (request.action, request.data) {
                (AccessAction::ScrollRight, _) => next.offset += pan(if rtl { NavMove::Back } else { NavMove::Forward }).unwrap_or(Vec2::ZERO),
                (AccessAction::ScrollLeft, _) => next.offset += pan(if rtl { NavMove::Forward } else { NavMove::Back }).unwrap_or(Vec2::ZERO),
                (AccessAction::ScrollDown, _) => next.offset += pan(NavMove::Down).unwrap_or(Vec2::ZERO),
                (AccessAction::ScrollUp, _) => next.offset += pan(NavMove::Up).unwrap_or(Vec2::ZERO),
                (AccessAction::SetScrollOffset, Some(AccessData::SetScrollOffset(point))) => {
                    // A scroll offset counts from the picture's top-left edge in the well; the view offset from its centre.
                    let slack = self.slack(well_size);
                    next.offset = Vec2::new(slack.x - point.x as f32, slack.y - point.y as f32);
                }
                _ => {}
            }
        }
        if let Some(wheel) = cx.take_wheel() {
            if (wheel.zoom - 1.0).abs() > ZOOM_EPSILON {
                let about = if self.zooms_about_pointer() { cx.hover_pos(WELL).map(|at| at - well.center()) } else { None };
                next = Self::rescaled(next, next.scale * wheel.zoom, &self.range(), about.unwrap_or(Vec2::ZERO));
            } else {
                next.offset += wheel.scroll;
            }
        }
        if self.refit {
            // Fitting already: nothing to return to.
            self.refit = self.fit == Some(false);
            next = Self::fit_view(texture.size, well_size, &self.range());
        } else {
            next = Self::settled(next, texture.size, well_size);
        }
        if next != shown || self.refit {
            self.moved = Some(next);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let inside = {
            let painter = ui.content_painter(cx);
            let inside = painter.plot_frame(cx.rect);
            match &self.state {
                PreviewState::Ready(texture) => {
                    let view = self.shown(cx.rect.size());
                    let size = texture.size * view.scale;
                    let picture = Rect::from_center_size(cx.rect.center() + view.offset, size);
                    let visible = picture.intersect(inside);
                    if size.x > 0.0 && size.y > 0.0 && visible.width() > 0.0 && visible.height() > 0.0 {
                        painter.checker(visible);
                        let uv = Rect::from_min_max(
                            pos2((visible.min.x - picture.min.x) / size.x, (visible.min.y - picture.min.y) / size.y),
                            pos2((visible.max.x - picture.min.x) / size.x, (visible.max.y - picture.min.y) / size.y),
                        );
                        painter.picture(texture.id, visible, uv);
                    }
                }
                PreviewState::Pending(_) => painter.checker(inside),
                PreviewState::Failed(_) => {}
            }
            inside
        };
        let (track, thickness, gap) =
            ui.with_style(cx.resolved.variant, |style| (style.tokens.size.length.track, style.track().thickness, style.tokens.space.gap.inline));
        let rendering = ui.env.words::<PreviewWords>().rendering.clone();
        match &self.state {
            PreviewState::Pending(done) => {
                let bar = Rect::from_center_size(inside.center(), Vec2::new(track.min(inside.width()), thickness));
                match done {
                    Some(fraction) => ui.child(PROGRESS, bar, Progress::new(*fraction).label(rendering)),
                    None => ui.child(PROGRESS, bar, Progress::indeterminate().label(rendering)),
                };
            }
            PreviewState::Failed(message) => {
                ui.child(MESSAGE, inside.shrink(gap), Text::new(message.clone()));
            }
            PreviewState::Ready(_) => {}
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> PreviewOutput {
        let shown = self.shown(cx.rect.size());
        let fit = match (self.fit, self.moved) {
            (None, _) | (_, None) => None,
            (Some(_), Some(_)) => Some(self.refit),
        };
        PreviewOutput { response: cx.response.clone(), view: self.moved, fit, shown }
    }
}

/// What a zoom row reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct PreviewZoomOutput {
    /// The row's response.
    pub response: Response,
    /// The view after a button of this pass; `None` when none was pressed.
    pub view: Option<CanvasTransform>,
    /// `Some(false)` with a `view`: a button leaves fit.
    pub fit: Option<bool>,
}

impl Outcome for PreviewZoomOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The zoom row of a preview: a zoom-out button, the zoom as a percentage and a zoom-in button.
#[must_use = "show the zoom row with `show`"]
#[derive(crate::contract::Pattern)]
pub struct PreviewZoom {
    view: CanvasTransform,
    range: RangeInclusive<f32>,
}

impl PreviewZoom {
    /// A zoom row for `view`. While the pane fits, `view` is the pane's `PreviewOutput::shown`.
    pub fn new(view: CanvasTransform) -> Self {
        PreviewZoom { view, range: ZOOM_RANGE }
    }

    /// The zoom the buttons stay inside: the range of the pane they step.
    #[must_use]
    pub fn zoom_range(mut self, range: RangeInclusive<f32>) -> Self {
        self.range = range;
        self
    }

    /// Shows the row.
    pub fn show(self, ui: &mut Ui) -> PreviewZoomOutput {
        let env = Env::of(ui);
        let words = env.words::<PreviewWords>();
        let readout = env.units().percent(f64::from(self.view.scale) * 100.0);
        let mut next = None;
        let row = Row::new().show(ui, |ui, _| {
            let out = IconButton::new(IconId::new("minus"))
                .label(words.zoom_out.clone())
                .enabled(PreviewPane::can_zoom_out_within(self.view, &self.range))
                .show(ui);
            Text::new(readout.clone()).show(ui);
            let into = IconButton::new(IconId::new("plus"))
                .label(words.zoom_in.clone())
                .enabled(PreviewPane::can_zoom_in_within(self.view, &self.range))
                .show(ui);
            if crate::contract::clicked(&out) {
                next = Some(PreviewPane::zoom_out_within(self.view, &self.range));
            }
            if crate::contract::clicked(&into) {
                next = Some(PreviewPane::zoom_in_within(self.view, &self.range));
            }
        });
        PreviewZoomOutput { response: row.output, fit: next.map(|_| false), view: next }
    }
}
