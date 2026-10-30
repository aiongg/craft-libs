//! [`TabStrip`]: a strip of document or panel tabs in the host's order, fitted to its width with
//! an overflow menu, one Tab stop, host areas at both ends, and the events the host applies.

use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::Hash;

use egui::accesskit::{Action, Orientation, Role};
use egui::os::OperatingSystem;
use egui::{
    AsIdSalt, Context, CornerRadius, CursorIcon, DragAndDrop, Event, EventFilter, Id, IdSalt, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2,
    Rect, Response, Sense, Ui, UiBuilder, pos2, vec2,
};

use super::layout::{
    DragZone, FocusMove, FocusSpot, OverflowMenu, StripPlan, StripPlanInput, TabKind, drag_zone, focus_move, focus_spot, reorder_key, step_focus,
};
use super::tab::{Tab, TabStripText, TabStyle, TabView};
use crate::behavior::{Axis, DragEnd, DropPosition, ListDrag, Move, Reorder, ReorderGesture, RowInfo, StackKey};
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::{self, Icon, IconSize};
use crate::localized::Labels as _;
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::popup_list;
use crate::primitives::{FocusRing, Interactive};
use crate::theme::{Tokens, TokensOf as _};

/// One notch of a mouse wheel in points of smooth scrolling.
const WHEEL_NOTCH: f32 = 40.0;

/// What a [`TabStrip`] reports, in the order it happened. The strip changes nothing itself: the
/// host applies each event to its documents or panels and passes the result next frame.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TabEvent<K> {
    /// Show this tab: a click, Enter or Space on the focused tab, a row of the overflow menu, the
    /// wheel, a screen reader's click.
    Activate(K),
    /// Close this tab: a click on its ×, or a middle click on a closable tab where
    /// [`middle_click_closes`](TabStrip::middle_click_closes). The host runs its guarded close.
    Close(K),
    /// A double click on a tab, or on the empty strip (`None`).
    DoubleClick(Option<K>),
    /// The host's context menu is asked for.
    ContextMenu(TabMenuRequest<K>),
    /// A tab was moved: a drag of a tab as one edit gesture (`Begin`, then `Commit` with the move
    /// on a drop, or `Cancel`), or a reorder key's `Set`. The move is relative to a shown tab; the
    /// host applies it to its order. Where a drag would drop is
    /// [`TabStripOutput::drop_target`], not an edit.
    Reorder(EditEvent<Move<K>>),
    /// A tab drag was released outside the strip's band
    /// ([`tear_off`](TabStrip::tear_off)), with the pointer at `pointer`. The drag's payload
    /// ([`ListDrag`] with the strip's id) stays for the widget under the pointer; the host floats
    /// the tab or leaves it to docking.
    DroppedOut { tab: K, pointer: Pos2 },
}

/// A tab dragged out of the strip's band: the tab and where the pointer is.
#[derive(Clone, Debug, PartialEq)]
pub struct DragOut<K> {
    pub tab: K,
    pub pointer: Pos2,
}

/// A context menu asked for on a strip: a secondary click, a screen reader's Show Context Menu,
/// or the Menu key or Shift+F10 on the focused tab.
#[derive(Clone, Debug, PartialEq)]
pub struct TabMenuRequest<K> {
    /// The tab; `None` for the empty strip.
    pub tab: Option<K>,
    /// The tab's rect (the strip's without a tab), where a menu opened from the keyboard goes.
    pub anchor: Rect,
    /// Where the pointer was for a secondary click; `None` for a key.
    pub pointer: Option<Pos2>,
}

/// What [`TabStrip::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the strip's events"]
pub struct TabStripOutput<K> {
    /// The strip: its keyboard focus and its rect. A host's context menu belongs to it
    /// (`ContextMenu::show(ui, &output.response)`).
    pub response: Response,
    /// The empty strip between the host's areas, under the tabs: clicks and drags that land on
    /// no tab (a panel group's drag bar). Not focusable.
    pub background: Response,
    /// What happened this frame, in order.
    pub events: Vec<TabEvent<K>>,
    /// The shown tabs and their rects, in reading order.
    pub tabs: Vec<(K, Rect)>,
    /// Where releasing the dragged tab now would drop it; `None` outside a drag, outside the
    /// band, and where the drop would change nothing.
    pub drop_target: Option<Move<K>>,
    /// The dragged tab while the pointer is outside the band, every frame
    /// ([`tear_off`](TabStrip::tear_off)).
    pub dragging_out: Option<DragOut<K>>,
    plan: StripPlan,
    rtl: bool,
}

impl<K: Clone> TabStripOutput<K> {
    /// The index in the host's order a drop at `pos` opens at: before the first shown tab whose
    /// centre is past `pos` in reading order, else after the last shown tab.
    pub fn slot_at(&self, pos: Pos2) -> usize {
        self.plan.slot_at(self.rtl, pos)
    }

    /// The shown tab under `pos`.
    pub fn tab_at(&self, pos: Pos2) -> Option<K> {
        self.tabs.iter().find(|(_, rect)| rect.contains(pos)).map(|(key, _)| key.clone())
    }
}

/// Where the keyboard focus of a strip sits.
#[derive(Clone, Debug, PartialEq)]
enum Spot<K> {
    Tab(K),
    Overflow,
}

/// What a strip keeps between frames, in egui memory under its id.
#[derive(Clone)]
struct State<K> {
    /// Where the keyboard focus sits while the strip has it, and where it was when it left.
    spot: Option<Spot<K>>,
    /// The strip had egui's focus at the end of the last pass.
    focused: bool,
    /// The wheel turned since the last whole notch.
    wheel: f32,
    /// The tab being dragged.
    drag: Option<K>,
}

impl<K: Clone + Send + Sync + 'static> State<K> {
    fn load(ui: &Ui, id: Id) -> Self {
        ui.data(|d| d.get_temp(id)).unwrap_or(Self { spot: None, focused: false, wheel: 0.0, drag: None })
    }

    fn store(self, ui: &Ui, id: Id) {
        ui.data_mut(|d| d.insert_temp(id, self));
    }
}

/// A host's area at one end of the strip: its width and what it draws there.
type HostArea<'a> = (f32, Box<dyn FnOnce(&mut Ui) + 'a>);

/// A strip of tabs: documents or panels, in the host's order.
///
/// ```
/// use craft_ui::tab_strip::{Tab, TabEvent, TabKind, TabStrip};
/// # egui::__run_test_ui(|ui| {
/// let docs = [Tab::new(1, "Untitled-1").closable(true), Tab::new(2, "Photo.psd").detail("RGB/8").closable(true)];
/// let mut shown = 1;
/// let out = TabStrip::new("documents", "Documents", TabKind::Document).selected(&shown).show(ui, &docs);
/// for event in out.events {
///     match event {
///         TabEvent::Activate(key) => shown = key,
///         TabEvent::Close(_key) => { /* run the guarded close */ }
///         _ => {}
///     }
/// }
/// # });
/// ```
///
/// - **Fit.** [`TabKind`] picks the rules: document tabs keep whole titles and send the rest to
///   an overflow menu of every document, the active one checked; panel tabs shrink to
///   `tab.min_width` and then overflow, the menu listing the hidden ones. The selected tab is
///   always shown. The host's [`leading`](Self::leading) and [`trailing`](Self::trailing) areas
///   keep their widths.
/// - **Pointer.** A click activates a tab, a click on its × closes it (a press on the × is the
///   ×'s alone), a middle click closes a closable tab where middle click closes, a double click
///   reports the tab or the empty strip, a secondary click asks for the host's context menu, and
///   with [`wheel_selects`](Self::wheel_selects) each wheel notch activates the next or previous
///   tab. The overflow button is a [`PressMenu`]: press, drag onto a row and release.
/// - **Keyboard.** The strip is one Tab stop; the tabs, their × and the overflow button are not.
///   Focus arriving lands on the active tab. ← and → move the focus over the shown tabs and onto
///   the overflow button without switching (swapped right to left), Home and End go to the first
///   and last tab, Enter and Space activate the focused tab or open the overflow menu, and the
///   Menu key or Shift+F10 asks for the focused tab's context menu at its rect. The focused tab
///   draws the focus ring while the keyboard is in use.
/// - **AccessKit.** The strip is a horizontal `TabList` named by the host's label, with
///   `active_descendant` the focused tab or button while it has the focus. Each shown tab is a
///   `Tab` with `selected` and `position_in_set`; the `TabList` carries `size_of_set`, which counts the
///   hidden tabs (screen readers read it from the container). The tabs take the
///   actions Click (activate), Focus and Show Context Menu; its × is a `Button` inside it. The
///   overflow button is a `Button` named [`TabStripText::more`] with a menu popup. Hidden tabs
///   have no node: their row in the overflow menu reaches them.
/// - **Host drops.** [`insertion_marker`](Self::insertion_marker) draws the drop line at a slot
///   ([`TabStripOutput::slot_at`]) and [`drop_highlight`](Self::drop_highlight) outlines a tab;
///   the strip takes no foreign payload itself.
#[must_use = "show the strip with `show`"]
pub struct TabStrip<'a, K> {
    id_salt: IdSalt,
    label: String,
    kind: TabKind,
    selected: Option<K>,
    large: bool,
    reorderable: bool,
    middle_click_closes: bool,
    wheel_selects: bool,
    tear_off: bool,
    insertion_marker: Option<usize>,
    drop_highlight: Option<K>,
    leading: Option<HostArea<'a>>,
    trailing: Option<HostArea<'a>>,
    text: TabStripText,
    /// The host passed `text`; otherwise it is read from the installed words when shown.
    text_set: bool,
    controls: Option<Id>,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> TabStrip<'a, K> {
    /// A strip of `kind` named `label` for AccessKit ("Documents", a panel group's name);
    /// `id_salt` keeps its focus.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, kind: TabKind) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            kind,
            selected: None,
            large: false,
            reorderable: true,
            middle_click_closes: kind.middle_click_closes(),
            wheel_selects: false,
            tear_off: false,
            insertion_marker: None,
            drop_highlight: None,
            leading: None,
            trailing: None,
            text: TabStripText::default(),
            text_set: false,
            controls: None,
        }
    }

    /// The shown tab. The strip never changes it: the host sets it from
    /// [`TabEvent::Activate`].
    pub fn selected(mut self, key: &K) -> Self {
        self.selected = Some(key.clone());
        self
    }

    /// The host's Large Tabs preference: `tab.height_large` and the larger label.
    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// Whether a drag on a tab is the tab's (to move it); `true` by default. Without it (a
    /// locked workspace) tabs sense clicks only and a drag on a tab goes to the
    /// [`background`](TabStripOutput::background).
    pub fn reorderable(mut self, reorderable: bool) -> Self {
        self.reorderable = reorderable;
        self
    }

    /// Whether a middle click on a closable tab closes it; on for documents, off for panels.
    pub fn middle_click_closes(mut self, closes: bool) -> Self {
        self.middle_click_closes = closes;
        self
    }

    /// Whether the wheel over the tabs activates the next or previous tab, one per notch (or 40
    /// points of smooth scrolling), stopping at the ends; off by default. Not with Ctrl or Cmd
    /// held.
    pub fn wheel_selects(mut self, selects: bool) -> Self {
        self.wheel_selects = selects;
        self
    }

    /// Whether a tab can be dragged out of the strip, off by default. The strip's band is its rect
    /// grown by `tab.tear_off` on every side: inside it the pointer resolves drop targets (with
    /// tear-off off, only its position along the strip counts); outside it there is no target and
    /// no marker, [`dragging_out`](TabStripOutput::dragging_out) names the tab each frame, and a
    /// release reports [`TabEvent::DroppedOut`].
    pub fn tear_off(mut self, tear_off: bool) -> Self {
        self.tear_off = tear_off;
        self
    }

    /// A host drag's slot, an index in the host's order (files over a document strip, a tab
    /// from another strip): the drop line at that slot.
    pub fn insertion_marker(mut self, slot: Option<usize>) -> Self {
        self.insertion_marker = slot;
        self
    }

    /// A host drag over this tab (layers over a document): the drop outline inside it.
    pub fn drop_highlight(mut self, key: Option<K>) -> Self {
        self.drop_highlight = key;
        self
    }

    /// The host's area at the reading start, `width` wide: `add` draws it (a panel group's
    /// header).
    pub fn leading(mut self, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.leading = Some((width, Box::new(add)));
        self
    }

    /// The host's area at the reading end, `width` wide: `add` draws it (a panel menu button).
    pub fn trailing(mut self, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.trailing = Some((width, Box::new(add)));
        self
    }

    /// The strip's words, translated by the host.
    pub fn text(mut self, text: TabStripText) -> Self {
        self.text = text;
        self.text_set = true;
        self
    }

    /// The host's panel node the selected tab controls (AccessKit `controls`).
    pub fn controls(mut self, panel: Id) -> Self {
        self.controls = Some(panel);
        self
    }

    /// Draws the strip on the available width and reads its input.
    pub fn show(mut self, ui: &mut Ui, tabs: &[Tab<K>]) -> TabStripOutput<K> {
        if !self.text_set {
            self.text = TabStripText::of(ui);
        }
        let id = ui.make_persistent_id(self.id_salt);
        let rtl = direction::is_rtl(ui);
        let style = Tab::<K>::style(&Tokens::of(ui));
        let mut state = State::<K>::load(ui, id);
        let focused = ui.memory(|m| m.has_focus(id));
        let mut pass = Pass {
            id,
            rtl,
            tabs,
            style,
            focused,
            events: Vec::new(),
            spot_set: false,
            keep_focus: false,
            active_node: None,
            gesture: Some(ReorderGesture::load(ui, id)),
            drop_target: None,
            dragging_out: None,
            dropped_out: None,
        };
        let selected = self.selected.clone();
        if focused && !state.focused {
            pass.arrive(&mut state, selected.as_ref());
            state.focused = true;
        }
        let sense = if tabs.is_empty() { Sense::hover() } else { Sense::FOCUSABLE };
        let builder = UiBuilder::new().scope_id(id).sense(sense).accessibility_role(Role::TabList).accessibility_label(self.label.clone());
        let inner = ui.scope_builder(builder, |ui| pass.draw(ui, self, &mut state));
        let (response, (background, shown, plan)) = (inner.response, inner.inner);
        if pass.keep_focus {
            ui.memory_mut(|m| m.request_focus(id));
        }
        if let Some(event) = pass.gesture.take().and_then(ReorderGesture::finish) {
            pass.events.push(TabEvent::Reorder(event));
        }
        if let Some(DragOut { tab, pointer }) = pass.dropped_out.take() {
            pass.events.push(TabEvent::DroppedOut { tab, pointer });
        }
        let now = ui.memory(|m| m.has_focus(id));
        if now && !state.focused && !pass.spot_set {
            // Tab moved the focus here during this pass; the ring shows from the next.
            pass.arrive(&mut state, selected.as_ref());
        }
        let bounds = response.rect;
        let active = pass.active_node.filter(|_| now);
        let set_size = pass.tabs.len();
        ui.ctx().accesskit_node_builder(id, |node| {
            // The `Ui`'s node took its bounds before its rect was known.
            node.set_bounds(egui::accesskit::Rect {
                x0: bounds.min.x.into(),
                y0: bounds.min.y.into(),
                x1: bounds.max.x.into(),
                y1: bounds.max.y.into(),
            });
            node.set_orientation(Orientation::Horizontal);
            node.set_size_of_set(set_size);
            match active {
                Some(node_id) => node.set_active_descendant(node_id.accesskit_id()),
                None => node.clear_active_descendant(),
            }
        });
        if now {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, vertical_arrows: false, tab: false, escape: false });
            });
        }
        state.focused = now;
        state.store(ui, id);
        TabStripOutput {
            response,
            background,
            events: pass.events,
            tabs: shown,
            drop_target: pass.drop_target,
            dragging_out: pass.dragging_out,
            plan,
            rtl,
        }
    }
}

/// One pass of a strip.
struct Pass<'t, K> {
    id: Id,
    rtl: bool,
    tabs: &'t [Tab<K>],
    style: TabStyle,
    /// The strip had egui's focus when the pass began.
    focused: bool,
    events: Vec<TabEvent<K>>,
    /// The focus spot was set this pass by the pointer or a screen reader.
    spot_set: bool,
    /// The strip keeps (or takes) egui's focus at the end of the pass.
    keep_focus: bool,
    /// The node of the focused tab or the overflow button.
    active_node: Option<Id>,
    /// The drag's edit gesture; taken at the end of the pass.
    gesture: Option<ReorderGesture<K>>,
    drop_target: Option<Move<K>>,
    dragging_out: Option<DragOut<K>>,
    dropped_out: Option<DragOut<K>>,
}

impl<K: Clone + Eq + Hash + Debug + Send + Sync + 'static> Pass<'_, K> {
    /// The focus arrived: it lands on the active tab.
    fn arrive(&self, state: &mut State<K>, selected: Option<&K>) {
        state.spot = selected.cloned().map(Spot::Tab);
    }

    fn draw(&mut self, ui: &mut Ui, strip: TabStrip<'_, K>, state: &mut State<K>) -> (Response, Vec<(K, Rect)>, StripPlan) {
        let TabStrip {
            kind,
            selected,
            large,
            reorderable,
            middle_click_closes,
            wheel_selects,
            tear_off,
            insertion_marker,
            drop_highlight,
            leading,
            trailing,
            text,
            controls,
            ..
        } = strip;
        let tabs = self.tabs;
        let style = self.style;
        let height = if kind == TabKind::Panel { style.panel_height } else { style.strip_height(large) };
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), height));
        if !kind.text_only() {
            ui.painter().rect_filled(rect, 0.0, style.strip);
        }
        let shows_close = kind.shows_close();
        let natural: Vec<f32> = tabs.iter().map(|tab| tab.natural_width(ui, large, shows_close)).collect();
        let selected_at = selected.as_ref().and_then(|key| tabs.iter().position(|tab| tab.key() == key));
        let plan = StripPlan::new(&StripPlanInput {
            kind,
            rect,
            rtl: self.rtl,
            leading: leading.as_ref().map_or(0.0, |(width, _)| *width),
            trailing: trailing.as_ref().map_or(0.0, |(width, _)| *width),
            natural: &natural,
            selected: selected_at,
            min_width: style.min_width,
            overflow_width: style.overflow,
        });
        let order: Vec<RowInfo<K>> = plan.shown.iter().map(|placed| RowInfo::leaf(tabs[placed.index].key().clone())).collect();
        let host_order: Vec<RowInfo<K>> = tabs.iter().map(|tab| RowInfo::leaf(tab.key().clone())).collect();
        self.end_stale_drag(ui.ctx(), state);
        let background = ui.interact(plan.tabs_area, self.id.with("background"), Sense::CLICK | Sense::DRAG);

        let mut spot = self.spot(state, &plan, selected_at);
        let mut open_overflow = false;
        if self.focused && ui.is_enabled() && !popup_list::any_open(ui.ctx()) {
            let keys = self.keys(ui, reorderable);
            for mv in keys.moves {
                if let Some(to) = spot.and_then(|from| step_focus(from, mv, plan.shown.len(), plan.overflow.is_some())) {
                    spot = Some(to);
                }
            }
            match spot {
                Some(FocusSpot::Tab(at)) => {
                    let placed = plan.shown[at];
                    let key = tabs[placed.index].key().clone();
                    if keys.activate {
                        self.events.push(TabEvent::Activate(key.clone()));
                    }
                    if keys.menu {
                        self.events.push(TabEvent::ContextMenu(TabMenuRequest { tab: Some(key.clone()), anchor: placed.rect, pointer: None }));
                    }
                    if let Some(&(stack, modifiers)) = keys.reorders.first() {
                        let mv = self.reorder().key_move(host_order.as_slice(), std::slice::from_ref(&key), stack, modifiers);
                        if let (Some(mv), Some(gesture)) = (mv, self.gesture.as_mut().filter(|g| !g.is_active())) {
                            gesture.set(mv);
                        }
                    }
                }
                Some(FocusSpot::Overflow) => open_overflow = keys.activate,
                None => {}
            }
            state.spot = spot.map(|spot| self.spot_key(spot, &plan));
        }

        let sense = if reorderable { Sense::CLICK | Sense::DRAG } else { Sense::CLICK };
        let mut shown = Vec::with_capacity(plan.shown.len());
        for (at, placed) in plan.shown.iter().enumerate() {
            let source = &tabs[placed.index];
            let tab: Cow<'_, Tab<K>> =
                if shows_close || !source.is_closable() { Cow::Borrowed(source) } else { Cow::Owned(source.clone().closable(false)) };
            let key = source.key();
            let active = selected.as_ref() == Some(key);
            let has_spot = spot == Some(FocusSpot::Tab(at));
            let view = TabView::new(self.id, &text, placed.index)
                .active(active)
                .focused(self.focused && has_spot)
                .large(large)
                .divider(kind.compact_dividers())
                .text_only(kind.text_only())
                .drop_highlight(drop_highlight.as_ref() == Some(key))
                .sense(sense)
                .controls(controls.filter(|_| active));
            let out = tab.show_at(ui, placed.rect, &view);
            ui.ctx().accesskit_node_builder(out.tab.id, |node| {
                node.add_action(Action::Focus);
                node.add_action(Action::ShowContextMenu);
            });
            if has_spot {
                self.active_node = Some(out.tab.id);
            }
            let closes = middle_click_closes && source.is_closable();
            self.tab_pointer(ui, state, key, &out.tab, out.close.as_ref(), closes);
            if reorderable && state.drag.is_none() && out.tab.drag_started_by(PointerButton::Primary) {
                self.start_drag(ui.ctx(), state, order.as_slice(), key, source.label());
            }
            shown.push((key.clone(), placed.rect));
        }

        if let Some(at) = plan.overflow {
            self.overflow(
                ui,
                state,
                (at, kind.overflow_menu()),
                (&plan, selected.as_ref()),
                (&text, open_overflow, spot == Some(FocusSpot::Overflow)),
            );
        }
        self.drag_frame(ui, state, &plan, (rect, tear_off));
        if let Some(slot) = insertion_marker
            && let Some((tab, left)) = marker_at(&plan, self.rtl, slot)
        {
            paint_marker(ui, &style, tab, left);
        }
        for (area, host) in [(plan.leading, leading), (plan.trailing, trailing)] {
            if let Some((_, add)) = host {
                let mut child = ui.new_child(UiBuilder::new().max_rect(area).layout(direction::row(ui)));
                child.set_clip_rect(area.intersect(ui.clip_rect()));
                add(&mut child);
            }
        }
        self.background(&background, rect);
        if wheel_selects {
            self.wheel(ui, state, plan.tabs_area, selected_at);
        }
        (background, shown, plan)
    }

    /// The rules of a strip's drops and keys: tabs side by side in reading order.
    fn reorder(&self) -> Reorder<'static, K> {
        Reorder::new(Axis::Horizontal).right_to_left(self.rtl)
    }

    /// Whether egui's drag and drop holds a tab of this strip.
    fn has_payload(&self, ctx: &Context) -> bool {
        DragAndDrop::payload::<ListDrag<K>>(ctx).is_some_and(|payload| payload.list == self.id)
    }

    /// Ends a drag whose gesture Esc or another cancel ended, or whose tab left the strip.
    fn end_stale_drag(&mut self, ctx: &Context, state: &mut State<K>) {
        if !self.gesture.as_ref().is_some_and(ReorderGesture::is_active) && state.drag.take().is_some() && self.has_payload(ctx) {
            DragAndDrop::clear_payload(ctx);
        }
        if state.drag.take_if(|key| !self.tabs.iter().any(|tab| tab.key() == key)).is_some() {
            self.end_drag(ctx, DragEnd::Removed, Modifiers::NONE, true);
        }
    }

    /// Starts a drag of the tab `key`: the payload is that one tab.
    fn start_drag(&mut self, ctx: &Context, state: &mut State<K>, order: &[RowInfo<K>], key: &K, label: &str) {
        let payload = ListDrag::new(self.id, order, &[], key.clone(), label);
        let Some(gesture) = self.gesture.as_mut() else { return };
        gesture.begin();
        if !gesture.is_active() {
            return;
        }
        state.drag = Some(key.clone());
        DragAndDrop::set_payload(ctx, payload);
    }

    /// Reports the end of a drag. A drop on the strip or a cancel clears the payload so no other
    /// widget takes it; a release outside the band leaves it to the widget under the pointer.
    fn end_drag(&mut self, ctx: &Context, end: DragEnd<K>, modifiers: Modifiers, clear: bool) {
        if clear && self.has_payload(ctx) {
            DragAndDrop::clear_payload(ctx);
        }
        if let Some(gesture) = self.gesture.as_mut() {
            gesture.end(end, modifiers);
        }
    }

    /// The drag under way: ends it on a release, Esc or focus loss; otherwise resolves the target
    /// and draws its marker, or reports the tab leaving the band.
    fn drag_frame(&mut self, ui: &Ui, state: &mut State<K>, plan: &StripPlan, (strip, tear_off): (Rect, bool)) {
        let Some(dragged) = state.drag.take() else { return };
        let ctx = ui.ctx().clone();
        let (released, down, modifiers, pointer) =
            ui.input(|i| (i.pointer.any_released(), i.pointer.any_down(), i.modifiers, i.pointer.latest_pos()));
        let zone = pointer.map_or(DragZone::Band, |pos| drag_zone(strip, pos, tear_off, self.style.tear_off));
        let drop = pointer.filter(|_| zone == DragZone::Band).and_then(|pos| {
            let slot = plan.slot_at(self.rtl, pos);
            self.slot_move(plan, &dragged, slot, modifiers).map(|mv| (slot, mv))
        });
        let target = drop.as_ref().map(|(_, mv)| mv.clone());
        if crate::focus::lost(&ctx) {
            self.end_drag(&ctx, DragEnd::FocusLost, modifiers, true);
        } else if released || !down {
            match (zone, pointer) {
                (DragZone::Outside, Some(pointer)) => {
                    self.end_drag(&ctx, DragEnd::Released(None), modifiers, false);
                    self.dropped_out = Some(DragOut { tab: dragged, pointer });
                }
                _ => self.end_drag(&ctx, DragEnd::Released(target), modifiers, true),
            }
        } else if !self.has_payload(&ctx) {
            // egui's drag and drop takes Esc and drops the payload before the gesture sees the key.
            let end = if DragAndDrop::aborted_by_escape(&ctx) { DragEnd::Escape } else { DragEnd::PayloadCleared };
            self.end_drag(&ctx, end, modifiers, true);
        } else {
            ctx.set_cursor_icon(CursorIcon::Grabbing);
            match (zone, pointer) {
                (DragZone::Outside, Some(pointer)) => self.dragging_out = Some(DragOut { tab: dragged.clone(), pointer }),
                _ => {
                    if let Some((slot, _)) = &drop
                        && let Some((rect, left)) = marker_at(plan, self.rtl, *slot)
                    {
                        paint_marker(ui, &self.style, rect, left);
                    }
                    self.drop_target = target;
                }
            }
            state.drag = Some(dragged);
        }
    }

    /// The move of `dragged` to the host slot `slot`: before the tab at the slot when it is shown,
    /// else after the tab before it. `None` for a move that changes nothing, which is the dragged
    /// tab's own slot and the one after it.
    fn slot_move(&self, plan: &StripPlan, dragged: &K, slot: usize, modifiers: Modifiers) -> Option<Move<K>> {
        let from = self.tabs.iter().position(|tab| tab.key() == dragged)?;
        let copy = self.reorder().copies(modifiers);
        if !copy && (slot == from || slot == from + 1) {
            return None;
        }
        let (target, position) =
            if plan.rect_of(slot).is_some() { (slot, DropPosition::Before) } else { (slot.checked_sub(1)?, DropPosition::After) };
        Some(Move { items: vec![dragged.clone()], target: self.tabs.get(target)?.key().clone(), position, copy })
    }

    /// The focus spot on `plan`: the tab or button the state names, the button for a tab moved
    /// into the overflow, else the active tab, else the first shown.
    fn spot(&self, state: &State<K>, plan: &StripPlan, selected: Option<usize>) -> Option<FocusSpot> {
        let named = match &state.spot {
            Some(Spot::Tab(key)) => self.tabs.iter().position(|t| t.key() == key).and_then(|index| focus_spot(plan, index)),
            Some(Spot::Overflow) => plan.overflow.map(|_| FocusSpot::Overflow),
            None => None,
        };
        named.or_else(|| selected.and_then(|index| focus_spot(plan, index))).or_else(|| (!plan.shown.is_empty()).then_some(FocusSpot::Tab(0)))
    }

    fn spot_key(&self, spot: FocusSpot, plan: &StripPlan) -> Spot<K> {
        match spot {
            FocusSpot::Tab(at) => Spot::Tab(self.tabs[plan.shown[at].index].key().clone()),
            FocusSpot::Overflow => Spot::Overflow,
        }
    }

    /// Takes the strip's keys from this pass's input.
    fn keys(&self, ui: &Ui, reorderable: bool) -> StripKeys {
        let mut keys = StripKeys::default();
        let rtl = self.rtl;
        let mac = matches!(ui.ctx().os(), OperatingSystem::Mac | OperatingSystem::IOS);
        ui.input_mut(|i| {
            i.events.retain(|event| {
                let Event::Key { key, pressed: true, modifiers, .. } = event else { return true };
                if let Some(stack) = reorderable.then(|| reorder_key(*key, *modifiers, mac)).flatten() {
                    keys.reorders.push((stack, *modifiers));
                    return false;
                }
                if let Some(mv) = focus_move(*key, *modifiers, rtl) {
                    keys.moves.push(mv);
                    return false;
                }
                let none = modifiers.matches_exact(Modifiers::NONE);
                match key {
                    Key::Enter | Key::Space if none => keys.activate = true,
                    Key::Menu if none => keys.menu = true,
                    Key::F10 if modifiers.matches_exact(Modifiers::SHIFT) => keys.menu = true,
                    _ => return true,
                }
                false
            });
        });
        keys
    }

    /// A tab's clicks, its ×'s, and a screen reader's actions on it.
    fn tab_pointer(&mut self, ui: &Ui, state: &mut State<K>, key: &K, tab: &Response, close: Option<&Response>, middle_closes: bool) {
        let menu = |pointer| TabEvent::ContextMenu(TabMenuRequest { tab: Some(key.clone()), anchor: tab.rect, pointer });
        let close_clicked = close.is_some_and(Response::clicked);
        let middle = tab.middle_clicked() || close.is_some_and(Response::middle_clicked);
        let secondary = [Some(tab), close].into_iter().flatten().find(|r| r.secondary_clicked());
        if close_clicked {
            self.events.push(TabEvent::Close(key.clone()));
        } else if tab.double_clicked() {
            self.events.push(TabEvent::DoubleClick(Some(key.clone())));
        } else if tab.clicked() {
            self.events.push(TabEvent::Activate(key.clone()));
        }
        if middle && middle_closes {
            self.events.push(TabEvent::Close(key.clone()));
        }
        if let Some(response) = secondary {
            self.events.push(menu(response.interact_pointer_pos()));
        }
        if ui.input(|i| i.has_accesskit_action_request(tab.id, Action::ShowContextMenu)) {
            self.events.push(menu(None));
        }
        let pressed = [Some(tab), close].into_iter().flatten().any(|r| r.is_pointer_button_down_on() || r.clicked() || r.secondary_clicked());
        if self.focused && pressed {
            // egui drops the focus on a press outside the strip's own widget.
            self.keep_focus = true;
            self.move_spot(state, Spot::Tab(key.clone()));
        }
        if ui.input(|i| i.has_accesskit_action_request(tab.id, Action::Focus)) {
            self.keep_focus = true;
            self.move_spot(state, Spot::Tab(key.clone()));
        }
    }

    fn move_spot(&mut self, state: &mut State<K>, spot: Spot<K>) {
        state.spot = Some(spot);
        self.spot_set = true;
    }

    /// The overflow button at `at` and its menu.
    fn overflow(
        &mut self,
        ui: &mut Ui,
        state: &mut State<K>,
        (at, list): (Rect, OverflowMenu),
        (plan, selected): (&StripPlan, Option<&K>),
        (text, open, has_spot): (&TabStripText, bool, bool),
    ) {
        let style = self.style;
        let tabs = self.tabs;
        let button_id = self.id.with("overflow");
        let button = Interactive::with_id(button_id, Role::Button, text.more.clone())
            .sense(Sense::CLICK)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, at)
            .finish(ui);
        ui.ctx().accesskit_node_builder(button_id, |node| node.add_action(Action::Focus));
        let row = |tab: &Tab<K>| match tab.detail_text() {
            Some(detail) => format!("{}, {detail}", tab.label()),
            None => tab.label().to_owned(),
        };
        let entries: Vec<MenuEntry<K>> = match list {
            OverflowMenu::AllTabs => {
                tabs.iter().map(|tab| MenuItem::new(row(tab), tab.key().clone()).check(selected == Some(tab.key())).into()).collect()
            }
            OverflowMenu::HiddenOnly => plan.hidden.iter().map(|&index| MenuItem::new(row(&tabs[index]), tabs[index].key().clone()).into()).collect(),
        };
        let menu = PressMenu::new(&entries).keyboard_owner(self.id, open).show(ui, &button);
        if let Some(key) = menu.chosen {
            self.events.push(TabEvent::Activate(key.clone()));
            if self.focused {
                self.move_spot(state, Spot::Tab(key));
            }
        }
        if self.focused && (button.is_pointer_button_down_on() || button.clicked()) {
            self.keep_focus = true;
            self.move_spot(state, Spot::Overflow);
        }
        if ui.input(|i| i.has_accesskit_action_request(button_id, Action::Focus)) {
            self.keep_focus = true;
            self.move_spot(state, Spot::Overflow);
        }
        if has_spot {
            self.active_node = Some(button_id);
        }
        let lit = (button.hovered() && ui.is_enabled()) || menu.open;
        if lit {
            let r = crate::theme::corner_radius(style.radius).nw;
            ui.painter().rect_filled(at, CornerRadius { nw: r, ne: r, sw: 0, se: 0 }, style.active);
        }
        let color = if lit { style.text } else { style.inactive_text };
        icons::paint_in(ui, at, Icon::ChevronsRight, icons::active(ui.ctx()), style.icon_size, IconSize::Sm.stroke(), color);
        let ring = FocusRing::style(&Tokens::of(ui));
        FocusRing::paint_if(ui, at.shrink(2.0 + ring.outset()), CornerRadius::ZERO, self.focused && has_spot);
    }

    /// The empty strip: a double click and a secondary click.
    fn background(&mut self, background: &Response, strip: Rect) {
        if background.double_clicked() {
            self.events.push(TabEvent::DoubleClick(None));
        }
        if background.secondary_clicked() {
            let pointer = background.interact_pointer_pos();
            self.events.push(TabEvent::ContextMenu(TabMenuRequest { tab: None, anchor: strip, pointer }));
        }
        if self.focused && (background.is_pointer_button_down_on() || background.clicked() || background.secondary_clicked()) {
            self.keep_focus = true;
        }
    }

    /// The wheel over the tabs: each notch activates the next or previous tab in the host's
    /// order, stopping at the ends; trackpad scrolling adds up to a notch.
    fn wheel(&mut self, ui: &Ui, state: &mut State<K>, area: Rect, selected: Option<usize>) {
        if !ui.rect_contains_pointer(area) {
            state.wheel = 0.0;
            return;
        }
        let notches: f32 = ui.input(|i| {
            i.events
                .iter()
                .map(|event| match event {
                    Event::MouseWheel { unit, delta, modifiers, .. } if !modifiers.command && !modifiers.ctrl => match unit {
                        MouseWheelUnit::Point => delta.y / WHEEL_NOTCH,
                        MouseWheelUnit::Line | MouseWheelUnit::Page => delta.y,
                    },
                    _ => 0.0,
                })
                .sum()
        });
        let Some(active) = selected.filter(|_| notches != 0.0 && notches.is_finite()) else { return };
        let total = state.wheel + notches;
        let steps = total.trunc();
        state.wheel = total - steps;
        // A wheel turned down scrolls content up: a negative delta activates the next tab.
        let last = self.tabs.len().saturating_sub(1) as i64;
        let to = (active as i64).saturating_sub(steps as i64).clamp(0, last) as usize;
        if to != active {
            self.events.push(TabEvent::Activate(self.tabs[to].key().clone()));
        }
    }
}

/// The strip's keys pressed this pass.
#[derive(Default)]
struct StripKeys {
    moves: Vec<FocusMove>,
    activate: bool,
    menu: bool,
    /// The reorder keys, read for a strip (`]` is later in reading order).
    reorders: Vec<(StackKey, Modifiers)>,
}

/// Where the drop line for `slot` goes on `plan`: the shown tab it lies in and whether the line is
/// at its left edge. Before the tab at `slot` when that tab is shown, else after the last shown tab
/// before the slot, else before the first shown tab. `None` when nothing is shown.
pub(super) fn marker_at(plan: &StripPlan, rtl: bool, slot: usize) -> Option<(Rect, bool)> {
    let before = |rect: Rect| (rect, !rtl);
    let after = |rect: Rect| (rect, rtl);
    if let Some(tab) = plan.shown.iter().find(|tab| tab.index == slot) {
        return Some(before(tab.rect));
    }
    match plan.shown.iter().filter(|tab| tab.index < slot).max_by_key(|tab| tab.index) {
        Some(tab) => Some(after(tab.rect)),
        None => plan.shown.first().map(|tab| before(tab.rect)),
    }
}

/// The drop line inside `tab` at its left or right edge: `tab.drop_width` of `tab.drop` on its
/// halo, the strip's full height.
pub(super) fn paint_marker(ui: &Ui, style: &TabStyle, tab: Rect, left: bool) {
    let halo = style.drop_halo;
    let width = style.drop_width + 2.0 * halo.width;
    let x = if left { tab.left() } else { tab.right() - width };
    let band = Rect::from_min_size(pos2(x, tab.top()), vec2(width, tab.height()));
    ui.painter().rect_filled(band, 0.0, halo.color);
    ui.painter().rect_filled(band.shrink2(vec2(halo.width, 0.0)), 0.0, style.drop);
}
