//! The mount host (16.2): where steps 6 to 16 of a container run. `Inline` runs in the parent's `Ui`; the others
//! open a `Ui` of their own and call `pass` once for each pass that `Ui` gets.

use super::body::placement_of;
use super::container::PaneEdge;
use super::overlay::{self, FloatPlace, Layer, Outside, Overlay, Place};
use super::run::{Host, HostCx, HostPass, ShowAt};
use super::{ContainerProps, Mount, ShellPane};
use crate::runtime::egui::areas;
use crate::runtime::egui::panels::{self, Edge};
use crate::runtime::env::{Env, Label};
use crate::runtime::focus::FocusPolicy;
use crate::runtime::layout_memory;
use crate::runtime::memory::{SaveKey, sub};
use crate::runtime::motion::{self, Presence};
use crate::runtime::overlay::{Backdrop, CloseReason, Rank};
use crate::runtime::windows::{self, ViewportOpen, WindowHost, WindowSpec};
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
        Mount::FullWindow => match layer(&full_window_overlay(props.focus)) {
            (Some(fade), exiting) => Mounted { host: Box::new(FullWindowHost { focus: props.focus, exiting, fade, dismissed: None }), exiting },
            (None, _) => closed(),
        },
        Mount::Viewport { fallback, spec } => {
            if !WindowHost::of(ctx).viewports_available() {
                return match layer(fallback) {
                    (Some(fade), exiting) => Mounted { host: Box::new(LayerHost::new(fallback.clone(), props.save.clone(), exiting, fade)), exiting },
                    (None, _) => closed(),
                };
            }
            if open {
                let viewport = ViewportHost {
                    overlay: fallback.clone(),
                    title: title.clone(),
                    save: props.save.as_ref().map(|key| key.as_str().to_owned()),
                    spec: spec.clone(),
                    dismissed: None,
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
    fn run(&mut self, _: Id, _: &mut Ui, _: &Env, _: &mut HostPass<'_>) {}
}

/// The parent `Ui` is the host `Ui`.
struct InlineHost;

impl Host for InlineHost {
    fn run(&mut self, _: Id, parent: &mut Ui, _: &Env, pass: &mut HostPass<'_>) {
        pass(parent, None, None);
    }
}

/// A pane slot of the shell: the one place `egui::Panel` and `CentralPanel` are called.
struct PaneHost(ShellPane);

impl Host for PaneHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut HostPass<'_>) {
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
            pass(ui, None, None);
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
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut HostPass<'_>) {
        // A sizing pass of the parent shows no layer: a layer takes no room in its parent.
        if parent.is_sizing_pass() {
            return;
        }
        let ctx = parent.ctx().clone();
        let placement = placement_of(&ctx, id, self.save.as_ref());
        let mut overlay = self.overlay.clone();
        // A resizable popover's default size is held as its minimum until the user resizes it.
        if overlay.fit == overlay::OverlayFit::Resizable && placement.size.is_none() {
            overlay.min_size = match (overlay.size, overlay.min_size) {
                (Some(size), Some(min)) => Some(size.max(min)),
                (size, min) => size.or(min),
            };
        }
        if let Some(pos) = placement.pos {
            overlay.place = Place::Float(FloatPlace::at(Pos2::from(pos)));
        }
        let at = placement.pos.zip(placement.size).map(|(pos, size)| ShowAt::Rect(Rect::from_min_size(Pos2::from(pos), Vec2::from(size))));
        // Without an opener, an anchor given as a rect or point is in the parent's layer, and `Response` is the
        // parent's cursor.
        let own = Rect::from_min_max(parent.cursor().min, parent.cursor().min);
        let anchor = overlay::resolve_anchor(&ctx, &overlay, own, areas::layer_to_screen(&ctx, parent.layer_id()));
        let layer = Layer { id, owner: id, overlay: &overlay, anchor, env, exiting: self.exiting, fade: self.fade };
        let out = overlay::host(&ctx, &layer, &mut |ui| pass(ui, Some(at.unwrap_or(ShowAt::Flow)), None));
        self.dismissed = out.dismissed;
    }

    fn dismissed(&self) -> Option<CloseReason> {
        self.dismissed
    }
}

/// The overlay of a full-window mount: a modal rank with no backdrop (the dialog covers the window) and the
/// container's focus policy as its scope, so the layer stack gives it the key class, blocking and dismissal rules
/// of a modal.
fn full_window_overlay(focus: FocusPolicy) -> Overlay {
    let mut overlay = Overlay::modal().fit(overlay::OverlayFit::Content).focus(focus);
    overlay.rank = Rank::modal(Backdrop::None);
    overlay
}

/// Over the whole window, on a layer of its own.
struct FullWindowHost {
    focus: FocusPolicy,
    exiting: bool,
    fade: f32,
    dismissed: Option<CloseReason>,
}

impl Host for FullWindowHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut HostPass<'_>) {
        if parent.is_sizing_pass() {
            return;
        }
        let ctx = parent.ctx().clone();
        let window = ctx.content_rect();
        let overlay = full_window_overlay(self.focus).place(Place::Float(FloatPlace::at(window.min)));
        let layer = Layer { id, owner: id, overlay: &overlay, anchor: Rect::ZERO, env, exiting: self.exiting, fade: self.fade };
        let out = overlay::host(&ctx, &layer, &mut |ui| pass(ui, Some(ShowAt::Rect(window)), None));
        self.dismissed = out.dismissed;
    }

    fn dismissed(&self) -> Option<CloseReason> {
        self.dismissed
    }
}

/// An OS viewport of the integration; the fallback layer is `LayerHost`. The fallback overlay's rank, key class,
/// focus scope and dismissal rules (but for outside presses, which an OS window has no outside for) apply to the
/// window as they do to the layer: the window is an entry of its viewport's layer stack.
struct ViewportHost {
    overlay: Overlay,
    title: Label,
    /// The key the window's placement is saved under.
    save: Option<String>,
    /// How the window opens; none opens by the fallback's rank.
    spec: Option<WindowSpec>,
    dismissed: Option<CloseReason>,
}

impl Host for ViewportHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut HostPass<'_>) {
        let ctx = parent.ctx().clone();
        let focus = self.overlay.focus;
        let mut window_rules = self.overlay.clone();
        window_rules.dismiss.outside = Outside::Keep;
        let title = env.view().resolve(&self.title);
        // Without a spec, a modal fallback is a dialog window and anything else a tool window.
        let by_rank = || WindowSpec::dialog().kind(if self.overlay.rank.is_modal() { WindowKind::Dialog } else { WindowKind::Tool });
        let spec = self.spec.clone().unwrap_or_else(by_rank);
        let open = ViewportOpen { id, title: title.as_str(), size: layout_memory::read(&ctx, id), save: self.save.as_deref(), spec: &spec };
        let dismissed = &mut self.dismissed;
        windows::show_viewport(&ctx, &open, |ui| {
            // The container's scope on the child viewport's own root: its scale, active state and window facts.
            let scoped_env = Env::root_of(&ctx).map_or_else(|| env.clone(), |root| env.rebase(&root));
            let installed = Env::install_for(ui, scoped_env.clone());
            let window = ui.max_rect();
            let real = !ui.is_sizing_pass();
            let scoped = focus != FocusPolicy::NONE && real;
            if scoped {
                crate::runtime::focus::push_scope(&ctx, id, focus, None);
            }
            if real {
                crate::runtime::overlay::begin_layer(&ctx, id, false);
            }
            pass(ui, Some(ShowAt::Rect(window)), Some(&scoped_env));
            if real {
                crate::runtime::overlay::end_layer(&ctx);
            }
            if scoped {
                crate::runtime::focus::pop_scope(&ctx);
            }
            if real {
                let layer = Layer { id, owner: id, overlay: &window_rules, anchor: Rect::ZERO, env: &scoped_env, exiting: false, fade: 1.0 };
                *dismissed = overlay::host_window(&ctx, &layer, window).dismissed;
            }
            Env::restore(&ctx, installed);
        });
    }

    fn dismissed(&self) -> Option<CloseReason> {
        self.dismissed
    }
}
