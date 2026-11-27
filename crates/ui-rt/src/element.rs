use std::any::Any;
use std::hash::Hash;

use emath::{Rect, Vec2};

use crate::access::AccessCx;
use crate::build::BuildCx;
use crate::event::{Event, EventCx, Handled};
use crate::geom::Constraints;
use crate::id::hash_key;
use crate::layout::LayoutCx;
use crate::paint::PaintCx;
use crate::platform::ImePurpose;

/// One kind of node: what component authors implement (the contract bridge implements it for
/// craft's `Component`s). Hooks map to the phases: `build` creates children, `layout` sizes and
/// places them, `paint` records display lists and hit shapes, `event` handles routed input, and
/// `access` describes the node to assistive technologies. Each runs only when its phase is dirty
/// for this node.
///
/// The parent creates a fresh value on each of its builds; the node keeps the
/// newest value and its `Local` across builds.
///
/// A child whose `props_key` is unchanged and that has no dirty mark keeps its whole subtree:
/// its build, layout and paint do not run. So the key must cover every prop that build, layout
/// or paint use. Callbacks need not be in it: the node always stores the newest value.
pub trait Element: 'static {
    /// What the parent's `show` returns: events this node emitted since the parent last built.
    type Output: Default + 'static;
    /// State the node keeps across builds (cell handles, text layouts).
    type Local: 'static;

    fn props_key(&self) -> u64;
    fn init(&self, cx: &mut BuildCx<'_>) -> Self::Local;

    /// Creates the children with [`BuildCx::child`].
    fn build(&self, local: &mut Self::Local, cx: &mut BuildCx<'_>) {
        let _ = (local, cx);
    }

    /// Lays out and places every child, and returns this node's size. The default stacks the
    /// children at the origin.
    fn layout(&self, local: &mut Self::Local, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let _ = local;
        let mut size = c.min;
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            let s = cx.layout_child(child, c.loosen());
            cx.place_child(child, Vec2::ZERO);
            size = size.max(s);
        }
        size
    }

    /// Records this node's own display list, in its local coordinates. Children paint themselves.
    fn paint(&self, local: &Self::Local, cx: &mut PaintCx<'_>) {
        let _ = (local, cx);
    }

    /// True when the node's size depends only on its constraints, so a relayout inside it
    /// never reaches its parent. Nodes with tight constraints are boundaries anyway.
    fn relayout_boundary(&self) -> bool {
        false
    }

    /// Handles an event routed to this node (see [`crate::Event`] for which events bubble).
    /// [`Handled::Yes`] stops bubbling; a press's `Down` handled here makes this node the
    /// press's owner for its later phases.
    fn event(&self, local: &mut Self::Local, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        let _ = (local, cx, ev);
        Handled::No
    }

    /// The node's access node; `None` makes it a generic container. The runtime sets bounds
    /// (the node's rect), transform (its offset) and children. Called when the node built,
    /// moved or resized, or asked for it.
    fn access(&self, local: &Self::Local, cx: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        let _ = (local, cx);
        None
    }

    /// A Tab stop. Focus order is tree order.
    fn focusable(&self) -> bool {
        false
    }

    /// A value ancestors read with [`EventCx::tag_below`] to tell which child a bubbled event
    /// came from (a row's item key). Zero is no tag.
    fn tag(&self) -> u64 {
        0
    }

    /// The node takes text input while it has focus: the runtime runs a platform IME session.
    fn text_input(&self) -> Option<ImePurpose> {
        None
    }

    /// The IME cursor rect in local coordinates, read after each layout while the node has an
    /// IME session.
    fn ime_cursor(&self, local: &Self::Local) -> Option<Rect> {
        let _ = local;
        None
    }

    /// The committed text around the caret when its revision is not `since` (the revision the
    /// session last sent; `None` at a session's start), read with the cursor while the node has an
    /// IME session. `None` when there is nothing new to send, so an unchanged text is never copied.
    fn ime_surrounding(&self, local: &Self::Local, since: Option<u64>) -> Option<crate::platform::ImeSurrounding> {
        let _ = (local, since);
        None
    }
}

/// `component.show(cx)`: craft's call shape.
pub trait Show: Element + Sized {
    fn show(self, cx: &mut BuildCx<'_>) -> Self::Output {
        cx.child(self)
    }
}

impl<E: Element> Show for E {}

pub(crate) trait AnyElement {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn build(&mut self, cx: &mut BuildCx<'_>);
    fn layout(&mut self, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2;
    fn paint(&self, cx: &mut PaintCx<'_>);
    fn relayout_boundary(&self) -> bool;
    fn event(&mut self, cx: &mut EventCx<'_>, ev: &Event) -> Handled;
    fn access(&self, cx: &mut AccessCx<'_>) -> Option<accesskit::Node>;
    fn focusable(&self) -> bool;
    fn tag(&self) -> u64;
    fn text_input(&self) -> Option<ImePurpose>;
    fn ime_cursor(&self) -> Option<Rect>;
    fn ime_surrounding(&self, since: Option<u64>) -> Option<crate::platform::ImeSurrounding>;
}

pub(crate) struct ElementBox<E: Element> {
    pub(crate) el: E,
    pub(crate) local: Option<E::Local>,
}

impl<E: Element> ElementBox<E> {
    pub(crate) fn new(el: E) -> ElementBox<E> {
        ElementBox { el, local: None }
    }
}

impl<E: Element> AnyElement for ElementBox<E> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn build(&mut self, cx: &mut BuildCx<'_>) {
        let local = match &mut self.local {
            Some(local) => local,
            None => self.local.insert(self.el.init(cx)),
        };
        self.el.build(local, cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        match &mut self.local {
            Some(local) => self.el.layout(local, cx, c),
            None => c.min,
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>) {
        if let Some(local) = &self.local {
            self.el.paint(local, cx);
        }
    }

    fn relayout_boundary(&self) -> bool {
        self.el.relayout_boundary()
    }

    fn event(&mut self, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match &mut self.local {
            Some(local) => self.el.event(local, cx, ev),
            None => Handled::No,
        }
    }

    fn access(&self, cx: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        self.local.as_ref().and_then(|local| self.el.access(local, cx))
    }

    fn focusable(&self) -> bool {
        self.el.focusable()
    }

    fn tag(&self) -> u64 {
        self.el.tag()
    }

    fn text_input(&self) -> Option<ImePurpose> {
        self.el.text_input()
    }

    fn ime_cursor(&self) -> Option<Rect> {
        self.local.as_ref().and_then(|local| self.el.ime_cursor(local))
    }

    fn ime_surrounding(&self, since: Option<u64>) -> Option<crate::platform::ImeSurrounding> {
        self.local.as_ref().and_then(|local| self.el.ime_surrounding(local, since))
    }
}

/// [`BuildCx::memo`]'s node: rebuilds its content only when the props hash moves.
pub struct Memo<P, F> {
    props: P,
    f: F,
}

impl<P: Hash + 'static, F: Fn(&P, &mut BuildCx<'_>) + 'static> Memo<P, F> {
    pub fn new(props: P, f: F) -> Self {
        Memo { props, f }
    }
}

impl<P: Hash + 'static, F: Fn(&P, &mut BuildCx<'_>) + 'static> Element for Memo<P, F> {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        hash_key(&self.props)
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        (self.f)(&self.props, cx);
    }
}

/// The node of an inline root ([`crate::Runtime::set_inline_root`]): its children come from the
/// host's closure in each frame that reaches it, and it stacks them at its origin.
pub(crate) struct InlineRoot;

impl Element for InlineRoot {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}
}
