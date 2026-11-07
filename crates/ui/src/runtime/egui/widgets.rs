//! The pass's widget rects and AccessKit nodes, for bypass detection: what egui made interactive, and which ids the
//! runtime made on purpose.

use egui::{Context, Id};

/// An egui widget the runtime makes for a purpose no component registers (a scrim that absorbs presses).
#[cfg(feature = "testing")]
pub(crate) fn note_widget(ctx: &Context, id: Id) {
    noted::with(ctx, |pass| {
        pass.widgets.insert(id);
    });
}

#[cfg(not(feature = "testing"))]
pub(crate) fn note_widget(_: &Context, _: Id) {}

/// A node the runtime wrote, so the pass-end walk knows it from a node a raw egui widget made.
#[cfg(feature = "testing")]
pub(crate) fn note_node(ctx: &Context, id: Id) {
    noted::with(ctx, |pass| {
        pass.nodes.insert(super::access::node_key(id));
    });
}

#[cfg(not(feature = "testing"))]
pub(crate) fn note_node(_: &Context, _: Id) {}

/// The ids the runtime noted in the root viewport's latest pass.
#[cfg(feature = "testing")]
pub(crate) fn noted_ids(ctx: &Context) -> (std::collections::HashSet<Id>, std::collections::HashSet<u64>) {
    noted::peek(ctx)
}

/// The interactive egui widgets of the root viewport's latest finished pass: those that sense a click or a drag
/// or take focus. A layer's own widget (an `Area`'s) is not one.
#[cfg(feature = "testing")]
pub(crate) fn interactive(ctx: &Context) -> Vec<Id> {
    ctx.viewport(|viewport| {
        let widgets = &viewport.prev_pass.widgets;
        let mut found = Vec::new();
        for (layer, rects) in widgets.layers() {
            found.extend(
                rects.iter().filter(|rect| rect.id != layer.id && (rect.sense.interactive() || rect.sense.is_focusable())).map(|rect| rect.id),
            );
        }
        found
    })
}

#[cfg(feature = "testing")]
mod noted {
    use crate::runtime::egui::memory;
    use crate::runtime::egui::pass;
    use egui::{Context, Id};
    use std::collections::HashSet;

    /// What the runtime noted in one pass; a new pass starts empty.
    #[derive(Default)]
    pub(super) struct Pass {
        pub(super) widgets: HashSet<Id>,
        pub(super) nodes: HashSet<u64>,
    }

    #[derive(Default)]
    struct Noted {
        stamp: Option<u64>,
        pass: Pass,
    }

    /// The root viewport's notes of its latest pass.
    pub(super) fn peek(ctx: &Context) -> (HashSet<Id>, HashSet<u64>) {
        memory::with(ctx, |noted: &mut Noted| (noted.pass.widgets.clone(), noted.pass.nodes.clone()))
    }

    /// Runs `f` on the running pass's notes. Only the root viewport is kept: its pass number stamps the notes, and
    /// a child viewport's pass leaves them alone.
    pub(super) fn with<R>(ctx: &Context, f: impl FnOnce(&mut Pass) -> R) -> R {
        let viewport = pass::current_viewport(ctx);
        let nr = pass::pass_nr(ctx, egui::ViewportId::ROOT);
        memory::with(ctx, |noted: &mut Noted| {
            if viewport != egui::ViewportId::ROOT {
                return f(&mut Pass::default());
            }
            if noted.stamp != Some(nr) {
                noted.stamp = Some(nr);
                noted.pass = Pass::default();
            }
            f(&mut noted.pass)
        })
    }
}
