//! [`RegionFocus`]: F6 and Shift+F6 move the keyboard between the window's regions (the tools,
//! the options bar, the document, the panels), as Windows apps and Photoshop's panels do.

use egui::accesskit::{Live, Role};
use egui::{Context, Id, LayerId, Order, Rect, Ui};

use super::key_owner::OwnerGate;
use crate::primitives::{Corner, FocusRing};
use crate::theme::{Tokens, TokensOf};

/// A region of the window F6 visits. The host registers the regions it shows each frame, in F6
/// order; a region it doesn't register this frame is skipped.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Region {
    /// The automation id (`tools`, `options`, `document`, `panels`, or the app's own).
    pub id: String,
    /// The accessible name announced when the region takes the keyboard ("Tools").
    pub label: String,
}

impl Region {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Region {
        Region { id: id.into(), label: label.into() }
    }
}

/// A region as registered: where it is and which widget takes the keyboard first.
#[derive(Clone, Debug)]
struct Entry {
    region: Region,
    rect: Rect,
    entry: Option<Id>,
}

/// One window's region focus, in egui temp data.
#[derive(Clone, Debug, Default)]
struct State {
    /// The pass `this` is being filled in; `previous` is the pass before it.
    pass: u64,
    this: Vec<Entry>,
    previous: Vec<Entry>,
    /// The pass pointer presses were last read in.
    synced: Option<u64>,
    current: Option<String>,
    /// The keyboard moved to `current` with F6 and no press has come since.
    ring: bool,
}

impl State {
    /// The regions of the last complete pass: `this` until the first registration of a new pass
    /// moves it to `previous`.
    fn known(&self, pass: u64) -> &[Entry] {
        if self.pass == pass { &self.previous } else { &self.this }
    }

    fn roll(&mut self, pass: u64) {
        if self.pass != pass {
            self.previous = std::mem::take(&mut self.this);
            self.pass = pass;
        }
    }
}

/// The commands F6 and Shift+F6 are bound to in the app's keymap (a user may rebind them).
// figcraft: crates/ui-egui/src/a11y.rs:29-30.
pub const NEXT_REGION_COMMAND: &str = "view.nextRegion";
pub const PREVIOUS_REGION_COMMAND: &str = "view.previousRegion";

/// Region focus for one window, kept in egui memory by window.
///
/// ```no_run
/// use craft_ui::focus::{Region, RegionFocus};
/// # fn demo(ui: &mut egui::Ui, tools_rect: egui::Rect, canvas_id: egui::Id) {
/// let focus = RegionFocus::new(ui.ctx());
/// focus.region(ui.ctx(), Region::new("tools", "Tools"), tools_rect, None);
/// focus.region(ui.ctx(), Region::new("document", "Document"), ui.max_rect(), Some(canvas_id));
/// // When the keymap runs view.nextRegion:
/// focus.step(ui.ctx(), 1);
/// # }
/// ```
///
/// - **Moving:** [`RegionFocus::step`] gives the next registered region the keyboard: its entry
///   widget (the one registered with it, else the region's first focusable widget) takes egui
///   focus. The order wraps. The document region's entry is the canvas, so tool keys work at once.
/// - **Ring:** the region that took the keyboard shows the focus ring around its rect until the
///   pointer is pressed anywhere (keyboard mode, as egui's own focus ring); a press in a region
///   makes it the current one without a ring.
/// - **Modal:** while a dialog, popup or menu is open, F6 is left to it (no step).
/// - **AccessKit:** the region's label is announced (a polite live region) when it takes the
///   keyboard.
#[derive(Clone, Copy, Debug)]
pub struct RegionFocus {
    id: Id,
}

impl RegionFocus {
    /// The region focus of the window `ctx` is drawing.
    pub fn new(ctx: &Context) -> RegionFocus {
        let focus = RegionFocus { id: Id::unique("craft-ui-region-focus").with(ctx.viewport_id()) };
        focus.read_presses(ctx);
        focus
    }

    /// Registers a region shown this frame, in F6 order, with its rect and its entry widget.
    // figcraft: crates/ui-egui/src/a11y.rs:44 (`Region::ALL`, F6 order), :580 `enter`.
    pub fn region(&self, ctx: &Context, region: Region, rect: Rect, entry: Option<Id>) {
        let pass = ctx.cumulative_pass_nr();
        self.with(ctx, |state| {
            state.roll(pass);
            state.this.retain(|e| e.region.id != region.id);
            state.this.push(Entry { region, rect, entry });
        });
    }

    /// Moves the keyboard `step` regions on (1 for F6, -1 for Shift+F6). Does nothing while a
    /// modal layer is open.
    // figcraft: crates/ui-egui/src/a11y.rs:131 `Nav::step`; wordcraft: crates/ui-egui/src/panes/host.rs:24 `route_f6`.
    pub fn step(&self, ctx: &Context, step: i32) {
        if super::key_owner::owner(ctx, &OwnerGate::default()).is_exclusive() {
            return;
        }
        let pass = ctx.cumulative_pass_nr();
        let target = self.with(ctx, |state| {
            let known = state.known(pass);
            let n = known.len() as i32;
            if n == 0 {
                return None;
            }
            let from = state.current.as_ref().and_then(|id| known.iter().position(|e| &e.region.id == id)).map(|i| i as i32);
            // Without a current region, forward starts at the first and backward at the last.
            let from = from.unwrap_or(if step >= 0 { -1 } else { n });
            Some(known[(from + step).rem_euclid(n) as usize].clone())
        });
        if let Some(target) = target {
            self.enter(ctx, &target);
        }
    }

    /// Gives `region` the keyboard (automation's `ui.set {focusRegion}`).
    pub fn go(&self, ctx: &Context, region: &str) {
        let pass = ctx.cumulative_pass_nr();
        let target = self.with(ctx, |state| state.known(pass).iter().find(|e| e.region.id == region).cloned());
        if let Some(target) = target {
            self.enter(ctx, &target);
        }
    }

    /// The region that has the keyboard, by id.
    pub fn current(&self, ctx: &Context) -> Option<String> {
        self.with(ctx, |state| state.current.clone())
    }

    /// Paints the ring around the current region when the keyboard moved there. Call it after
    /// the regions draw, on the window's top layer.
    // figcraft: crates/ui-egui/src/a11y.rs:284 `keyboard` (the ring shows only in keyboard mode).
    pub fn paint_ring(&self, ui: &Ui) {
        let ctx = ui.ctx();
        let Some((rect, label)) = self.ring(ctx) else { return };
        let style = FocusRing::style(&Tokens::of(ui));
        // Inside the rect: a region at the window's edge would lose half the ring outside it.
        let shape = FocusRing::shape(&Tokens::of(ui), Corner::Panel, rect.shrink(style.outset()));
        ctx.layer_painter(LayerId::new(Order::Foreground, self.id.with("ring"))).add(shape);
        ctx.accesskit_node_builder(self.id.with("announce"), |node| {
            node.set_role(Role::Status);
            node.set_live(Live::Polite);
            node.set_label(label);
        });
    }

    /// The rect and label of the region showing the ring.
    pub(super) fn ring(&self, ctx: &Context) -> Option<(Rect, String)> {
        self.with(ctx, |state| {
            let current = state.current.as_ref().filter(|_| state.ring)?;
            // After the regions drew, `this` holds all of this pass; before, the pass before.
            state.this.iter().find(|e| &e.region.id == current).map(|e| (e.rect, e.region.label.clone()))
        })
    }

    fn with<R>(&self, ctx: &Context, f: impl FnOnce(&mut State) -> R) -> R {
        ctx.data_mut(|d| f(d.get_temp_mut_or_default::<State>(self.id)))
    }

    /// A press makes the region it landed in current and hides the ring. Read once per pass.
    fn read_presses(&self, ctx: &Context) {
        let pass = ctx.cumulative_pass_nr();
        let press = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten());
        self.with(ctx, |state| {
            if state.synced == Some(pass) {
                return;
            }
            state.synced = Some(pass);
            let Some(pos) = press else { return };
            state.ring = false;
            // The smallest region under the pointer: a floating region sits over the document.
            let hit = state.known(pass).iter().filter(|e| e.rect.contains(pos)).min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()));
            if let Some(hit) = hit {
                state.current = Some(hit.region.id.clone());
            }
        });
    }

    /// Gives `target` the keyboard and shows the ring around it.
    fn enter(&self, ctx: &Context, target: &Entry) {
        let widget = target.entry.or_else(|| first_focusable(ctx, target.rect));
        ctx.memory_mut(|m| match widget {
            Some(id) => m.request_focus(id),
            None => {
                if let Some(focused) = m.focused() {
                    m.surrender_focus(focused);
                }
            }
        });
        self.with(ctx, |state| {
            state.current = Some(target.region.id.clone());
            state.ring = true;
        });
        ctx.request_repaint();
    }
}

/// The first focusable widget the last pass drew inside `rect`, in painting order (egui's Tab
/// order).
// figcraft: crates/ui-egui/src/a11y.rs:647 `focusables`.
fn first_focusable(ctx: &Context, rect: Rect) -> Option<Id> {
    ctx.viewport(|v| {
        let mut layers: Vec<_> = v.prev_pass.widgets.layers().filter(|(layer, _)| !matches!(layer.order, Order::Tooltip | Order::Debug)).collect();
        layers.sort_by_key(|(layer, _)| layer.order);
        layers
            .into_iter()
            .flat_map(|(_, widgets)| widgets)
            .find(|w| w.enabled && w.sense.is_focusable() && w.rect.is_positive() && w.rect.is_finite() && rect.contains(w.rect.center()))
            .map(|w| w.id)
    })
}
