//! The mount host (16.2): where steps 6 to 16 of a container run. `Inline` runs in the parent's `Ui`; the others
//! open a `Ui` of their own and call `pass` once for each pass that `Ui` gets.

use super::body::{Placement, placement_id};
use super::container::PaneEdge;
use super::overlay::{self, FloatPlace, Layer, Overlay, Place};
use super::run::{Host, ShowAt};
use super::{ContainerProps, Mount, ShellPane};
use crate::runtime::egui::panels::{self, Edge};
use crate::runtime::egui::{memory as store, viewports};
use crate::runtime::env::Env;
use crate::runtime::layout_memory;
use crate::runtime::memory::sub;
use egui::{Id, Pos2, Rect, Ui, Vec2};

/// The host for `props.mount`.
pub(super) fn host(props: &ContainerProps) -> Box<dyn Host> {
    match &props.mount {
        Mount::Inline => Box::new(InlineHost),
        Mount::Pane(pane) => Box::new(PaneHost(*pane)),
        Mount::Layer(overlay) => Box::new(LayerHost(overlay.clone())),
        Mount::FullWindow => Box::new(FullWindowHost),
        Mount::Viewport { fallback } => Box::new(ViewportHost(LayerHost(fallback.clone()))),
    }
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
        let last = layout_memory::read(parent.ctx(), id).map(|size| if along_x { size.x } else { size.y });
        panels::show_pane(parent, edge, sub(id, ("pane", self.0.name)), last, |ui| {
            let installed = Env::install_for(ui, env.clone());
            pass(ui, None);
            Env::restore(&ui.ctx().clone(), installed);
        });
    }
}

/// A layer through the overlay host, at the container's placement: the overlay's own place until the user moves it.
struct LayerHost(Overlay);

impl Host for LayerHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {
        let ctx = parent.ctx().clone();
        let placement: Placement = store::peek_clone(&ctx, placement_id(id)).unwrap_or_default();
        let mut overlay = self.0.clone();
        if let Some(pos) = placement.pos {
            overlay.place = Place::Float(FloatPlace::at(Pos2::from(pos)));
        }
        let at = placement.pos.zip(placement.size).map(|(pos, size)| ShowAt::Rect(Rect::from_min_size(Pos2::from(pos), Vec2::from(size))));
        let layer = Layer { id: sub(id, "layer"), overlay: &overlay, anchor: Rect::ZERO, env, exiting: false, fade: 1.0 };
        overlay::host(&ctx, &layer, &mut |ui| pass(ui, Some(at.unwrap_or(ShowAt::Flow))));
    }
}

/// Over the whole window, on a layer of its own.
struct FullWindowHost;

impl Host for FullWindowHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {
        let ctx = parent.ctx().clone();
        let window = ctx.content_rect();
        let overlay = Overlay::floating().place(Place::Float(FloatPlace::at(window.min)));
        let layer = Layer { id: sub(id, "window"), overlay: &overlay, anchor: Rect::ZERO, env, exiting: false, fade: 1.0 };
        overlay::host(&ctx, &layer, &mut |ui| pass(ui, Some(ShowAt::Rect(window))));
    }
}

/// An OS viewport where the integration opens them, else the fallback layer.
struct ViewportHost(LayerHost);

impl Host for ViewportHost {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {
        let ctx = parent.ctx().clone();
        if !viewports::available(&ctx) {
            return self.0.run(id, parent, env, pass);
        }
        let size = layout_memory::read(&ctx, id);
        viewports::show_immediate(&ctx, viewports::viewport_id(id), "", size, |ui| {
            let installed = Env::install_for(ui, env.clone());
            let window = ui.max_rect();
            pass(ui, Some(ShowAt::Rect(window)));
            Env::restore(&ctx, installed);
        });
    }
}
