//! The skeleton's stand-in components, cut to the parts the bridge's trials exercise: `IconButton`
//! (a leaf whose look follows the pointer and which reports clicks), `Stack` (a container that lays
//! its body out one child under another), `Slider` (a value dragged along a track part), `Toggle`
//! (a sweeping on/off value) and `ScrollArea` (a container the wheel scrolls). They stand in for the
//! `ui-widgets` components until the pipeline tasks move those onto craft-ui-rt.

use std::ops::RangeInclusive;

use craft_behavior::{IconId, PartKey, PressPhase, ValueDrag};
use craft_ui_rt::{ImageKey, Pos2, Rect, Sense, Vec2, hash_key};

use super::access::Access;
use super::container::{Container, show_container};
use super::key::ComponentKey;
use super::node::Pointer;
use super::response::{EditResponse, Response};
use super::run::{Component, Cx, MeasureCx, Paint, PartsCx, show_leaf};
use super::ui::Ui;
use crate::contract::{Bind, BindMut, ContainerProps, Gap, IdSource, MemoKey, MemoProps, Scroll, Sizing};
use crate::primitives::PaintShape;
use crate::style::{StyleColor, UiTokens, rgba_color};

/// A square button showing one icon, named by `label` for assistive technologies.
pub struct IconButton<'a> {
    id: IdSource,
    icon: IconId,
    label: &'a str,
}

impl<'a> IconButton<'a> {
    /// A button showing `icon`.
    pub fn new(icon: IconId, label: &'a str) -> IconButton<'a> {
        IconButton { id: IdSource::Auto, icon, label }
    }

    /// Where the button's id comes from.
    pub fn id_source(mut self, id: IdSource) -> IconButton<'a> {
        self.id = id;
        self
    }

    /// Shows the button.
    pub fn show(self, ui: &mut Ui<'_, '_>) -> Response {
        show_leaf(self, ui)
    }
}

impl ComponentKey for IconButton<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.icon.key(key);
        self.label.key(key);
    }
}

impl Component for IconButton<'_> {
    const ROLE: accesskit::Role = accesskit::Role::Button;
    type Output = Response;

    fn id_source(&self) -> IdSource {
        self.id
    }

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::splat(cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn paint(&mut self, paint: &Paint<'_>, _: &Cx<'_, Self>) {
        let bounds = paint.bounds();
        let (state, radius) = (paint.tokens.color.state, paint.tokens.size.button / 4.0);
        let draw = paint.draw();
        let fill = match paint.pointer() {
            Pointer::Rest => None,
            Pointer::Hover => Some(state.hover),
            Pointer::Press => Some(state.pressed),
        };
        if let Some(fill) = fill {
            draw.fill(&PaintShape::Rect(bounds, UiTokens::corner(radius)), StyleColor(rgba_color(fill)));
        }
        let glyph = Rect::from_center_size(bounds.center(), Vec2::splat(paint.tokens.size.icon.md));
        let ink = rgba_color(paint.tokens.color.text.normal);
        let uv = Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0));
        draw.canvas().image_mask(ImageKey(hash_key(&self.icon)), glyph, uv, ink);
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.label(self.label);
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Children one under another, with the theme's stack gap between them.
pub struct Stack {
    id: IdSource,
    fill_width: bool,
    /// Follows from `fill_width`, which the key holds.
    props: ContainerProps,
}

impl Stack {
    /// A stack that takes its children's size.
    pub fn new() -> Stack {
        let mut props = ContainerProps::new();
        props.body_gap = Gap::Stack;
        Stack { id: IdSource::Auto, fill_width: false, props }
    }

    /// Where the stack's id comes from.
    pub fn id_source(mut self, id: IdSource) -> Stack {
        self.id = id;
        self
    }

    /// Takes the width of the room instead of the children's.
    pub fn fill_width(mut self) -> Stack {
        self.fill_width = true;
        self.props.sizing = Sizing::FillWidth;
        self
    }

    /// Shows the stack with `body` as its children.
    pub fn show<R>(self, ui: &mut Ui<'_, '_>, body: impl FnMut(&mut Ui<'_, '_>) -> R) -> R {
        show_container(self, ui, body).0
    }
}

impl Default for Stack {
    fn default() -> Stack {
        Stack::new()
    }
}

impl ComponentKey for Stack {
    fn key(&self, key: &mut MemoKey) {
        self.fill_width.key(key);
    }
}

impl Component for Stack {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn id_source(&self) -> IdSource {
        self.id
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Stack {
    fn props(&self) -> &ContainerProps {
        &self.props
    }
}

/// A container as large as its room whose body scrolls vertically: the wheel moves its content
/// without a run.
pub struct ScrollArea {
    id: IdSource,
    props: ContainerProps,
}

impl ScrollArea {
    /// A scroll area that fills its room.
    pub fn new() -> ScrollArea {
        let mut props = ContainerProps::new().scroll(Scroll::vertical()).sizing(Sizing::Fill);
        props.body_gap = Gap::Stack;
        ScrollArea { id: IdSource::Auto, props }
    }

    /// Shows the area with `body` as its scrolled content.
    pub fn show<R>(self, ui: &mut Ui<'_, '_>, body: impl FnMut(&mut Ui<'_, '_>) -> R) -> R {
        show_container(self, ui, body).0
    }
}

impl Default for ScrollArea {
    fn default() -> ScrollArea {
        ScrollArea::new()
    }
}

impl ComponentKey for ScrollArea {
    fn key(&self, _: &mut MemoKey) {}
}

impl Component for ScrollArea {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn id_source(&self) -> IdSource {
        self.id
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for ScrollArea {
    fn props(&self) -> &ContainerProps {
        &self.props
    }
}

/// A horizontal slider over `range`: a press on its track sets the value there, a drag follows the
/// pointer, the release commits.
pub struct Slider<'a> {
    id: IdSource,
    value: Bind<'a, f64>,
    range: RangeInclusive<f64>,
}

impl<'a> Slider<'a> {
    /// The track part: the whole slider.
    pub const TRACK: PartKey = PartKey::new("track");

    /// A slider editing `value` within `range`.
    pub fn new(value: Bind<'a, f64>, range: RangeInclusive<f64>) -> Slider<'a> {
        Slider { id: IdSource::Auto, value, range }
    }

    /// Shows the slider.
    pub fn show(self, ui: &mut Ui<'_, '_>) -> EditResponse<f64> {
        show_leaf(self, ui)
    }

    fn fraction(&self, value: f64) -> f32 {
        let (lo, hi) = (*self.range.start(), *self.range.end());
        if hi == lo { 0.0 } else { ((value - lo) / (hi - lo)) as f32 }
    }
}

impl ComponentKey for Slider<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.value.key(key);
        self.range.key(key);
    }
}

/// Where a press phase puts the pointer: the press origin for `Down`, the phase's position after.
fn phase_pos(phase: PressPhase) -> Option<Pos2> {
    match phase {
        PressPhase::Down { start, .. } => Some(start.origin),
        PressPhase::DragStart { pos, .. } | PressPhase::Drag { pos, .. } | PressPhase::Release { pos, .. } | PressPhase::Held { pos, .. } => {
            Some(pos)
        }
        _ => None,
    }
}

impl Component for Slider<'_> {
    const ROLE: accesskit::Role = accesskit::Role::Slider;
    type Output = EditResponse<f64>;

    fn id_source(&self) -> IdSource {
        self.id
    }

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::new(cx.room.x.min(12.0 * cx.tokens.size.button), cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(Self::TRACK, rect).sense(Sense::CLICK | Sense::DRAG | Sense::SETS_ON_PRESS);
    }

    fn bind(&mut self) -> Option<BindMut<'_, f64>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let phase = cx.part(Self::TRACK).press.phase;
        let drag = ValueDrag::absolute(cx.rect.min.x..cx.rect.max.x, self.range.clone());
        let Some(value) = phase_pos(phase).and_then(|pos| drag.at(pos, false)) else {
            if matches!(phase, PressPhase::Cancel { .. }) {
                cx.commit();
            }
            return;
        };
        cx.propose(value);
        if matches!(phase, PressPhase::Release { .. }) {
            cx.commit();
        }
    }

    fn paint(&mut self, paint: &Paint<'_>, cx: &Cx<'_, Self>) {
        let bounds = paint.bounds();
        let tokens = paint.tokens;
        let draw = paint.draw();
        let groove = Rect::from_center_size(bounds.center(), Vec2::new(bounds.width(), tokens.size.button / 6.0));
        draw.fill(&PaintShape::Rect(groove, UiTokens::corner(groove.height() / 2.0)), StyleColor(rgba_color(tokens.color.state.hover)));
        let at = self.fraction(cx.value().copied().unwrap_or(*self.range.start())).clamp(0.0, 1.0);
        let thumb = Rect::from_center_size(Pos2::new(bounds.min.x + at * bounds.width(), bounds.center().y), Vec2::splat(bounds.height() / 2.0));
        draw.fill(&PaintShape::Rect(thumb, UiTokens::corner(thumb.height() / 2.0)), StyleColor(rgba_color(tokens.color.text.normal)));
    }

    fn finish(self, cx: &Cx<'_, Self>) -> EditResponse<f64> {
        cx.edit_response()
    }
}

/// An on/off value: a click flips it; a primary drag that starts on it sets every sibling toggle it
/// crosses to the value it took.
pub struct Toggle<'a> {
    id: IdSource,
    value: Bind<'a, bool>,
}

impl<'a> Toggle<'a> {
    /// A toggle editing `value`.
    pub fn new(value: Bind<'a, bool>) -> Toggle<'a> {
        Toggle { id: IdSource::Auto, value }
    }

    /// Shows the toggle.
    pub fn show(self, ui: &mut Ui<'_, '_>) -> EditResponse<bool> {
        show_leaf(self, ui)
    }
}

impl ComponentKey for Toggle<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.value.key(key);
    }
}

impl Component for Toggle<'_> {
    const ROLE: accesskit::Role = accesskit::Role::CheckBox;
    type Output = EditResponse<bool>;

    fn id_source(&self) -> IdSource {
        self.id
    }

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::splat(cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::DRAG
    }

    fn bind(&mut self) -> Option<BindMut<'_, bool>> {
        Some(self.value.reborrow())
    }

    fn sweep_value(&self) -> Option<bool> {
        self.value.value().copied()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let on = cx.value().copied();
        if let Some(swept) = cx.swept_toggle(on) {
            cx.set(swept);
        } else if cx.clicked() {
            cx.set(!on.unwrap_or(false));
        }
    }

    fn paint(&mut self, paint: &Paint<'_>, cx: &Cx<'_, Self>) {
        let bounds = paint.bounds().shrink(paint.tokens.size.button / 6.0);
        let fill = if cx.value().copied().unwrap_or(false) { paint.tokens.color.text.normal } else { paint.tokens.color.state.hover };
        paint.draw().fill(&PaintShape::Rect(bounds, UiTokens::corner(bounds.height() / 4.0)), StyleColor(rgba_color(fill)));
    }

    fn finish(self, cx: &Cx<'_, Self>) -> EditResponse<bool> {
        cx.edit_response()
    }
}
