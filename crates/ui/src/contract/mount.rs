//! The mount host (16.2): where steps 6 to 16 of a container run. `Inline` runs in the parent's `Ui`; the others
//! open a `Ui` of their own and call `pass` once for each pass that `Ui` gets.

use super::body::placement_of;
use super::container::PaneEdge;
use super::overlay::{self, FloatPlace, Layer, Overlay, Place};
use super::run::{Host, HostCx, ShowAt};
use super::{ContainerProps, Mount, ShellPane};
use crate::runtime::egui::memory as store;
use crate::runtime::egui::panels::{self, Edge};
use crate::runtime::egui::areas;
use crate::runtime::env::{Env, Label};
use crate::runtime::focus::FocusPolicy;
use crate::runtime::layout_memory;
use crate::runtime::memory::{SaveKey, sub};
use crate::runtime::motion::{self, Presence};
use crate::runtime::overlay::CloseReason;
use crate::runtime::windows::{self, ViewportOpen, WindowHost};
use craft_host::target::WindowKind;
use egui::{Id, Pos2, Rect, Ui, Vec2};

/// The host a mount chose, and whether it shows the container fading out.
pub(super) struct Mounted {
    pub host: Box<dyn Host>,
    /// The container is closed and its layer is still fading: the body still shows, as it was.
    pub exiting: bool,
}

/// The host for `props.mount`. A layer-like mount of a closed container shows nothing once its exit fade ended.
pub(super) fn host(props: &ContainerProps, at: &HostCx<'_>, open: bool, title: &Label) -> Mounted {
    let ctx = at.ui.ctx();
    let layer = |overlay: &Overlay| {
        let presence = if at.ui.is_sizing_pass() {
            Presence { t: 1.0, visible: open, exiting: false }
        } else {
            motion::layer_presence(ctx, sub(at.id, "presence"), at.id, at.env.motion, open, overlay.motion)
        };
        (presence.visible.then_some(presence.t), !open)
    };
    let closed = || Mounted { host: Box::new(ClosedHost), exiting: false };
    match &props.mount {
        Mount::Inline => Mounted { host: Box::new(InlineHost), exiting: false },
        Mount::Pane(pane) => Mounted { host: Box::new(PaneHost(*pane)), exiting: false },
        Mount::Layer(overlay) => match layer(overlay) {
            (Some(fade), exiting) => Mounted { host: Box::new(LayerHost::new(overlay.clone(), props.save.clone(), exiting, fade)), exiting },
            (None, _) => closed(),
        },
        Mount::FullWindow => match layer(&Overlay::floating()) {
            (Some(fade), exiting) => Mounted { host: Box::new(FullWindowHost { exiting, fade }), exiting },
            (None, _) => closed(),
        },
        Mount::Viewport { fallback } => {
            if !WindowHost::of(ctx).viewports_available() {
                return match layer(fallback) {
                    (Some(fade), exiting) => Mounted { host: Box::new(LayerHost::new(fallback.clone(), props.save.clone(), exiting, fade)), exiting },
                    (None, _) => closed(),
                };
            }
            if open {
                let viewport = ViewportHost {
                    focus: fallback.focus,
                    title: title.clone(),
                    save: props.save.as_ref().map(|key| key.as_str().to_owned()),
                };
                Mounted { host: Box::new(viewport), exiting: false }
            } else {
                closed()
            }
        }
    }
}

/// A closed layer-like container: nothing shows and no layer is recorded.
struct ClosedHost;

impl Host for ClosedHost {
    fn run(&mut self, _: Id, _: &mut Ui, _: &Env, _: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {}
}

/// The parent `Ui` is the host `Ui`.
struct InlineHost;

impl Host for InlineHost {
    fn run(&mut self, _: Id, parent: &mut Ui, _: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {
        pass(parent, None);
    }
}

/// A pane slot of the shell: the one place `egui::Panel` and `CentralPanel` are called.
struct PaneHost(ShellPane);

impl Host for PaneHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {
        let rtl = env.direction.is_rtl();
        let (edge, along_x) = match self.0.edge {
            PaneEdge::Start => (if rtl { Edge::Right } else { Edge::Left }, true),
            PaneEdge::End => (if rtl { Edge::Left } else { Edge::Right }, true),
            PaneEdge::Top => (Edge::Top, false),
            PaneEdge::Bottom => (Edge::Bottom, false),
            _ => (Edge::Center, false),
        };
        let ctx = parent.ctx().clone();
        // The pane's extent is its own memory, seeded from the size the pane first took and kept until the user
        // resizes it: the body's size does not carry it, since a filling body takes what the pane gives.
        let extent = sub(id, "pane_extent");
        let last = layout_memory::read(&ctx, extent).map(|size| if along_x { size.x } else { size.y });
        let outer = panels::show_pane(parent, edge, sub(id, ("pane", self.0.name)), last, |ui| {
            let installed = Env::install_for(ui, env.clone());
            pass(ui, None);
            Env::restore(&ui.ctx().clone(), installed);
        });
        if last.is_none() && edge != Edge::Center && outer.is_positive() {
            layout_memory::record(&ctx, extent, outer.size());
        }
    }
}

/// A layer through the overlay host, at the container's placement: the overlay's own place until the user moves it.
/// The placement lives in the container's saved slot when it has a `SaveKey`, else in shown memory.
struct LayerHost {
    overlay: Overlay,
    save: Option<SaveKey>,
    exiting: bool,
    fade: f32,
    dismissed: Option<CloseReason>,
}

impl LayerHost {
    fn new(overlay: Overlay, save: Option<SaveKey>, exiting: bool, fade: f32) -> Self {
        LayerHost { overlay, save, exiting, fade, dismissed: None }
    }
}

impl Host for LayerHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {
        // A sizing pass of the parent shows no layer: a layer takes no room in its parent.
        if parent.is_sizing_pass() {
            return;
        }
        let ctx = parent.ctx().clone();
        let placement = placement_of(&ctx, id, self.save.as_ref());
        let mut overlay = self.overlay.clone();
        if let Some(pos) = placement.pos {
            overlay.place = Place::Float(FloatPlace::at(Pos2::from(pos)));
        }
        let at = placement.pos.zip(placement.size).map(|(pos, size)| ShowAt::Rect(Rect::from_min_size(Pos2::from(pos), Vec2::from(size))));
        // Without an opener, an anchor given as a rect or point is in the parent's layer, and `Response` is the
        // parent's cursor.
        let own = Rect::from_min_max(parent.cursor().min, parent.cursor().min);
        let anchor = overlay::resolve_anchor(&ctx, &overlay, own, areas::layer_to_screen(&ctx, parent.layer_id()));
        let layer = Layer { id, owner: id, overlay: &overlay, anchor, env, exiting: self.exiting, fade: self.fade };
        let out = overlay::host(&ctx, &layer, &mut |ui| pass(ui, Some(at.unwrap_or(ShowAt::Flow))));
        self.dismissed = out.dismissed;
    }

    fn dismissed(&self) -> Option<CloseReason> {
        self.dismissed
    }
}

/// Over the whole window, on a layer of its own.
struct FullWindowHost {
    exiting: bool,
    fade: f32,
}

impl Host for FullWindowHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {
        if parent.is_sizing_pass() {
            return;
        }
        let ctx = parent.ctx().clone();
        let window = ctx.content_rect();
        let overlay = Overlay::floating().place(Place::Float(FloatPlace::at(window.min)));
        let layer = Layer { id, owner: id, overlay: &overlay, anchor: Rect::ZERO, env, exiting: self.exiting, fade: self.fade };
        overlay::host(&ctx, &layer, &mut |ui| pass(ui, Some(ShowAt::Rect(window))));
    }
}

/// An OS viewport of the integration; the fallback layer is `LayerHost`.
struct ViewportHost {
    /// The fallback overlay's focus policy: the viewport's root scope.
    focus: FocusPolicy,
    title: Label,
    /// The key the window's placement is saved under.
    save: Option<String>,
}

impl Host for ViewportHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {
        let ctx = parent.ctx().clone();
        let focus = self.focus;
        let title = env.resolve(&self.title);
        let open =
            ViewportOpen { id, title: title.as_str(), size: layout_memory::read(&ctx, id), save: self.save.as_deref(), kind: WindowKind::Tool };
        windows::show_viewport(&ctx, &open, |ui| {
            let installed = Env::install_for(ui, env.clone());
            let window = ui.max_rect();
            let scoped = focus != FocusPolicy::NONE && !ui.is_sizing_pass();
            if scoped {
                crate::runtime::focus::push_scope(&ctx, id, focus, None);
            }
            pass(ui, Some(ShowAt::Rect(window)));
            if scoped {
                crate::runtime::focus::pop_scope(&ctx);
            }
            Env::restore(&ctx, installed);
        });
    }
}
