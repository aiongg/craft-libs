//! The window skeleton: an ordered stack of top bars starting with the title bar, a start-side
//! rail, the body, an end-side rail and an ordered stack of bottom bars.
//!
//! The shell decides which slots show (per-slot flags from the host, a body-only mode, a
//! temporary reveal in body-only mode), gives every id and every piece of state a window scope,
//! and paints the body's backdrop. It draws no bar, rail or panel: slots are closures.
//!
//! ```no_run
//! use craft_ui::shell::{AppShell, BodyMode, Slot, WindowKey};
//! # fn demo(ui: &mut egui::Ui, hides_chrome: bool) {
//! let out = AppShell::new(WindowKey::main())
//!     .title(Slot::new("title").height(32.0).reveal(), |ui| { ui.label("Title"); })
//!     .top(Slot::new("options").height(32.0).reveal().label("Options"), |ui| { ui.label("Options"); })
//!     .bottom(Slot::new("status").height(22.0).label("Status"), |ui| { ui.label("Status"); })
//!     .start(Slot::new("tools").width(40.0).reveal().label("Tools"), |ui| { ui.label("Tools"); })
//!     .body_mode(if hides_chrome { BodyMode::Only } else { BodyMode::Normal })
//!     .show(ui, |ui, _cx| { ui.label("Document"); });
//! # let _ = out;
//! # }
//! ```
//!
//! Order. Slots claim space in this order, which is also the order they are drawn and the Tab
//! order: the title, then the `top` slots in call order from the window's top edge inward; the
//! `bottom` slots in call order from the bottom edge inward; then `start`, then `end`; then the
//! body in what is left. A theme or size never changes it.

mod canvas;
#[cfg(test)]
mod canvas_tests;
mod ids;
mod screen_mode;
#[cfg(test)]
mod screen_mode_tests;
mod slot;
#[cfg(test)]
mod tests;
mod visibility;
mod window;
#[cfg(all(test, feature = "window-frame"))]
mod window_tests;

use egui::{Context, Frame, Id, Panel, Rect, Ui, UiBuilder, accesskit::Role};

pub use canvas::{CanvasLabels, CanvasOutput, CanvasShell, PanelOptions, PanelRail};
pub use ids::WindowKey;
pub use screen_mode::{ScreenMode, full_screen_commands};
pub use slot::{Edge, Slot, SlotKey};
pub use visibility::{BodyMode, tab_toggles_chrome, take_tab_toggle};

use ids::State;

use crate::direction;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

type SlotFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;
type BodyFn<'a, R> = Box<dyn FnOnce(&mut Ui, &ShellCx) -> R + 'a>;

/// What the body (and the host, through the slot closures' parent) can ask of the shell.
#[derive(Clone, Debug)]
pub struct ShellCx {
    window: WindowKey,
    mode: BodyMode,
    revealed: bool,
    tab_reveal: bool,
    edges: [bool; 3],
    tabs: bool,
    shown: Vec<SlotKey>,
    ctx: Context,
}

impl ShellCx {
    /// The window's key; components in slots key their ids and memory by it.
    pub fn window(&self) -> &WindowKey {
        &self.window
    }

    pub fn body_mode(&self) -> BodyMode {
        self.mode
    }

    /// Is the slot with this key shown on this pass?
    pub fn slot_visible(&self, key: SlotKey) -> bool {
        self.shown.contains(&key)
    }

    /// Whether the host's floating panels show: always in normal mode; in body-only mode only
    /// while the Tab reveal is on.
    pub fn floating_visible(&self) -> bool {
        self.mode == BodyMode::Normal || self.tab_reveal
    }

    /// Whether the document tabs show: the host's flag in normal mode, never in body-only mode.
    pub fn tabs_visible(&self) -> bool {
        self.mode == BodyMode::Normal && self.tabs
    }

    /// Are slots shown by a reveal (Tab or edge) on this pass?
    pub fn revealed(&self) -> bool {
        self.revealed
    }

    /// Is the edge reveal on for `edge`? Only the top, start and end edges reveal. False in
    /// normal mode and during the Tab reveal.
    pub fn edge_revealed(&self, edge: Edge) -> bool {
        visibility::index(edge).is_some_and(|i| self.edges[i])
    }

    /// Whether the host's dock and panel rail show: in normal mode, during the Tab reveal and
    /// during the end-edge reveal.
    pub fn dock_visible(&self) -> bool {
        self.mode == BodyMode::Normal || self.tab_reveal || self.edge_revealed(Edge::End)
    }

    /// Registers the id of the body's canvas widget, so [`tab_toggles_chrome`] counts focus on
    /// it as "no control has focus". Call it on every pass the body has a canvas; a pass
    /// without the call (a start screen, a document switch) registers none.
    pub fn canvas(&self, id: Id) {
        ids::set_canvas(&self.ctx, id);
    }
}

/// What [`AppShell::show`] drew.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ShellOutput<R = ()> {
    /// What the body closure returned.
    pub inner: R,
    /// The body's rect.
    pub body: Rect,
    /// Slots shown by a reveal on this pass (Tab or edge).
    pub revealed: bool,
    /// The rects of the shown slots, in the order drawn.
    pub slots: Vec<(SlotKey, Rect)>,
}

/// The window skeleton. See the [module](self).
#[must_use = "call `show`"]
pub struct AppShell<'a> {
    window: WindowKey,
    bare: bool,
    title: Option<(Slot, SlotFn<'a>)>,
    top: Vec<(Slot, SlotFn<'a>)>,
    bottom: Vec<(Slot, SlotFn<'a>)>,
    start: Vec<(Slot, SlotFn<'a>)>,
    end: Vec<(Slot, SlotFn<'a>)>,
    mode: BodyMode,
    allow_reveal: bool,
    edge_reveal: bool,
    tabs: bool,
    backdrop: Option<egui::Color32>,
    body_label: Option<String>,
    level: window::Level<'a>,
}

impl<'a> AppShell<'a> {
    pub fn new(window: WindowKey) -> Self {
        Self {
            window,
            bare: false,
            title: None,
            top: Vec::new(),
            bottom: Vec::new(),
            start: Vec::new(),
            end: Vec::new(),
            mode: BodyMode::Normal,
            allow_reveal: true,
            edge_reveal: true,
            tabs: true,
            backdrop: None,
            body_label: None,
            level: window::Level::default(),
        }
    }

    /// A window with no chrome (a torn-off document window): only a body, with the same window
    /// scope and duplicate-key check. Slots added to it are a bug.
    pub fn bare(window: WindowKey) -> Self {
        Self { bare: true, ..Self::new(window) }
    }

    /// Whether this is a [bare](Self::bare) shell.
    pub fn is_bare(&self) -> bool {
        self.bare
    }

    /// The title slot, the first top slot, with plain contents. `slot` should be keyed `"title"`.
    /// For the window's title bar use [`title_bar`](Self::title_bar) (feature `window-frame`).
    pub fn title(mut self, slot: Slot, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.title = Some((slot, Box::new(add)));
        self
    }

    /// A top slot, below the ones added before it.
    pub fn top(mut self, slot: Slot, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.top.push((slot, Box::new(add)));
        self
    }

    /// A bottom slot, above the ones added before it.
    pub fn bottom(mut self, slot: Slot, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.bottom.push((slot, Box::new(add)));
        self
    }

    /// A slot at the reading start, inside the ones added before it.
    pub fn start(mut self, slot: Slot, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.start.push((slot, Box::new(add)));
        self
    }

    /// A slot at the reading end, inside the ones added before it.
    pub fn end(mut self, slot: Slot, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.end.push((slot, Box::new(add)));
        self
    }

    pub fn body_mode(mut self, mode: BodyMode) -> Self {
        self.mode = mode;
        self
    }

    /// Whether [`toggle_reveal`](Self::toggle_reveal) and the edge reveal work in body-only
    /// mode. Off for a presentation. Default on.
    pub fn allow_reveal(mut self, allow: bool) -> Self {
        self.allow_reveal = allow;
        self
    }

    /// Whether the pointer at the top, start or end edge reveals the hidden bars in body-only
    /// mode. Serves Preferences > Workspace > "Auto-Show Hidden Panels". Default on. Off, the
    /// Tab reveal ([`toggle_reveal`](Self::toggle_reveal)) still works.
    pub fn edge_reveal(mut self, on: bool) -> Self {
        self.edge_reveal = on;
        self
    }

    /// The host's flag for the document tabs, read back through [`ShellCx::tabs_visible`].
    pub fn document_tabs(mut self, visible: bool) -> Self {
        self.tabs = visible;
        self
    }

    /// The host's pasteboard colour, behind the document in normal mode.
    pub fn backdrop(mut self, color: Option<egui::Color32>) -> Self {
        self.backdrop = color;
        self
    }

    /// The translated AccessKit name of the body.
    pub fn body_label(mut self, label: impl Into<String>) -> Self {
        self.body_label = Some(label.into());
        self
    }

    /// Turns the Tab reveal of `window` on or off: the host's `window.togglePanels` command
    /// calls it in body-only mode. Ignored unless the window's last pass was in body-only mode
    /// with [`allow_reveal`](Self::allow_reveal). Returns whether the reveal is now on.
    pub fn toggle_reveal(ctx: &Context, window: &WindowKey) -> bool {
        visibility::toggle(ctx, window)
    }

    /// Ends the Tab and edge reveal of `window`; the host calls it when it changes the screen mode.
    pub fn clear_reveal(ctx: &Context, window: &WindowKey) {
        visibility::clear(ctx, window);
    }

    /// Is the Tab reveal of `window` on?
    pub fn reveal_active(ctx: &Context, window: &WindowKey) -> bool {
        visibility::active(ctx, window)
    }

    /// Lays out the slots, then the body in what is left.
    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui, &ShellCx) -> R + 'a) -> ShellOutput<R> {
        self.show_dyn(ui, Box::new(body))
    }

    #[cfg_attr(not(feature = "window-frame"), allow(unused_mut))]
    fn show_dyn<R>(mut self, ui: &mut Ui, body: BodyFn<'a, R>) -> ShellOutput<R> {
        let ctx = ui.ctx().clone();
        // An app-modal window of any viewport refuses the close of this one.
        crate::viewport::ViewportWindow::refuse_close(&ctx);
        #[cfg(feature = "window-restore")]
        window::track(&mut self, &ctx);
        #[cfg(feature = "window-frame")]
        let bar_drawn = window::bar_slot(&mut self);
        let window = self.window.clone();
        if ids::shown_before(&ctx, &window) {
            let message = format!("craft-ui: two AppShells with the window key {window:?} in one pass; give each window its own WindowKey");
            debug_assert!(false, "{message}");
            log::error!("{message}");
        }
        let tokens = Tokens::of(ui);
        let shell_tokens = tokens.component.app_shell;
        let rect = ui.max_rect();
        #[cfg_attr(not(feature = "window-frame"), allow(unused_mut))]
        let mut state = visibility::begin_pass(&ctx, &window, self.mode, self.allow_reveal, self.edge_reveal, rect, shell_tokens.edge_reveal);
        // Defaults from the token sizes, by edge.
        let default_size = |slot: &Slot, edge: Edge| match edge {
            Edge::Top if slot.key == "title" => tokens.size.bar.title,
            Edge::Top if slot.narrow => tokens.size.bar.options_narrow,
            Edge::Top => tokens.size.bar.options,
            Edge::Bottom => tokens.size.bar.status,
            // Each added column is a button and PhotoCraft's 2 pt gap (`panels/toolbar.rs:89`).
            Edge::Start | Edge::End => tokens.size.rail.width + f32::from(slot.columns - 1) * (tokens.size.rail.button + 2.0),
        };

        let Self { bare, title, top, bottom, start, end, mode, allow_reveal, tabs, backdrop, body_label, level, .. } = self;
        debug_assert!(
            !bare || (title.is_none() && top.is_empty() && bottom.is_empty() && start.is_empty() && end.is_empty()),
            "craft-ui: slots added to a bare AppShell"
        );
        // Slots shown by the edge reveal alone overlay the body; the Tab reveal pushes the layout.
        let overlay = mode == BodyMode::Only && !state.tab;
        let mut free = rect;
        let mut seen: Vec<SlotKey> = Vec::new();
        let mut shown: Vec<SlotKey> = Vec::new();
        let mut rects: Vec<(SlotKey, Rect)> = Vec::new();
        let mut placed: Vec<(Edge, Rect)> = Vec::new();
        let groups = [(Edge::Top, title.into_iter().chain(top).collect::<Vec<_>>()), (Edge::Bottom, bottom), (Edge::Start, start), (Edge::End, end)];
        for (edge, slots) in groups {
            for (slot, add) in slots {
                if seen.contains(&slot.key) {
                    let message = format!("craft-ui: two slots with the key {:?} in the window {window:?}", slot.key);
                    debug_assert!(false, "{message}");
                    log::error!("{message}");
                }
                seen.push(slot.key);
                if !visibility::shows(&slot, edge, mode, allow_reveal, &state) {
                    continue;
                }
                let size = slot.size.unwrap_or_else(|| default_size(&slot, edge));
                let mut builder = UiBuilder::new().accessibility_role(Role::Region);
                if let Some(label) = &slot.label {
                    builder = builder.accessibility_label(label.as_str());
                }
                let frame = slot.frame.unwrap_or(Frame::NONE);
                let salt = ids::slot_salt(&window, slot.key);
                let r = if overlay {
                    // An edge reveal floats over the body, which does not move.
                    let rect = overlay_rect(&ctx, &mut free, edge, size);
                    let inner = rect.size() - frame.total_margin().sum();
                    let area = egui::Area::new(salt).order(egui::Order::Middle).fixed_pos(rect.min).constrain(false).show(&ctx, |ui| {
                        frame.show(ui, |ui| {
                            ui.set_min_size(inner);
                            ui.set_max_size(inner);
                            ui.scope_builder(builder, add);
                        });
                    });
                    area.response.rect
                } else {
                    let panel = match edge {
                        Edge::Top => Panel::top(salt),
                        Edge::Bottom => Panel::bottom(salt),
                        Edge::Start => direction::start_panel(&ctx, salt),
                        Edge::End => direction::end_panel(&ctx, salt),
                    };
                    panel
                        .resizable(false)
                        .show_separator_line(false)
                        .frame(frame)
                        .exact_size(size)
                        .show(ui, |ui| {
                            ui.scope_builder(builder, add);
                        })
                        .response
                        .rect
                };
                shown.push(slot.key);
                rects.push((slot.key, r));
                placed.push((edge, r));
            }
        }

        let backdrop = match mode {
            BodyMode::Only => shell_tokens.body_only_backdrop.to_egui(),
            BodyMode::Normal => backdrop.unwrap_or_else(|| shell_tokens.backdrop.to_egui()),
        };
        let revealed = mode == BodyMode::Only && !shown.is_empty();
        let cx = ShellCx { window: window.clone(), mode, revealed, tab_reveal: state.tab, edges: state.edges, tabs, shown, ctx: ctx.clone() };
        let mut builder = UiBuilder::new().accessibility_role(Role::Main);
        if let Some(label) = &body_label {
            builder = builder.accessibility_label(label.as_str());
        }
        let central = egui::CentralPanel::default().frame(Frame::NONE.fill(backdrop)).show(ui, |ui| {
            let rect = ui.max_rect();
            (rect, ui.scope_builder(builder, |ui| body(ui, &cx)).inner)
        });
        let (body_rect, inner) = central.inner;

        #[cfg(feature = "window-frame")]
        window::finish_pass(&level, ui, bar_drawn.is_some_and(|d| d.get()), &mut state);
        #[cfg(not(feature = "window-frame"))]
        let _ = level;
        ids::set_state(&ctx, &window, State { shown: placed, ..state });
        ShellOutput { inner, body: body_rect, revealed, slots: rects }
    }
}

/// The rect of an overlaid slot of `size` against `edge` of `free`, which then shrinks by it.
fn overlay_rect(ctx: &Context, free: &mut Rect, edge: Edge, size: f32) -> Rect {
    let whole = *free;
    match edge {
        Edge::Top => {
            free.min.y = (whole.min.y + size).min(whole.max.y);
            Rect::from_min_max(whole.min, egui::pos2(whole.max.x, free.min.y))
        }
        Edge::Bottom => {
            free.max.y = (whole.max.y - size).max(whole.min.y);
            Rect::from_min_max(egui::pos2(whole.min.x, free.max.y), whole.max)
        }
        Edge::Start | Edge::End => {
            let at_left = (edge == Edge::Start) != direction::is_rtl(ctx);
            if at_left {
                free.min.x = (whole.min.x + size).min(whole.max.x);
                Rect::from_min_max(whole.min, egui::pos2(free.min.x, whole.max.y))
            } else {
                free.max.x = (whole.max.x - size).max(whole.min.x);
                Rect::from_min_max(egui::pos2(free.max.x, whole.min.y), whole.max)
            }
        }
    }
}
