//! `OverlayUi`: what the `overlay` hook gets for the component's popups. Each open popup is a craft-ui-rt layer child
//! of the component's node (`contract::rt::overlay`): a closed popup is not reached, so its layer goes and costs
//! nothing; an open one keeps its content while nothing under it changed.

use super::dismissal::{closed_after_show, closes_before_show};
use super::placement::PopupRegion;
use super::popups::PopupHandle;
use super::props::Overlay;
use crate::contract::paint_ui::{UiHost, host_ui};
use crate::contract::rt::overlay::{BodyClose, LayerBody, LayerShow, PopupStates, Surface, show_layer_with};
use crate::contract::{BodyCx, Component, Cx, Id, Inner, Resolved, ShowContainer, Ui};
use crate::runtime::env::Env;
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::overlay::CloseReason;
use craft_behavior::{PartRef, Proposal};
use craft_ui_rt::{Pos2, Rect, Vec2};

/// What the `overlay` hook gets: the component's popups and their open state, shown as layer children of its node
/// (docs/contract-rt.md 15.1).
pub struct OverlayUi<'a> {
    /// The run's `Ui`, on the component's node: popups are its layer children.
    host: &'a mut (dyn UiHost + 'a),
    pub(crate) owner: Id,
    /// The owner's window rect in this frame.
    pub(crate) anchor: Rect,
    resolved: &'a Resolved<'a>,
    /// The env the popups' bodies run in (`contain_popups_below` narrows it).
    env: Env,
    popups: &'a mut PopupStates,
    /// The window rects of the owner's toggling parts.
    parts: Vec<(PartRef, Rect)>,
    /// The owner's hooks ran in this build: the open popups' bodies run too, since they show what its props hold.
    /// On a build that only reaches a marked node under a popup, they keep their content.
    owner_ran: bool,
    /// A popup opened or closed in a build where the owner kept its outputs: the owner runs again in this frame.
    pub(crate) changed: bool,
}

/// The component whose overlays are shown: its id, its window rect and the window rects of its parts that toggle a
/// popup.
pub(crate) struct OverlayOwner {
    pub(crate) id: Id,
    pub(crate) anchor: Rect,
    pub(crate) parts: Vec<(PartRef, Rect)>,
}

impl<'a> OverlayUi<'a> {
    pub(crate) fn new(
        host: &'a mut (dyn UiHost + 'a),
        owner: OverlayOwner,
        resolved: &'a Resolved<'a>,
        env: Env,
        popups: &'a mut PopupStates,
        owner_ran: bool,
    ) -> Self {
        let OverlayOwner { id, anchor, parts } = owner;
        OverlayUi { host, owner: id, anchor, resolved, env, popups, parts, owner_ran, changed: false }
    }

    /// Why `popup` closed in this run, as `show` decided (its dismissal, a close its body asked for) or `update`
    /// closed it; `None` while it stays open. The component that opened it reads this to settle what the popup was
    /// proposing.
    pub fn closing(&self, popup: PopupHandle) -> Option<CloseReason> {
        self.popups.closing(popup.id())
    }

    /// Whether a layer the owner opens lays out right to left (`Scoped::layer_direction`): a layer its caller places by
    /// position mirrors its own geometry by it, as an anchored placement mirrors the side.
    pub fn is_rtl(&self) -> bool {
        self.env.layer_direction().is_rtl()
    }

    /// The anchor at the rect of the toggling part `part` of the owner. The layer's placement mirrors the side and
    /// alignment of an anchor in a right-to-left layout, so the same call serves both directions.
    pub fn anchor_part(&self, part: impl Into<PartRef>) -> super::Anchor {
        let part = part.into();
        let rect = self.parts.iter().find(|(declared, _)| *declared == part).map(|(_, rect)| *rect);
        debug_assert!(rect.is_some(), "`anchor_part` names a part that declared `toggles`");
        super::Anchor::Rect(rect.unwrap_or(self.anchor))
    }

    /// Keeps the popups this component shows from here on, and every popup opened from them, below the component:
    /// inside the window less the part above the component's bottom edge, and above the bottom of the inherited
    /// `PopupRegion` when one is set (the bottom slots of a shell). A menu bar's menus and their submenus never cover
    /// the bar.
    pub fn contain_popups_below(&mut self) {
        let room = host_ui(&mut *self.host, |ui| ui.ctx().content_rect());
        let bottom = self.env.get::<PopupRegion>().0.map_or(room.bottom(), |region| region.bottom().min(room.bottom())).max(room.top());
        let top = self.anchor.bottom().clamp(room.top(), bottom);
        let region = Rect::from_min_max(Pos2::new(room.left(), top), Pos2::new(room.right(), bottom));
        self.env = self.env.derive(|scoped| {
            scoped.inherited_mut().values.set(PopupRegion(Some(region)));
        });
    }

    /// Shows the popup while its handle is open, on a popup surface inside `POPUP_PAD`, with `body` as its content;
    /// `None` while it is closed. The value is the body's when it ran in this build, else the one it returned last.
    ///
    /// The popup is sized from its content: a new popup measures its body before it is placed, a kept one is placed by
    /// its kept size and builds again in the same frame when that size moved. A child that fills the width takes the
    /// width the popup settles on, not the window's. What its body keeps in memory lasts while the popup shows.
    pub fn show<R: Clone + 'static>(
        &mut self,
        popup: PopupHandle,
        overlay: Overlay,
        mut body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R,
    ) -> Option<R> {
        let env = self.env.clone();
        let mut content = PopupContent { body: &mut body, env: &env, resolved: self.resolved, id: sub(popup.id(), "body") };
        self.show_layer(popup, &overlay, Surface::Popup, None, &mut content)
    }

    /// Shows `container` as the layer of `popup` while the popup is open: the container is the layer's content and
    /// takes its id, and its own steps run inside it (move band and grips, `placement_changed`, its close). A close the
    /// container decides (its dismissal rules, Escape, a close its body asked for) closes the popup, and
    /// [`closing`](Self::closing) reports it, as for [`show`](Self::show). `None` while the popup is closed, and in a
    /// build where the layer kept its content.
    pub fn show_container<C: ShowContainer, R>(
        &mut self,
        popup: PopupHandle,
        container: C,
        mut body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R,
    ) -> Option<Inner<C::Output, R>> {
        let crate::contract::Mount::Layer(overlay) = container.props(crate::contract::Internal::new()).mount.clone() else {
            debug_assert!(false, "`show_container` shows a container mounted as `Mount::Layer`");
            return None;
        };
        let mut content = ContainerContent { container: Some(container), body: &mut body, shown: None };
        self.show_layer(popup, &overlay, Surface::Plain, Some(popup.id()), &mut content)?;
        content.shown
    }

    /// Shows the layer of an open `popup` with `content`, closing it first when it was dismissed since the owner's last
    /// build, and after, when its content closed it.
    fn show_layer<R: Clone + 'static>(
        &mut self,
        popup: PopupHandle,
        overlay: &Overlay,
        surface: Surface,
        host: Option<Id>,
        content: &mut dyn LayerBody<R>,
    ) -> Option<R> {
        if !self.popups.is_open(popup.id()) {
            return None;
        }
        let (anchor, owner_ran, rtl, enabled) = (self.anchor, self.owner_ran, self.is_rtl(), self.resolved.enabled);
        let hover = self.popups.watches_hover(popup.id());
        // A popup with no opener hangs from the owner's node; a part anchor hangs from the part's rect.
        let key = popup.id().value();
        let shown = host_ui(&mut *self.host, |ui| {
            let (tokens, sweep) = (ui.flow.tokens.clone(), ui.flow.sweep);
            let cx = &mut *ui.cx;
            let base = LayerShow::of(overlay, cx.node(), anchor, rtl, &tokens);
            let layer = base.layer;
            if let Some(reason) = closes_before_show(cx, overlay, layer, key, enabled) {
                return Err(reason);
            }
            let pad = if surface == Surface::Plain { 0.0 } else { base.pad };
            let show = LayerShow { sweep, surface, pad, host, hover, ..base };
            let value = show_layer_with(cx, popup.id(), key, show, owner_ran, content);
            Ok((value, closed_after_show(cx, layer, key), layer))
        });
        match shown {
            Err(reason) => {
                self.close(popup, reason);
                None
            }
            Ok((value, closed, layer)) => {
                self.popups.shown_on(popup.id(), layer);
                if let Some(reason) = closed {
                    self.close(popup, reason);
                }
                Some(value)
            }
        }
    }

    fn close(&mut self, popup: PopupHandle, reason: CloseReason) {
        if self.popups.close(popup.id(), reason) {
            self.changed = true;
        }
    }
}

/// A popup's body closure as a layer's content: it runs in the popup's env with a `BodyCx` on the layer's memory.
struct PopupContent<'b, R> {
    body: &'b mut dyn FnMut(&mut Ui, &mut BodyCx<'_>) -> R,
    env: &'b Env,
    resolved: &'b Resolved<'b>,
    id: Id,
}

impl<R> PopupContent<'_, R> {
    fn run(&mut self, ui: &mut Ui<'_, '_>, memory: &mut MemoryStage) -> (R, Option<CloseReason>) {
        let rect = ui.flow.region;
        let mut open = true;
        let (out, asked) = Env::run_in(ui, self.env.clone(), |ui| {
            let mut bcx = BodyCx::new(self.id, self.env.view(), self.resolved, rect, memory, &mut open, None, None);
            let out = (self.body)(ui, &mut bcx);
            (out, bcx.close_requested())
        });
        (out, asked.or((!open).then_some(CloseReason::Requested)))
    }
}

impl<R> LayerBody<R> for PopupContent<'_, R> {
    fn measure(&mut self, ui: &mut Ui<'_, '_>, memory: &mut MemoryStage) -> Vec2 {
        self.run(ui, memory);
        ui.flow.content_size()
    }

    fn show(&mut self, ui: &mut Ui<'_, '_>, memory: &mut MemoryStage, closed: &BodyClose) -> R {
        let (out, close) = self.run(ui, memory);
        if close.is_some() && closed.get().is_none() {
            closed.set(close);
        }
        out
    }
}

/// A container as a layer's content: measured through its size cache, shown at the layer's body region. It finds
/// the layer it is the content of as the provided `LayerHost`, and reports its close there.
struct ContainerContent<'b, C: ShowContainer, R> {
    container: Option<C>,
    body: &'b mut dyn FnMut(&mut Ui, &mut BodyCx<'_>) -> R,
    shown: Option<Inner<C::Output, R>>,
}

impl<C: ShowContainer, R> LayerBody<()> for ContainerContent<'_, C, R> {
    fn measure(&mut self, ui: &mut Ui<'_, '_>, _: &mut MemoryStage) -> Vec2 {
        let room = ui.flow.region.size();
        let body = &mut *self.body;
        self.container.as_mut().map_or(Vec2::ZERO, |container| container.measure_in(ui, Proposal::Ideal, |ui, cx| body(ui, cx)).min(room))
    }

    fn show(&mut self, ui: &mut Ui<'_, '_>, _: &mut MemoryStage, _: &BodyClose) {
        let rect = ui.flow.region;
        let body = &mut *self.body;
        if let Some(container) = self.container.take() {
            self.shown = Some(container.show_at(ui, rect, |ui, cx| body(ui, cx)));
        }
    }
}

/// Step 10 of a run: the `overlay` hook of `component`, with the popups the run holds (`CxCore::popups`), shown on
/// `host` (the run's `Ui` on the component's node). `parts` are the window rects of the toggling parts. Returns
/// whether a popup opened or closed here, so a node that kept its outputs runs again in this frame.
pub(crate) fn run_overlay<C: Component>(
    component: &mut C,
    cx: &mut Cx<'_, C>,
    host: &mut (dyn UiHost + '_),
    env: &Env,
    parts: Vec<(PartRef, Rect)>,
    owner_ran: bool,
) -> bool {
    let mut popups = std::mem::take(&mut cx.core.popups);
    let owner = OverlayOwner { id: cx.core.id, anchor: cx.core.rect, parts };
    let resolved = cx.core.resolved;
    let mut overlays = OverlayUi::new(host, owner, resolved, env.clone(), &mut popups, owner_ran);
    component.overlay(&mut overlays, cx);
    let changed = overlays.changed;
    drop(overlays);
    cx.core.popups = popups;
    changed
}
