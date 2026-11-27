//! The Layers panel: the list over the footer bar (look sheet 2, 3.12, 3.14).

use crate::model::Op;
use accesskit::{Node, Role};
use craft_host::target::Os;
use craft_ui_rt::{AccessCx, BuildCx, Constraints, Element, LayoutCx, PaintCx, Vec2};

use crate::controls::{ButtonKind, IconButton};
use crate::doc::{DocCell, edit};
use crate::icons::Icon;
use crate::list::List;
use crate::look::{Edge, Look};
use crate::stack::{Slot, Stack};

/// The panel as an app writes it: one call over the document cell.
pub struct LayersPanel {
    pub doc: DocCell,
    pub os: Os,
}

impl Element for LayersPanel {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        let look = Look::of(&cx.tokens());
        let (doc, os) = (self.doc, self.os);
        let slots = [Slot::Fill, Slot::Fixed(look.footer_height())];
        cx.child(
            Stack::column((), slots, move |_, cx| {
                cx.child(List { doc, os });
                cx.child(FooterBar { doc });
            })
            .gap(look.gap),
        );
        cx.child(Announcer);
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let size = Vec2::new(if c.max.x.is_finite() { c.max.x } else { c.min.x }, if c.max.y.is_finite() { c.max.y } else { c.min.y });
        let child = cx.child_at(0);
        cx.layout_child(child, Constraints::tight(size));
        cx.place_child(child, Vec2::ZERO);
        let announcer = cx.child_at(1);
        cx.layout_child(announcer, Constraints::tight(Vec2::ZERO));
        cx.place_child(announcer, Vec2::ZERO);
        size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let look = Look::of(&cx.tokens());
        cx.fill_rect(cx.bounds(), look.panel);
    }
}

/// The panel's live region for announcements: an empty status node with no area, after the
/// panel's content, as craft's runtime publishes one.
struct Announcer;

impl Element for Announcer {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
        Vec2::ZERO
    }

    fn access(&self, _: &(), _: &mut AccessCx<'_>) -> Option<Node> {
        Some(Node::new(Role::Status))
    }
}

/// The divider along the top and the two actions from the end edge, Delete outermost, a gap
/// apart; the bar is the "Actions" group.
struct FooterBar {
    doc: DocCell,
}

impl Element for FooterBar {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        let doc = self.doc;
        cx.child(IconButton::new(ButtonKind::Footer, Icon::NewLayer, "New Layer").on_press(move |cx| edit(doc, cx, Op::AddLayer)));
        cx.child(IconButton::new(ButtonKind::Footer, Icon::Trash, "Delete Layer").on_press(move |cx| edit(doc, cx, Op::DeleteSelected)));
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let look = Look::of(&cx.tokens());
        let size = Vec2::new(if c.max.x.is_finite() { c.max.x } else { c.min.x }, if c.max.y.is_finite() { c.max.y } else { c.min.y });
        let side = Constraints::tight(Vec2::new(look.control, look.control));
        let y = (size.y - look.control) / 2.0;
        // Children in reading order: New Layer, then Delete Layer at the end edge.
        for i in 0..2 {
            let from_end = (1 - i) as f32;
            let child = cx.child_at(i);
            cx.layout_child(child, side);
            cx.place_child(child, Vec2::new(size.x - look.control * (from_end + 1.0) - look.gap * from_end, y));
        }
        size
    }

    fn access(&self, _: &(), _: &mut AccessCx<'_>) -> Option<Node> {
        let mut node = Node::new(Role::Group);
        node.set_label("Actions");
        Some(node)
    }

    fn relayout_boundary(&self) -> bool {
        true
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let look = Look::of(&cx.tokens());
        let bounds = cx.bounds();
        look.edge_rule(cx, bounds, Edge::Top);
    }
}
