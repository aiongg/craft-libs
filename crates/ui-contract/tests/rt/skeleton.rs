//! The trials' stand-in components on the real `contract::Component`: `IconButton` (a leaf whose look
//! follows the pointer and which reports clicks), `Stack` (a container that lays its body out one
//! child under another), `Slider` (a value dragged along a track part), `Toggle` (a sweeping on/off
//! value) and `ScrollArea` (a container the wheel scrolls). Test-local: they stand in for the
//! `ui-widgets` components, which depend on this crate.

use std::ops::RangeInclusive;

use craft_behavior::{IconId, PartKey, PressPhase, ValueDrag};
use craft_ui_contract::contract::*;
use craft_ui_contract::primitives::PaintShape;
use craft_ui_contract::runtime::focus::{FocusPolicy, Roving};
use craft_ui_contract::style::{StyleColor, StyleCx, UiTokens, rgba_color};
use craft_ui_rt::{ImageKey, Pos2, Rect, Sense, Vec2, hash_key};
use craft_ui_test_kit::contract::PlainStyle;

/// The stand-ins' control size.
const CONTROL: f32 = 24.0;

fn theme(ui: &PaintUi<'_>) -> craft_theme::Tokens {
    ui.env.tokens.theme().clone()
}

/// A square button showing one icon, named by its label for assistive technologies.
#[derive(Component)]
pub struct IconButton {
    #[common]
    common: Common,
    icon: IconId,
}

impl IconButton {
    /// A button showing `icon`, named `label`.
    pub fn new(icon: IconId, label: &str) -> IconButton {
        IconButton { common: Common::new(), icon }.label(label.to_owned())
    }

    /// Shows the button.
    pub fn show(self, ui: &mut Ui<'_, '_>) -> Response {
        Show::show(self, ui)
    }
}

impl Component for IconButton {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: IconButtonStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(CONTROL)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    /// The look follows the pointer through the state step: a hover and a press fill.
    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let tokens = theme(ui);
        let bounds = cx.rect;
        let draw = ui.draw();
        let fill = match cx.state().state {
            craft_theme::State::Hover => Some(tokens.color.state.hover),
            craft_theme::State::Pressed => Some(tokens.color.state.pressed),
            _ => None,
        };
        if let Some(fill) = fill {
            draw.fill(&PaintShape::Rect(bounds, UiTokens::corner(CONTROL / 4.0)), StyleColor(rgba_color(fill)));
        }
        let glyph = Rect::from_center_size(bounds.center(), Vec2::splat(CONTROL * 0.66));
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        draw.canvas().image_mask(ImageKey(hash_key(&self.icon)), glyph, uv, rgba_color(tokens.color.text.normal));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Children one under another, with the theme's stack gap between them.
#[derive(Component)]
#[component(container, unregistered)]
pub struct Stack {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Stack {
    /// A stack that takes its children's size.
    pub fn new() -> Stack {
        let mut props = ContainerProps::new();
        props.body_gap = Gap::Stack;
        Stack { common: Common::new(), props }
    }

    /// Takes the width of the room instead of the children's.
    pub fn fill_width(mut self) -> Stack {
        self.props.sizing = Sizing::FillWidth;
        self
    }

    /// Makes the stops in the stack one roving group.
    pub fn roving(mut self, roving: Roving) -> Stack {
        self.props.roving = Some(roving);
        self
    }

    /// Opens a focus scope around the stack's content.
    pub fn focus(mut self, policy: FocusPolicy) -> Stack {
        self.props.focus = policy;
        self
    }

    /// Shows `text` as the side label of the stack's controls.
    pub fn labelled(mut self, text: &str) -> Stack {
        self.common.label = text.into();
        self.props = self.props.label(SideLabel::default());
        self
    }

    /// Shows the stack with `body` as its children.
    pub fn show<R>(self, ui: &mut Ui<'_, '_>, mut body: impl FnMut(&mut Ui<'_, '_>) -> R) -> R {
        ShowContainer::show(self, ui, |ui, _| body(ui)).inner.expect("a stack shows its body")
    }
}

impl Default for Stack {
    fn default() -> Stack {
        Stack::new()
    }
}

impl Component for Stack {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: StackStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Stack {}

/// A container as large as its room whose body scrolls vertically: the wheel moves its content
/// without a run.
#[derive(Component)]
#[component(container, unregistered)]
pub struct ScrollArea {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl ScrollArea {
    /// A scroll area that fills its room.
    pub fn new() -> ScrollArea {
        let mut props = ContainerProps::new().scroll(Scroll::vertical()).sizing(Sizing::Fill);
        props.body_gap = Gap::Stack;
        ScrollArea { common: Common::new(), props }
    }

    /// Shows the area with `body` as its scrolled content.
    pub fn show<R>(self, ui: &mut Ui<'_, '_>, mut body: impl FnMut(&mut Ui<'_, '_>) -> R) -> R {
        ShowContainer::show(self, ui, |ui, _| body(ui)).inner.expect("a scroll area shows its body")
    }
}

impl Default for ScrollArea {
    fn default() -> ScrollArea {
        ScrollArea::new()
    }
}

impl Component for ScrollArea {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ScrollAreaStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for ScrollArea {}

/// A horizontal slider over `range`: a press on its track sets the value there, a drag follows the
/// pointer, the release commits.
#[derive(Component)]
pub struct Slider<'a> {
    #[common]
    common: Common,
    value: Bind<'a, f64>,
    range: RangeInclusive<f64>,
}

impl<'a> Slider<'a> {
    /// The track part: the whole slider.
    pub const TRACK: PartKey = PartKey::new("track");

    /// A slider editing `value` within `range`.
    pub fn new(value: Bind<'a, f64>, range: RangeInclusive<f64>) -> Slider<'a> {
        Slider { common: Common::new(), value, range }
    }

    /// Shows the slider.
    pub fn show(self, ui: &mut Ui<'_, '_>) -> EditResponse<f64> {
        Show::show(self, ui)
    }

    fn fraction(&self, value: f64) -> f32 {
        let (lo, hi) = (*self.range.start(), *self.range.end());
        if hi == lo { 0.0 } else { ((value - lo) / (hi - lo)) as f32 }
    }
}

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
    const ROLE: Role = Role::SLIDER;
    const INPUT: Input = Input::Slider;
    type Style = PlainStyle;
    type Output = EditResponse<f64>;

    fn style(_: &StyleCx<'_>, _: SliderStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(cx.room.unwrap_or(12.0 * CONTROL).min(12.0 * CONTROL), CONTROL)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(Self::TRACK, rect).sense(Sense::CLICK | Sense::DRAG | Sense::SETS_ON_PRESS).pointer_only("the arrows step the value");
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

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let tokens = theme(ui);
        let bounds = cx.rect;
        let draw = ui.draw();
        let groove = Rect::from_center_size(bounds.center(), Vec2::new(bounds.width(), CONTROL / 6.0));
        draw.fill(&PaintShape::Rect(groove, UiTokens::corner(groove.height() / 2.0)), StyleColor(rgba_color(tokens.color.state.hover)));
        let at = self.fraction(cx.value().copied().unwrap_or(*self.range.start())).clamp(0.0, 1.0);
        let thumb = Rect::from_center_size(Pos2::new(bounds.min.x + at * bounds.width(), bounds.center().y), Vec2::splat(bounds.height() / 2.0));
        draw.fill(&PaintShape::Rect(thumb, UiTokens::corner(thumb.height() / 2.0)), StyleColor(rgba_color(tokens.color.text.normal)));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f64> {
        cx.edit_response()
    }
}

/// An on/off value: a click flips it, a primary-button sweep over a row of toggles sets each.
#[derive(Component)]
pub struct Toggle<'a> {
    #[common]
    common: Common,
    value: Bind<'a, bool>,
}

impl<'a> Toggle<'a> {
    /// A toggle editing `value`.
    pub fn new(value: Bind<'a, bool>) -> Toggle<'a> {
        Toggle { common: Common::new(), value }
    }

    /// Shows the toggle.
    pub fn show(self, ui: &mut Ui<'_, '_>) -> EditResponse<bool> {
        Show::show(self, ui)
    }
}

impl Component for Toggle<'_> {
    const ROLE: Role = Role::CHECK_BOX;
    type Style = PlainStyle;
    type Output = EditResponse<bool>;

    fn style(_: &StyleCx<'_>, _: ToggleStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(CONTROL)
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
        } else if cx.response.clicked() {
            cx.set(!on.unwrap_or(false));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let tokens = theme(ui);
        let bounds = cx.rect.shrink(CONTROL / 6.0);
        let fill = if cx.value().copied().unwrap_or(false) { tokens.color.text.normal } else { tokens.color.state.hover };
        ui.draw().fill(&PaintShape::Rect(bounds, UiTokens::corner(bounds.height() / 4.0)), StyleColor(rgba_color(fill)));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<bool> {
        cx.edit_response()
    }
}

/// A leaf with every real step a look reads: a label, a face on a part, a surface with a state fill
/// and the veil of `DisabledLook::Faded` while disabled. The count trial for the real leaf pipeline
/// shows it.
#[derive(Component)]
pub struct Chip {
    #[common]
    common: Common,
}

impl Chip {
    /// The part its face paints into.
    pub const MARK: PartKey = PartKey::new("mark");

    /// A chip named `label`.
    pub fn new(label: &str) -> Chip {
        Chip { common: Common::new() }.label(label.to_owned())
    }

    /// Shows the chip.
    pub fn show(self, ui: &mut Ui<'_, '_>) -> Response {
        Show::show(self, ui)
    }
}

impl Component for Chip {
    const ROLE: Role = Role::BUTTON;
    const DISABLED: DisabledLook = DisabledLook::Faded;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ChipStyleKey) -> PlainStyle {
        PlainStyle::surfaced()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(4.0 * CONTROL, CONTROL).max(cx.field_text(&cx.label().run()))
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let mark = Rect::from_min_size(rect.min, Vec2::splat(rect.height()));
        cx.part(Self::MARK, mark);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.face(Self::MARK, IconButton::new(IconId::new("chip"), ""));
        ui.draw().label(cx.content_rect, &cx.label());
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
