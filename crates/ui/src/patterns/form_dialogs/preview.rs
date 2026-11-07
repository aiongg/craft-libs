//! `PreviewPane` and `PreviewZoom`: a picture in a well that the pointer pans and the zoom buttons scale, for the
//! preview of a filter dialog and of an export dialog.

use crate::buttons::IconButton;
use crate::contract::{
    Common, Component, Cx, Focus as _, Input, MeasureCx, NoValue, Outcome, PaintUi, PartsCx, Pointer as _, Props as _, Role, Show as _,
    ShowContainer as _,
};
use crate::display::Text;
use crate::layout_components::Row;
use crate::runtime::env::{Env, Label};
use crate::runtime::focus::{ActivateKeys, KeepKeys, KeyUse};
use crate::style::{ComponentStyle, FocusLook, StyleCx};
use craft_behavior::{CanvasTransform, CursorIcon, CursorPriority, IconId, Metrics, NavMove, PartKey, PressPhase, Sense};
use egui::load::SizedTexture;
use egui::{Color32, Rect, Response, Ui, Vec2, pos2};

#[cfg(feature = "testing")]
mod stories;

/// The well: a drag on it pans the picture and a click centres the view on the point clicked.
const WELL: PartKey = PartKey::new("well");

/// The zoom levels of the preview, as fractions of the picture's size.
const ZOOM_STEPS: [f32; 11] = [0.125, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0];

/// Within this distance two zoom levels are the same level.
const ZOOM_EPSILON: f32 = 1e-4;

/// The share of the well an arrow key pans the view by.
const PAN_SHARE: f32 = 0.1;

/// The tint that shows the picture as it is.
const UNTINTED: Color32 = Color32::from_gray(u8::MAX);

craft_ui::words! {
    /// The words of the preview pane and its zoom row.
    pub struct PreviewWords {
        zoom_in: "formDialogs.zoomIn" = "Zoom in";
        zoom_out: "formDialogs.zoomOut" = "Zoom out";
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

/// What a preview pane reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct PreviewOutput {
    /// The pane's response.
    pub response: Response,
    /// The view after a pan or zoom of this pass; `None` when the view did not change.
    pub view: Option<CanvasTransform>,
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
/// - Drag pans, a click centres the view on the point clicked, the arrow keys pan, the wheel pans and Ctrl or Cmd with
///   the wheel zooms.
/// - The pane keeps no view: it shows the one given and reports the next in `PreviewOutput::view`. `PreviewZoom`
///   steps the same view with buttons.
#[derive(crate::contract::Component)]
#[component(content_painter)]
pub struct PreviewPane {
    #[common]
    common: Common,
    texture: SizedTexture,
    view: CanvasTransform,
    moved: Option<CanvasTransform>,
}

impl PreviewPane {
    /// A pane named `label` showing `texture` through `view`.
    pub fn new(label: impl Into<Label>, texture: SizedTexture, view: CanvasTransform) -> Self {
        PreviewPane { common: Common::new(), texture, view, moved: None }.label(label)
    }

    /// The view at the next zoom level above the view's, with the picture's centre kept in place.
    pub fn zoom_in(view: CanvasTransform) -> CanvasTransform {
        let next = ZOOM_STEPS.iter().copied().find(|step| *step > view.scale + ZOOM_EPSILON).unwrap_or(ZOOM_STEPS[ZOOM_STEPS.len() - 1]);
        Self::rescaled(view, next)
    }

    /// The view at the next zoom level below the view's, with the picture's centre kept in place.
    pub fn zoom_out(view: CanvasTransform) -> CanvasTransform {
        let next = ZOOM_STEPS.iter().rev().copied().find(|step| *step < view.scale - ZOOM_EPSILON).unwrap_or(ZOOM_STEPS[0]);
        Self::rescaled(view, next)
    }

    /// Whether `view` can zoom further in.
    pub fn can_zoom_in(view: CanvasTransform) -> bool {
        view.scale < ZOOM_STEPS[ZOOM_STEPS.len() - 1] - ZOOM_EPSILON
    }

    /// Whether `view` can zoom further out.
    pub fn can_zoom_out(view: CanvasTransform) -> bool {
        view.scale > ZOOM_STEPS[0] + ZOOM_EPSILON
    }

    /// `view` at zoom `scale` (kept inside the zoom levels), the point under the well's centre staying there.
    fn rescaled(view: CanvasTransform, scale: f32) -> CanvasTransform {
        let scale = scale.clamp(ZOOM_STEPS[0], ZOOM_STEPS[ZOOM_STEPS.len() - 1]);
        CanvasTransform::new(scale, view.offset * (scale / view.scale))
    }

    /// `view` with its offset moved to where the picture of `picture` points shown in a well of `well` points allows:
    /// centred on an axis where the picture fits, else not further than the picture's edge at the well's edge.
    pub fn settled(view: CanvasTransform, picture: Vec2, well: Vec2) -> CanvasTransform {
        let slack = ((picture * view.scale - well) / 2.0).max(Vec2::ZERO);
        CanvasTransform::new(view.scale, view.offset.clamp(-slack, slack))
    }

    /// The view the pane shows in a well of `well` points.
    fn shown(&self, well: Vec2) -> CanvasTransform {
        Self::settled(self.view, self.texture.size, well)
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
        Sense::FOCUS
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let pannable = self.texture.size.x * self.view.scale > rect.width() || self.texture.size.y * self.view.scale > rect.height();
        let part = cx.part(WELL, rect).sense(Sense::CLICK | Sense::DRAG).pointer_only("the arrow keys pan the view");
        if pannable {
            part.cursor(CursorIcon::Grab);
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let well = cx.rect;
        let shown = self.shown(well.size());
        let mut next = shown;
        match cx.part(WELL).press.phase {
            PressPhase::Drag { delta, .. } => {
                next.offset += delta;
                cx.cursor(CursorIcon::Grabbing, CursorPriority::Drag);
            }
            PressPhase::Release { pos, click: true, .. } => next.offset += well.center() - pos,
            _ => {}
        }
        while let Some((mv, _)) = cx.intents().take_move() {
            let by = match mv {
                NavMove::Forward => -Vec2::X,
                NavMove::Back => Vec2::X,
                NavMove::Down => -Vec2::Y,
                NavMove::Up => Vec2::Y,
                _ => continue,
            } * well.size()
                * PAN_SHARE;
            next.offset += by;
        }
        if let Some(wheel) = cx.take_wheel() {
            if (wheel.zoom - 1.0).abs() > ZOOM_EPSILON {
                next = Self::rescaled(next, next.scale * wheel.zoom);
            } else {
                next.offset += wheel.scroll;
            }
        }
        let next = Self::settled(next, self.texture.size, well.size());
        if next != shown {
            self.moved = Some(next);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let painter = ui.content_painter(cx);
        let inside = painter.plot_frame(cx.rect);
        let view = self.shown(cx.rect.size());
        let size = self.texture.size * view.scale;
        let picture = Rect::from_center_size(cx.rect.center() + view.offset, size);
        let visible = picture.intersect(inside);
        if size.x <= 0.0 || size.y <= 0.0 || visible.width() <= 0.0 || visible.height() <= 0.0 {
            return;
        }
        painter.checker(visible);
        let uv = Rect::from_min_max(
            pos2((visible.min.x - picture.min.x) / size.x, (visible.min.y - picture.min.y) / size.y),
            pos2((visible.max.x - picture.min.x) / size.x, (visible.max.y - picture.min.y) / size.y),
        );
        crate::runtime::egui::textures::paint(&painter.painter, self.texture.id, visible, uv, UNTINTED);
    }

    fn finish(self, cx: Cx<'_, Self>) -> PreviewOutput {
        PreviewOutput { response: cx.response.clone(), view: self.moved }
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
}

impl PreviewZoom {
    /// A zoom row for `view`.
    pub fn new(view: CanvasTransform) -> Self {
        PreviewZoom { view }
    }

    /// Shows the row.
    pub fn show(self, ui: &mut Ui) -> PreviewZoomOutput {
        let env = Env::of(ui);
        let words = env.words::<PreviewWords>();
        let readout = env.units().percent(f64::from(self.view.scale) * 100.0);
        let mut next = None;
        let row = Row::new().show(ui, |ui, _| {
            let out = IconButton::new(IconId::new("minus")).label(words.zoom_out.clone()).enabled(PreviewPane::can_zoom_out(self.view)).show(ui);
            Text::new(readout.clone()).show(ui);
            let into = IconButton::new(IconId::new("plus")).label(words.zoom_in.clone()).enabled(PreviewPane::can_zoom_in(self.view)).show(ui);
            if crate::contract::clicked(&out) {
                next = Some(PreviewPane::zoom_out(self.view));
            }
            if crate::contract::clicked(&into) {
                next = Some(PreviewPane::zoom_in(self.view));
            }
        });
        PreviewZoomOutput { response: row.output, view: next }
    }
}
