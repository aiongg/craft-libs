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

mod ids;
mod slot;
#[cfg(test)]
mod tests;
mod visibility;

use egui::{Context, Frame, Id, Panel, Rect, Ui, UiBuilder, accesskit::Role};

pub use ids::WindowKey;
pub use slot::{Slot, SlotKey};
pub use visibility::{BodyMode, tab_toggles_chrome, take_tab_toggle};

use ids::State;
use slot::Edge;

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

    /// Registers the id of the body's canvas widget, so [`tab_toggles_chrome`] counts focus on
    /// it as "no control has focus".
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
    tabs: bool,
    backdrop: Option<egui::Color32>,
    body_label: Option<String>,
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
            tabs: true,
            backdrop: None,
            body_label: None,
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

    /// The title slot, the first top slot. `slot` should be keyed `"title"`.
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

    fn show_dyn<R>(self, ui: &mut Ui, body: BodyFn<'a, R>) -> ShellOutput<R> {
        let ctx = ui.ctx().clone();
        let window = self.window.clone();
        if ids::shown_before(&ctx, &window) {
            let message = format!("craft-ui: two AppShells with the window key {window:?} in one pass; give each window its own WindowKey");
            debug_assert!(false, "{message}");
            log::error!("{message}");
        }
        let tokens = Tokens::of(ui);
        let shell_tokens = tokens.component.app_shell;
        let rect = ui.max_rect();
        let state = visibility::begin_pass(&ctx, &window, self.mode, self.allow_reveal, rect, shell_tokens.edge_reveal);
        // Defaults from the token sizes, by edge.
        let default_size = |slot: &Slot, edge: Edge| match edge {
            Edge::Top if slot.key == "title" => tokens.size.bar.title,
            Edge::Top => tokens.size.bar.options,
            Edge::Bottom => tokens.size.bar.status,
            Edge::Start | Edge::End => tokens.size.rail.width,
        };

        let Self { bare, title, top, bottom, start, end, mode, allow_reveal, tabs, backdrop, body_label, .. } = self;
        debug_assert!(
            !bare || (title.is_none() && top.is_empty() && bottom.is_empty() && start.is_empty() && end.is_empty()),
            "craft-ui: slots added to a bare AppShell"
        );
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
                let panel = match edge {
                    Edge::Top => Panel::top(ids::slot_salt(&window, slot.key)),
                    Edge::Bottom => Panel::bottom(ids::slot_salt(&window, slot.key)),
                    Edge::Start => direction::start_panel(&ctx, ids::slot_salt(&window, slot.key)),
                    Edge::End => direction::end_panel(&ctx, ids::slot_salt(&window, slot.key)),
                };
                let mut builder = UiBuilder::new().accessibility_role(Role::Region);
                if let Some(label) = &slot.label {
                    builder = builder.accessibility_label(label.as_str());
                }
                let r = panel.resizable(false).show_separator_line(false).frame(slot.frame.unwrap_or(Frame::NONE)).exact_size(size).show(ui, |ui| {
                    ui.scope_builder(builder, add);
                });
                shown.push(slot.key);
                rects.push((slot.key, r.response.rect));
                placed.push((edge, r.response.rect));
            }
        }

        let backdrop = match mode {
            BodyMode::Only => shell_tokens.body_only_backdrop.to_egui(),
            BodyMode::Normal => backdrop.unwrap_or_else(|| shell_tokens.backdrop.to_egui()),
        };
        let revealed = mode == BodyMode::Only && !shown.is_empty();
        let cx = ShellCx { window: window.clone(), mode, revealed, tab_reveal: state.tab, tabs, shown, ctx: ctx.clone() };
        let mut builder = UiBuilder::new().accessibility_role(Role::Main);
        if let Some(label) = &body_label {
            builder = builder.accessibility_label(label.as_str());
        }
        let central = egui::CentralPanel::default().frame(Frame::NONE.fill(backdrop)).show(ui, |ui| {
            let rect = ui.max_rect();
            (rect, ui.scope_builder(builder, |ui| body(ui, &cx)).inner)
        });
        let (body_rect, inner) = central.inner;

        ids::set_state(&ctx, &window, State { shown: placed, ..state });
        ShellOutput { inner, body: body_rect, revealed, slots: rects }
    }
}
