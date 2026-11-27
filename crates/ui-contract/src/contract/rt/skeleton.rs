//! The skeleton's two components, `IconButton` and `Stack`, cut to the parts the bridge's trials
//! exercise: a leaf whose look follows the pointer and which reports clicks, and a container that
//! lays its body out one child under another. They stand in for `ui-widgets`' `IconButton` and
//! `layout_components::Stack` until the pipeline tasks move those onto craft-ui-rt.

use craft_behavior::IconId;
use craft_ui_rt::{ImageKey, Rect, Sense, Vec2, hash_key};

use super::key::ComponentKey;
use super::node::Pointer;
use super::run::{Component, Container, Cx, MeasureCx, Paint, show_container, show_leaf};
use super::ui::Ui;
use crate::contract::IdSource;

/// What `IconButton::show` returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonOutput {
    /// A click reached the button since its last run.
    pub clicked: bool,
    /// The button's rect in this frame, in window coordinates.
    pub rect: Rect,
}

/// A square button showing one icon, named by `label` for assistive technologies.
#[derive(ComponentKey)]
pub struct IconButton<'a> {
    #[key(skip)]
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
    pub fn show(self, ui: &mut Ui<'_, '_>) -> ButtonOutput {
        show_leaf(self, ui)
    }
}

impl Component for IconButton<'_> {
    const ROLE: accesskit::Role = accesskit::Role::Button;
    type Output = ButtonOutput;

    fn id_source(&self) -> IdSource {
        self.id
    }

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::splat(cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn paint(&mut self, paint: &mut Paint<'_>, _: &Cx<'_>) {
        let bounds = paint.bounds();
        let (state, radius) = (paint.tokens.color.state, paint.tokens.size.button / 4.0);
        match paint.pointer() {
            Pointer::Rest => {}
            Pointer::Hover => paint.rect(bounds, radius, state.hover, None),
            Pointer::Press => paint.rect(bounds, radius, state.pressed, None),
        }
        let glyph = Rect::from_center_size(bounds.center(), Vec2::splat(paint.tokens.size.icon.md));
        let ink = paint.tokens.color.text.normal;
        paint.image_mask(ImageKey(hash_key(&self.icon)), glyph, ink);
    }

    fn access(&self, node: &mut accesskit::Node, _: &Cx<'_>) {
        node.set_label(self.label);
    }

    fn finish(self, cx: &Cx<'_>) -> ButtonOutput {
        ButtonOutput { clicked: cx.clicked(), rect: cx.rect }
    }
}

/// Children one under another, with the theme's stack gap between them.
#[derive(ComponentKey)]
pub struct Stack {
    #[key(skip)]
    id: IdSource,
    fill_width: bool,
}

impl Stack {
    /// A stack that takes its children's size.
    pub fn new() -> Stack {
        Stack { id: IdSource::Auto, fill_width: false }
    }

    /// Where the stack's id comes from.
    pub fn id_source(mut self, id: IdSource) -> Stack {
        self.id = id;
        self
    }

    /// Takes the width of the room instead of the children's.
    pub fn fill_width(mut self) -> Stack {
        self.fill_width = true;
        self
    }

    /// Shows the stack with `body` as its children.
    pub fn show<R>(self, ui: &mut Ui<'_, '_>, body: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
        let fill = self.fill_width;
        show_container(self, ui, fill, body).0
    }
}

impl Default for Stack {
    fn default() -> Stack {
        Stack::new()
    }
}

impl Component for Stack {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = ();

    fn id_source(&self) -> IdSource {
        self.id
    }

    fn finish(self, _: &Cx<'_>) {}
}

impl Container for Stack {
    fn gap(&self, cx: &MeasureCx<'_>) -> f32 {
        cx.tokens.space.gap.stack
    }
}
