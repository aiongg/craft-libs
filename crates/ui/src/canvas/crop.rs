//! `CropOverlay`: the crop rectangle of a document, a canvas control (20.6). Its corners are parts in the
//! canvas's arena; the value is in document units, measured from the top left of the rect the overlay is shown at.

use super::words::CanvasWords;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, MeasureCx, Name, PaintUi, PartsCx, Pointer as _, Props as _,
    Role, Styled as _, Validity, WordSetCx as _,
};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::runtime::env::Label;
use crate::runtime::focus::KeyUse;
use crate::style::{ComponentStyle, HandleLook, StyleColor, StyleCx};
use craft_behavior::{CursorIcon, HitLevel, HitPriority, HitShape, Metrics, PartKey, PressPhase, Sense};
use craft_theme::variant::Tone;
use egui::{CornerRadius, Pos2, Rect, Vec2};

/// A corner of the crop: `CORNER.at(i)` with `i` counted top left, top right, bottom left, bottom right.
pub(super) const CORNER: PartKey = PartKey::new("corner");

const CORNERS: u64 = 4;

/// The distance an arrow key moves a corner, in document units; Shift moves it by `NUDGE_FAR`.
const NUDGE: f32 = 1.0;
const NUDGE_FAR: f32 = 10.0;

/// The style of the crop overlay.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct CropStyle {
    #[style(metrics)]
    metrics: Metrics,
    handle: HandleLook,
    line: StyleColor,
    halo: StyleColor,
    shade: StyleColor,
    invalid: StyleColor,
}

/// The crop rectangle of a document, edited by dragging its corners or moving a focused corner with the arrow keys.
#[derive(crate::contract::Component)]
#[component(canvas_control)]
pub struct CropOverlay<'a> {
    #[common]
    common: Common,
    crop: Bind<'a, Rect>,
}

impl<'a> CropOverlay<'a> {
    /// An overlay named `label` over `crop`, a rect in document units. A mixed value shows the whole document.
    pub fn new(label: impl Into<Label>, crop: Bind<'a, Rect>) -> Self {
        CropOverlay { common: Common::new(), crop }.label(label)
    }
}

fn corners(rect: Rect) -> [Pos2; 4] {
    [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()]
}

/// The crop to show: the value, or the whole document while the value is mixed.
fn shown(value: Option<&Rect>, document: Rect) -> Rect {
    value.copied().unwrap_or_else(|| Rect::from_min_size(Pos2::ZERO, document.size()))
}

/// The crop with corner `corner` moved to `point` and the opposite corner kept, inside the document.
fn with_corner(crop: Rect, corner: u64, point: Pos2, document: Rect) -> Rect {
    let fixed = corners(crop)[(CORNERS - 1 - corner) as usize];
    Rect::from_two_pos(fixed, Rect::from_min_size(Pos2::ZERO, document.size()).clamp(point))
}

fn cursor(corner: u64) -> CursorIcon {
    if corner == 0 || corner == CORNERS - 1 { CursorIcon::ResizeNwSe } else { CursorIcon::ResizeNeSw }
}

impl Component for CropOverlay<'_> {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::CANVAS;
    type Style = CropStyle;
    type Output = EditResponse<Rect>;

    fn style(cx: &StyleCx<'_>, _: CropOverlayStyleKey) -> CropStyle {
        let handle = cx.handle();
        let overlay = &cx.tokens.color.overlay;
        CropStyle {
            metrics: Metrics::new().min_size(Vec2::splat(handle.side)),
            handle,
            line: cx.color(overlay.line),
            halo: cx.color(overlay.halo),
            shade: cx.color(cx.tokens.color.surface.scrim),
            invalid: cx.tone(Tone::Bad),
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(cx.style.handle.side)
    }

    fn bind(&mut self) -> Option<BindMut<'_, Rect>> {
        Some(self.crop.reborrow())
    }

    /// The corners: drag parts at handle priority, each its own Tab stop that the arrow keys move.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let crop = shown(cx.value(), rect);
        let (origin, radius) = (rect.min.to_vec2(), cx.style.handle.side / 2.0);
        for (at, corner) in corners(crop).into_iter().enumerate() {
            cx.part(CORNER.at(at as u64), HitShape::circle(corner + origin, radius))
                .sense(Sense::DRAG | Sense::TAKE_FOCUS)
                .priority(HitPriority::new(HitLevel::Handle, 0))
                .cursor(cursor(at as u64))
                .focusable();
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let document = cx.rect;
        let crop = shown(cx.value(), document);
        let origin = document.min.to_vec2();
        for at in 0..CORNERS {
            let phase = cx.part(CORNER.at(at)).press.phase;
            match phase {
                PressPhase::DragStart { .. } => cx.begin(),
                PressPhase::Drag { pos, .. } => cx.propose(with_corner(crop, at, pos - origin, document)),
                PressPhase::Release { .. } => cx.commit(),
                _ => {}
            }
        }
        let focused = cx.active_part().filter(|part| part.key == CORNER).and_then(|part| part.index);
        if let Some(at) = focused
            && let Some((direction, mods)) = cx.intents().take_nudge()
        {
            let step = if mods.shift { NUDGE_FAR } else { NUDGE };
            let moved = corners(crop)[at as usize] + direction * step;
            let next = with_corner(crop, at, moved, document);
            if next != crop {
                cx.set(next);
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let document = cx.rect;
        let crop = shown(cx.value(), document).translate(document.min.to_vec2());
        let line = if matches!(cx.validity(), Validity::Refuse(_)) { cx.style.invalid } else { cx.style.line };
        let draw = ui.draw();
        let bands = [
            Rect::from_min_max(document.min, Pos2::new(document.right(), crop.top())),
            Rect::from_min_max(Pos2::new(document.left(), crop.bottom()), document.max),
            Rect::from_min_max(Pos2::new(document.left(), crop.top()), Pos2::new(crop.left(), crop.bottom())),
            Rect::from_min_max(Pos2::new(crop.right(), crop.top()), Pos2::new(document.right(), crop.bottom())),
        ];
        for band in bands.into_iter().filter(|band| band.width() > 0.0 && band.height() > 0.0) {
            draw.fill(&PaintShape::Rect(band, CornerRadius::ZERO), cx.style.shade);
        }
        let outline = PaintShape::Rect(crop, CornerRadius::ZERO);
        draw.stroke(&outline, StrokeWidth::Thick, cx.style.halo);
        draw.stroke(&outline, StrokeWidth::Medium, line);
        let look = &cx.style.handle;
        for (at, corner) in corners(crop).into_iter().enumerate() {
            let square = PaintShape::Rect(Rect::from_center_size(corner, Vec2::splat(look.side)), CornerRadius::ZERO);
            draw.fill(&square, cx.resolve_part(CORNER.at(at as u64), &look.fill));
            draw.stroke(&square, StrokeWidth::Thin, look.stroke);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let words = cx.words::<CanvasWords>();
        let names = [&words.crop_top_left, &words.crop_top_right, &words.crop_bottom_left, &words.crop_bottom_right];
        for (at, name) in names.into_iter().enumerate() {
            access.node(CORNER.at(at as u64), Role::BUTTON).name = Name::Text(name.as_str().to_owned());
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<Rect> {
        cx.edit_response()
    }
}
