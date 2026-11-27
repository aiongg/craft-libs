//! Putting the panel on a runtime: the one setup every run mode shares. The runner is the panel's host: its
//! root applies the edits the panel sends to the fixture document and supplies the thumbnail pictures.

use std::sync::Arc;

use craft_host::target::Os;
use craft_rt_layers::{DocCell, LayersPanel, thumb_side};
use craft_ui_rt::{BuildCx, Constraints, Element, LayoutCx, Runtime, SizeU, Vec2, WindowId, Writes};
use craft_ui_rt::RgbaImage;

use crate::fixture::{thumb_key, thumbnail};
use craft_rt_layers::pro_dark_tokens;

/// A runtime whose window `window` shows the panel over `doc`, and the cell holding the document
/// the panel shows. The platform's window gives the size and scale when the driver is made.
pub fn mount(window: WindowId, doc: craft_rt_layers::Doc) -> (Runtime, DocCell) {
    let mut runtime = Runtime::new(pro_dark_tokens());
    let cell = DocCell::create(runtime.store_mut(), doc);
    runtime.set_root(window, Host { doc: cell, os: Os::current() });
    (runtime, cell)
}

/// The panel and its host: before the panel builds, applies the edits it sent to the document and gives every
/// picture row a thumbnail picture at the side the window's scale asks for.
pub struct Host {
    pub doc: DocCell,
    pub os: Os,
}

#[derive(Default)]
pub struct HostLocal {
    side: u32,
}

impl Element for Host {
    type Output = ();
    type Local = HostLocal;

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) -> HostLocal {
        HostLocal::default()
    }

    fn build(&self, local: &mut HostLocal, cx: &mut BuildCx<'_>) {
        // The read subscribes this build to the outbox, so an edit reaches the document in the same frame.
        self.doc.has_edits(cx);
        let edits = self.doc.take_edits(cx);
        if !edits.is_empty() {
            self.doc.update_doc(cx, |d| edits.into_iter().for_each(|op| d.apply(op)));
        }
        let tokens = cx.tokens();
        let scale = cx.env().scale.get(cx);
        let side = thumb_side(&tokens, scale);
        let resized = std::mem::replace(&mut local.side, side) != side;
        let doc = self.doc.peek(cx.store_mut()).clone();
        let wanted: Vec<_> = doc
            .layers
            .iter()
            .filter(|l| !matches!(l.kind, craft_rt_layers::Kind::Type) && (resized || l.thumb.is_none()))
            .map(|l| (l.id, thumb_key(l, side), l.clone()))
            .collect();
        if wanted.is_empty() {
            cx.child(LayersPanel { doc: self.doc, os: self.os });
            return;
        }
        for (_, key, layer) in &wanted {
            if cx.images().get(*key).is_none() {
                let image = RgbaImage { size: SizeU::new(side, side), pixels: thumbnail(layer, side) };
                cx.images().insert(*key, Arc::new(image));
            }
        }
        self.doc.update_doc(cx, |d| {
            for (id, key, _) in &wanted {
                if let Some(l) = d.layers.iter_mut().find(|l| l.id == *id) {
                    l.thumb = Some(*key);
                }
            }
        });
        cx.child(LayersPanel { doc: self.doc, os: self.os });
    }

    fn layout(&self, _: &mut HostLocal, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let size = Vec2::new(if c.max.x.is_finite() { c.max.x } else { c.min.x }, if c.max.y.is_finite() { c.max.y } else { c.min.y });
        let child = cx.child_at(0);
        cx.layout_child(child, Constraints::tight(size));
        cx.place_child(child, Vec2::ZERO);
        size
    }
}
