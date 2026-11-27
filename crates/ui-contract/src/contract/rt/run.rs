//! A component's run inside its scope's build, and the keep decision before it
//! (docs/contract-rt.md sections 3.3, 4, 7 and 8).
//!
//! The skeleton's [`Component`] has the hooks of `contract::Component` that `IconButton` and
//! `Stack` use, over contexts on craft-ui-rt. The pipeline tasks move the real trait's contexts
//! (`Cx`, `MeasureCx`, `PaintUi`, `Draw`) onto these and retire this cut.

use std::sync::Arc;

use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, CornerRadii, DisplayList, Gesture, ImageKey, InlineState, Item, PointerButton, Pos2, Rect, Rgba, Sense, Stroke, Vec2};
use smallvec::SmallVec;

use super::key::{ComponentKey, component_key, fold, type_key};
use super::node::{ComponentNode, Kept, NodeInput, Place, Pointer, Variant};
use super::ui::Ui;
use crate::contract::IdSource;

/// What `measure` sees: the room the component is shown in.
pub struct MeasureCx<'a> {
    /// The room the component is shown in.
    pub room: Vec2,
    /// The tokens the node's run read.
    pub tokens: &'a Tokens,
}

/// The facts of one run, or of a kept node (settled: no input, the kept facts).
pub struct Cx<'a> {
    /// The node's rect in this frame, in window coordinates.
    pub rect: Rect,
    /// The tokens of the run, or of the scope that shows a kept node.
    pub tokens: &'a Tokens,
    input: &'a [NodeInput],
    settled: bool,
}

impl<'a> Cx<'a> {
    pub(crate) fn settled(rect: Rect, tokens: &'a Tokens) -> Cx<'a> {
        Cx { rect, tokens, input: &[], settled: true }
    }

    /// The inputs queued since the last run, oldest first.
    pub fn input(&self) -> &[NodeInput] {
        self.input
    }

    /// The node did not run: its outputs are the kept ones.
    pub fn is_settled(&self) -> bool {
        self.settled
    }

    /// A primary click or an assistive technology's click reached the node since the last run.
    pub fn clicked(&self) -> bool {
        self.input.iter().any(|i| match i {
            NodeInput::Pointer { gesture: Gesture::Up { click, .. }, button, .. } => *click && *button == PointerButton::Primary,
            NodeInput::Access(action) => *action == accesskit::Action::Click,
            _ => false,
        })
    }
}

/// Records one paint variant in the node's local coordinates, and whether the paint read the
/// pointer state.
pub struct Paint<'a> {
    /// The tokens of the run.
    pub tokens: &'a Tokens,
    size: Vec2,
    pointer: Pointer,
    read: bool,
    over: bool,
    content: DisplayList,
    overlay: DisplayList,
}

impl Paint<'_> {
    /// The pointer state this variant shows. Reading it makes the run record a variant per state.
    pub fn pointer(&mut self) -> Pointer {
        self.read = true;
        self.pointer
    }

    /// The node's rect in its own coordinates.
    pub fn bounds(&self) -> Rect {
        Rect::from_min_size(Pos2::ZERO, self.size)
    }

    /// Records `item` at the current level.
    pub fn push(&mut self, item: Item) {
        if self.over { self.overlay.items.push(item) } else { self.content.items.push(item) }
    }

    /// A rounded rect, filled and optionally stroked.
    pub fn rect(&mut self, rect: Rect, radii: impl Into<CornerRadii>, fill: Rgba, stroke: Option<Stroke>) {
        self.push(Item::Rect { rect, radii: radii.into(), fill, stroke });
    }

    /// `color` through an image's alpha (an icon mask).
    pub fn image_mask(&mut self, key: ImageKey, rect: Rect, color: Rgba) {
        self.push(Item::ImageMask { key, rect, uv: Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), color });
    }

    /// Records what `f` adds over the node's children.
    pub fn over<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = std::mem::replace(&mut self.over, true);
        let out = f(self);
        self.over = saved;
        out
    }
}

/// The skeleton's component-author trait: `contract::Component`'s hooks over craft-ui-rt contexts.
/// The hooks run in the build of the scope that shows the component, and only when the keep
/// decision says so; their outputs are kept per node.
pub trait Component: ComponentKey + Sized {
    /// The AccessKit role.
    const ROLE: accesskit::Role;
    /// What `show` returns.
    type Output;

    /// Where the node's id comes from (`Common`'s id source).
    fn id_source(&self) -> IdSource {
        IdSource::Auto
    }

    /// The node's size in the room it has.
    fn measure(&mut self, _cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::ZERO
    }

    /// How input reaches the component.
    fn sense(&self) -> Sense {
        Sense::empty()
    }

    /// Sees the queued input ([`Cx::input`]).
    fn update(&mut self, _cx: &mut Cx<'_>) {}

    /// Paints in local coordinates; may run once per pointer state it reads.
    fn paint(&mut self, _paint: &mut Paint<'_>, _cx: &Cx<'_>) {}

    /// Fills the node's AccessKit node (its role is set).
    fn access(&self, _node: &mut accesskit::Node, _cx: &Cx<'_>) {}

    /// Runs at every `show`: after the run, or on the settled `Cx` of a kept node.
    fn finish(self, cx: &Cx<'_>) -> Self::Output;
}

/// Whether a node keeps its outputs: it ran before with this component key and carries no mark.
/// Its room may have moved: what it shows depends on its size, which the size cache or `measure`
/// gives for the new room.
pub(crate) fn holds(kept: &Kept, key: Option<u128>, state: InlineState) -> bool {
    !state.must_run() && key.is_some() && kept.key == key && !kept.variants.is_empty()
}

/// Shows a leaf: keeps it, or runs its hooks. In a measuring run it only reports its size: a kept
/// node's from its size cache, otherwise `measure`'s.
pub fn show_leaf<C: Component>(component: C, ui: &mut Ui<'_, '_>) -> C::Output {
    let (id, child) = ui.flow.child(type_key::<C>(), component.id_source());
    let key = component_key(&component);
    let room = ui.flow.room();
    let mut component = component;
    if let Some(under) = ui.flow.measuring {
        let cached =
            under.and_then(|u| ui.cx.peek::<ComponentNode>(u, child)).filter(|p| holds(p.local, key, p.state)).and_then(|p| p.local.cached(room));
        let tokens = ui.flow.tokens.clone();
        let size = cached.unwrap_or_else(|| component.measure(&MeasureCx { room, tokens: &tokens }));
        let rect = ui.flow.allocate(None, size, false);
        return component.finish(&Cx::settled(rect, &tokens));
    }
    let Ui { cx, flow } = ui;
    cx.keyed(child, |cx| {
        cx.inline(ComponentNode { key: key.map_or(0, fold) }, |kept, state, cx| {
            let kept_outputs = holds(kept, key, state);
            let size = match kept.cached(room).filter(|_| kept_outputs) {
                Some(size) => size,
                None => {
                    let tokens = if kept_outputs { flow.tokens.clone() } else { cx.tokens() };
                    component.measure(&MeasureCx { room, tokens: &tokens })
                }
            };
            let rect = flow.allocate(Some(cx.node()), size, false);
            if kept_outputs && size == kept.size {
                cx.keep_children();
                kept.cache(room, size);
                return component.finish(&Cx::settled(rect, &flow.tokens));
            }
            let tokens = cx.tokens();
            run(component, kept, cx, Ran { id, key, place: Place { room }, size, rect, tokens: &tokens })
        })
    })
}

/// The facts of a run decided before its hooks.
pub(crate) struct Ran<'a> {
    pub(crate) id: egui::Id,
    pub(crate) key: Option<u128>,
    pub(crate) place: Place,
    pub(crate) size: Vec2,
    pub(crate) rect: Rect,
    pub(crate) tokens: &'a Arc<Tokens>,
}

/// Runs the hooks after `measure`: input, update, paint variants, access, finish. The node's
/// layout, paint and access run in this frame.
pub(crate) fn run<C: Component>(mut component: C, kept: &mut Kept, cx: &mut BuildCx<'_>, ran: Ran<'_>) -> C::Output {
    cx.count_run();
    cx.mark_changed();
    let sense = component.sense();
    let input = std::mem::take(&mut kept.queue);
    let mut rcx = Cx { rect: ran.rect, tokens: ran.tokens, input: &input, settled: false };
    component.update(&mut rcx);
    kept.variants = record_variants(&mut component, &rcx, ran.size, sense);
    let mut node = accesskit::Node::new(C::ROLE);
    component.access(&mut node, &rcx);
    kept.access = Some(node);
    kept.id = ran.id;
    kept.key = ran.key;
    kept.place = ran.place;
    kept.size = ran.size;
    kept.forget_sizes();
    kept.cache(ran.place.room, ran.size);
    kept.sense = sense;
    component.finish(&rcx)
}

/// One paint, then one per further pointer state when the paint read it (`Press` only for a
/// component that takes presses).
fn record_variants<C: Component>(component: &mut C, cx: &Cx<'_>, size: Vec2, sense: Sense) -> SmallVec<[Variant; 1]> {
    let mut paint_as = |look: Pointer| {
        let mut p = Paint {
            tokens: cx.tokens,
            size,
            pointer: look,
            read: false,
            over: false,
            content: DisplayList::default(),
            overlay: DisplayList::default(),
        };
        component.paint(&mut p, cx);
        (Variant { look, content: p.content, overlay: p.overlay }, p.read)
    };
    let (rest, read) = paint_as(Pointer::Rest);
    let mut out = SmallVec::new();
    out.push(rest);
    if read {
        out.push(paint_as(Pointer::Hover).0);
        if sense.takes_press() {
            out.push(paint_as(Pointer::Press).0);
        }
    }
    out
}
