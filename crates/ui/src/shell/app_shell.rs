//! `AppShell`: the window skeleton. An ordered stack of top bars starting with the title bar, bottom bars, start and end
//! rails, and the body in what is left. Each bar and rail is a container on `Mount::Pane`, so F6 pane focus and the
//! name of each region come from the pipeline.

#[cfg(feature = "testing")]
mod stories;

use super::reveal::{Aim, PanelsHidden, Reveal, Tracking};
use super::screen_mode::{FullScreenPlan, ScreenMode, ScreenModeCommand};
use super::words::AppShellWords;
use crate::contract::overlay::Resize;
use crate::contract::{
    Bind, BodyCx, Common, Component, Container, ContainerProps, Cx, Mount, NoValue, Outcome, PaneEdge, Props as _, Remember as _, Role, ShellPane,
    ShowContainer as _,
};
use crate::notices::ToastStack;
use crate::runtime::commands::{CommandSet as _, Commands as _};
use crate::runtime::egui::pass;
use crate::runtime::env::{AppValues, Env, Label, RevealEdge, ShellCx, ShellRegions, SlotKey, WindowKey};
use crate::runtime::focus::FocusPolicy;
use crate::runtime::frame::{ChromeToggle, request_repaint};
use crate::runtime::overlay::{OpenFilter, any_open};
use crate::runtime::pointer;
use crate::runtime::windows::{WindowHost, os_of};
use crate::shell::Slot;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{EditEvent, GestureId, Metrics, Mixed, Sense};
use craft_host::target::Os;
use egui::{Rect, Response, Ui, ViewportId};
use std::sync::Arc;

/// The closure that fills a slot or the body.
pub(super) type RegionFn<'a> = Box<dyn FnMut(&mut Ui, &mut BodyCx<'_>) + 'a>;

/// The closure that fills a region, as a `func` prop.
pub(super) struct RegionBody<'a>(pub(super) RegionFn<'a>);

impl<'a> RegionBody<'a> {
    /// The body that runs `body`.
    pub(super) fn new(body: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        RegionBody(Box::new(body))
    }
}

/// The style of a shell region: the surface comes from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ShellRegionStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a shell region reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ShellRegionOutput {
    /// The region's response; its rect is the region's place in the window.
    pub response: Response,
}

impl Outcome for ShellRegionOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A region of the window skeleton: the root window's content, the scope that carries the shell to its slots, or one
/// pane (a bar, a rail or the body).
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct ShellRegion {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    shell: Option<ShellCx>,
    panels_hidden: PanelsHidden,
}

impl ShellRegion {
    /// The whole content of the window: its edges resize the window where the app draws the frame.
    pub fn window() -> Self {
        let props =
            ContainerProps::new().mount(Mount::window(Resize::GRIPS)).surface(SurfaceKind::Background).command_scope(ScreenModeCommand::SCOPE);
        ShellRegion { common: Common::new(), props, shell: None, panels_hidden: PanelsHidden::new(false) }
    }

    /// An inline region that fills its room and sets `shell` for everything inside it.
    pub fn scope(shell: ShellCx, panels_hidden: PanelsHidden) -> Self {
        let props = ContainerProps::new().sizing(crate::contract::Sizing::Fill);
        ShellRegion { common: Common::new(), props, shell: Some(shell), panels_hidden }
    }

    /// A pane of the window; it fills the room its edge gives it.
    pub fn pane(pane: ShellPane) -> Self {
        let props = ContainerProps::new().mount(Mount::Pane(pane)).focus(FocusPolicy::PANE);
        ShellRegion { common: Common::new(), props, shell: None, panels_hidden: PanelsHidden::new(false) }
    }
}

impl Component for ShellRegion {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = ShellRegionStyle;
    type Output = ShellRegionOutput;

    fn style(_: &StyleCx<'_>, _: ShellRegionStyleKey) -> ShellRegionStyle {
        ShellRegionStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn content_env(&self, _: &Cx<'_, Self>, values: &mut AppValues) {
        if let Some(shell) = &self.shell {
            shell.put(values);
            values.set(self.panels_hidden);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> ShellRegionOutput {
        ShellRegionOutput { response: cx.response.clone() }
    }
}

impl Container for ShellRegion {}

/// One bar or rail with the edge it sits on and the closure that fills it.
struct Placed<'a> {
    slot: Slot,
    edge: PaneEdge,
    add: RegionFn<'a>,
}

impl Placed<'_> {
    /// The order slots claim space in: the title and top bars, the bottom bars, the start rails, the end rails.
    fn rank(&self) -> u8 {
        match self.edge {
            PaneEdge::Top => 0,
            PaneEdge::Bottom => 1,
            PaneEdge::Start => 2,
            _ => 3,
        }
    }
}

/// What the shell keeps between passes, in the root container's slot.
#[derive(Clone, Debug, Default)]
struct ShellState {
    reveal: Reveal,
    plan: FullScreenPlan,
    body: Option<Rect>,
    slots: Vec<(SlotKey, Rect)>,
}

/// What a pass of the shell leaves.
#[derive(Clone, Debug)]
struct Parts {
    regions: ShellRegions,
    revealed: bool,
    panels_hidden: bool,
    mode: ScreenMode,
}

/// What [`AppShell::show`] drew.
#[derive(Debug)]
#[non_exhaustive]
pub struct AppShellOutput {
    /// The response of the window's root region.
    pub response: Response,
    /// The body's rect and the rects of the shown slots, in the order drawn.
    pub regions: ShellRegions,
    /// Slots are shown by a reveal on this pass (Tab or edge).
    pub revealed: bool,
    /// The panels are hidden while the tools and bars stay (Shift+Tab). A body that hosts panels reads it with
    /// `Env::get::<PanelsHidden>()`.
    pub panels_hidden: bool,
    /// The screen mode after this pass.
    pub screen_mode: ScreenMode,
}

impl Outcome for AppShellOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The window skeleton.
///
/// - **Order.** Slots claim space in this order, which is also the F6 order: the title, then the `top` slots in call
///   order from the window's top edge inward; the `bottom` slots from the bottom edge inward; then `start`, then `end`;
///   then the body in what is left. Start and end swap sides in a right-to-left window; top and bottom do not. A theme or
///   size never changes the order.
/// - **Screen mode.** The `screen_mode` binding holds the window's mode. The window's `view.screenMode.*` commands
///   (F and Shift+F cycle) change it, and a change to or from full screen is carried out through `WindowHost::act`.
///   In `FullScreen` no slot shows but those built with [`Slot::reveal`], during a reveal.
/// - **Reveal.** In body-only mode Tab (or the `window.toggle_panels` command) shows the revealing slots until the next
///   Tab; the pointer at the top, start or end edge shows that edge's slots, unless a button is held or a popup is
///   open or `edge_reveal` is off. A screen-mode change ends a reveal. The shell acts on the toggles it is given with
///   `chrome_toggles` (see [`chrome_toggles`](super::chrome_toggles)): in normal mode Tab hides every slot and panel and
///   shows them again.
/// - **Shell data.** Slots and the body read the skeleton with `BodyCx::shell()`.
#[must_use = "show the shell with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct AppShell<'a> {
    window: WindowKey,
    bare: bool,
    /// The screen mode. Without a binding the shell keeps it, starting at `Standard`.
    #[prop]
    screen_mode: Bind<'a, ScreenMode>,
    /// Whether the Tab reveal and the edge reveal work in body-only mode. Off for a presentation.
    #[prop]
    allow_reveal: bool,
    /// Whether the pointer at the top, start or end edge reveals the hidden bars in body-only mode.
    #[prop]
    edge_reveal: bool,
    /// The host's flag for the document tabs, read back through `ShellCx::tabs_visible`.
    #[prop]
    document_tabs: bool,
    /// The name of the body region; the translated "Document" without it.
    #[prop(into)]
    body_label: Option<Label>,
    /// The chrome toggles of this frame, from [`chrome_toggles`](super::chrome_toggles).
    #[prop]
    chrome_toggles: &'a [(ViewportId, ChromeToggle)],
    title: Option<Placed<'a>>,
    slots: Vec<Placed<'a>>,
    /// The body, shown in what the slots leave.
    #[prop(func = "FnMut(&mut Ui, &mut BodyCx<'_>)")]
    body: Option<RegionBody<'a>>,
}

impl<'a> AppShell<'a> {
    /// A shell for `window` with no slots.
    pub fn new(window: WindowKey) -> Self {
        AppShell {
            window,
            bare: false,
            screen_mode: Bind::kept(),
            allow_reveal: true,
            edge_reveal: true,
            document_tabs: true,
            body_label: None,
            chrome_toggles: &[],
            title: None,
            slots: Vec::new(),
            body: None,
        }
    }

    /// A window with no chrome (a torn-off document window): only a body, with the same window scope. Slots added to
    /// it are not shown.
    pub fn bare(window: WindowKey) -> Self {
        AppShell { bare: true, ..Self::new(window) }
    }

    /// The title bar slot, the first top slot, keyed `"title"`: visible, and revealing in body-only mode. The title bar
    /// types fill it.
    pub fn title_bar(self, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.title(Slot::new("title").reveal(), add)
    }

    /// The title slot with a slot of your own, for a title that follows the host's menu bar flag.
    pub fn title(mut self, slot: Slot, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.title = Some(Placed { slot, edge: PaneEdge::Top, add: Box::new(add) });
        self
    }

    /// A top slot, below the ones added before it.
    pub fn top(self, slot: Slot, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.place(PaneEdge::Top, slot, add)
    }

    /// A bottom slot, above the ones added before it.
    pub fn bottom(self, slot: Slot, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.place(PaneEdge::Bottom, slot, add)
    }

    /// A slot at the reading start, inside the ones added before it.
    pub fn start(self, slot: Slot, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.place(PaneEdge::Start, slot, add)
    }

    /// A slot at the reading end, inside the ones added before it.
    pub fn end(self, slot: Slot, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.place(PaneEdge::End, slot, add)
    }

    fn place(mut self, edge: PaneEdge, slot: Slot, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.slots.push(Placed { slot, edge, add: Box::new(add) });
        self
    }

    /// Lays out the slots, then the body in what is left, and shows the toasts of the window.
    pub fn show(self, ui: &mut Ui) -> AppShellOutput {
        let AppShell { window, bare, mut screen_mode, allow_reveal, edge_reveal, document_tabs, body_label, chrome_toggles, title, slots, mut body } =
            self;
        let words = Env::of(ui).words::<AppShellWords>();
        let body_label = body_label.unwrap_or_else(|| words.document.clone());
        let mut slots: Vec<Placed<'a>> = title.into_iter().chain(slots).collect();
        slots.sort_by_key(Placed::rank);

        let root = ShellRegion::window().label(words.window.clone());
        let shown = root.show(ui, |ui, cx| {
            let ctx = ui.ctx().clone();
            let viewport = pass::current_viewport(&ctx);
            let mut state = cx.keep::<ShellState>().clone();

            let before = match screen_mode.reborrow().read(cx) {
                Some(Mixed::Same(held)) => held,
                _ => ScreenMode::default(),
            };
            let mut mode = before;
            while let Some((command, _)) = cx.take_command::<ScreenModeCommand>() {
                mode = command.apply(mode);
            }
            if mode != before {
                let name: Arc<str> = words.screen_mode.as_str().into();
                let event = EditEvent::set(GestureId::default(), Mixed::Same(before), mode, name, None);
                let _ = screen_mode.reborrow().write(cx, &event);
                state.reveal.clear();
                if mode.is_full_screen() != before.is_full_screen() {
                    state.plan.start(mode.is_full_screen());
                }
            }
            if state.plan.is_active() {
                let host = WindowHost::of(&ctx);
                if let Some(facts) = host.facts(viewport) {
                    let keep_apart = os_of(&ctx) == Os::Windows;
                    if let Some(action) = state.plan.step(keep_apart, facts.state) {
                        host.act(viewport, action);
                    }
                }
                request_repaint(&ctx);
            }

            let host_only = mode.body_only();
            state.reveal.apply(chrome_toggles, viewport, pass::frame_nr(&ctx, viewport), host_only, allow_reveal);
            let only = state.reveal.only(host_only);
            let pointer = pointer::frame(&ctx);
            let frozen = !pointer.buttons.is_empty() || any_open(&ctx, OpenFilter::Keys);
            state.reveal.track(
                Tracking::new(only, allow_reveal, edge_reveal),
                Aim::new(pointer.pos, frozen, cx.rect, pointer.target, cx.env.direction().is_rtl()),
            );

            let visible: Vec<bool> = slots.iter().map(|placed| !bare && state.reveal.shows(&placed.slot, placed.edge, only, allow_reveal)).collect();
            let mut earlier = ShellRegions::new(state.body.unwrap_or(cx.rect));
            for (placed, shown) in slots.iter().zip(&visible) {
                if *shown {
                    let rect = state.slots.iter().find(|(key, _)| *key == placed.slot.key()).map_or(Rect::ZERO, |(_, rect)| *rect);
                    earlier = earlier.with_slot(placed.slot.key(), rect);
                }
            }
            let tabs = mode.tabs_visible() && document_tabs;
            let mut shell =
                ShellCx::new(window.clone(), if only { crate::runtime::env::BodyMode::Only } else { crate::runtime::env::BodyMode::Normal }, earlier)
                    .with_tabs(tabs)
                    .with_tab_reveal(state.reveal.tab());
            for edge in [RevealEdge::Top, RevealEdge::Start, RevealEdge::End] {
                shell = shell.with_edge_reveal(edge, state.reveal.edge(edge));
            }
            let backdrop = if only { SurfaceKind::Scrim } else { SurfaceKind::Canvas };
            let panels_hidden = state.reveal.panels_hidden();

            let laid = ShellRegion::scope(shell, PanelsHidden::new(panels_hidden)).id_salt("shell").show(ui, |ui, _| {
                let mut placed_rects: Vec<(SlotKey, PaneEdge, Rect)> = Vec::new();
                for (placed, shown) in slots.iter_mut().zip(&visible) {
                    if !*shown {
                        continue;
                    }
                    let Placed { slot, edge, add } = placed;
                    let mut region = ShellRegion::pane(ShellPane::new(slot.key()).edge(*edge)).id_salt(("slot", slot.key()));
                    if let Some(label) = slot.name() {
                        region = region.label(label.clone());
                    }
                    let out = region.show(ui, |ui, bcx| add(ui, bcx));
                    placed_rects.push((slot.key(), *edge, out.response.rect));
                }
                let center = ShellRegion::pane(ShellPane::new("body")).surface(backdrop).label(body_label.clone());
                let out = center.show(ui, |ui, bcx| {
                    if let Some(RegionBody(body)) = body.as_mut() {
                        body(ui, bcx);
                    }
                });
                (placed_rects, out.response.rect)
            });
            let (placed_rects, body_rect) = laid.inner.unwrap_or_else(|| (Vec::new(), cx.rect));

            ToastStack::show(ui);

            let mut regions = ShellRegions::new(body_rect);
            let mut edges = Vec::new();
            for (key, edge, rect) in &placed_rects {
                regions = regions.with_slot(*key, *rect);
                if let Some(edge) = edge.reveal_edge() {
                    edges.push((edge, *rect));
                }
            }
            state.reveal.record(edges);
            state.body = Some(body_rect);
            state.slots = placed_rects.iter().map(|(key, _, rect)| (*key, *rect)).collect();
            let revealed = only && !placed_rects.is_empty();
            *cx.keep::<ShellState>() = state;
            Parts { regions, revealed, panels_hidden, mode }
        });
        let response = shown.output.response;
        match shown.inner {
            Some(parts) => AppShellOutput {
                response,
                regions: parts.regions,
                revealed: parts.revealed,
                panels_hidden: parts.panels_hidden,
                screen_mode: parts.mode,
            },
            None => AppShellOutput {
                response,
                regions: ShellRegions::new(Rect::ZERO),
                revealed: false,
                panels_hidden: false,
                screen_mode: ScreenMode::default(),
            },
        }
    }
}
