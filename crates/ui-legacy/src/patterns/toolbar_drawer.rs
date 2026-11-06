//! [`ToolbarDrawer`]: Edit Toolbar's All Tools drawer beside the tool rail, where tools move
//! between the toolbar and the drawer by dragging.

use std::hash::Hash;

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::Role;
use egui::{Area, AsIdSalt, DragAndDrop, Id, IdSalt, Key, Modifiers, Order, Rect, Sense, Ui, Vec2, pos2, vec2};

use super::rail::tool::{ToolDrag, dragged_target, drop_places, edit_state, edit_update};
use super::rail::{RailTool, SlotId, ToolSlot};
use crate::controls::{Button, IconButton, Kbd, icon_size};
use crate::direction;
use crate::focus::{Region, RegionFocus};
use crate::icons::{self, Icon};
use crate::localized::Labels as _;
use crate::menu::row::{Columns, MenuStyle, RowContent, RowPainter, RowState};
use crate::menu::{self, ItemKind};
use crate::popover::{Popover, PopoverAnchor, PopoverClose, PopoverSize};
use crate::primitives::{Interactive, Surface, SurfaceKind, Text};
use crate::style::rail::RailStyle;
use crate::theme::{ToEgui, Tokens, TokensOf};

/// A heading in the drawer and the tools under it ("Select", "Paint").
#[derive(Clone, Debug, PartialEq)]
pub struct DrawerGroup<T> {
    /// The heading text. Translate it in the app.
    pub label: String,
    /// The tools listed under the heading, in order.
    pub tools: Vec<RailTool<T>>,
}

/// A toolbar edit the user made in the drawer. The host applies it to its toolbar layout (the
/// [`ToolSlot`]s it passes to [`super::ToolRail`]) and stores the layout with its preferences.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ToolbarEdit<T> {
    /// A tool (with its icon, name and shortcut, as the drawer lists it) dragged from the drawer
    /// onto the toolbar: into slot `slot` at `index` of its tools
    /// (dropped onto a tool: joins its group), or as a new slot before slot index `before` (dropped
    /// between two slots).
    Add {
        /// The tool added.
        tool: RailTool<T>,
        /// Where it was dropped.
        at: DropAt,
    },
    /// A tool dragged from the toolbar into the drawer.
    Remove {
        /// The id of the tool removed.
        tool: T,
    },
    /// A tool dragged to another place in the toolbar.
    Move {
        /// The id of the tool moved.
        tool: T,
        /// Where it was dropped.
        at: DropAt,
    },
    /// "Restore Defaults" at the drawer's foot.
    Reset,
}

/// Where a dragged tool lands in the toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropAt {
    /// Into an existing slot's group, at this position of its tools.
    Slot {
        /// The slot's id.
        slot: SlotId,
        /// The position among the slot's tools.
        index: usize,
    },
    /// As a new slot before the slot at this index (the slot count for the end).
    NewSlot {
        /// The index of the slot the new one goes before.
        before: usize,
    },
}

/// What [`ToolbarDrawer::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ToolbarDrawerOutput<T> {
    /// The toolbar edits made this frame, in order.
    pub edits: Vec<ToolbarEdit<T>>,
    /// A tool clicked in the drawer: the host selects it, as a rail click would.
    pub select: Option<T>,
    /// Esc, a click outside, or the close button: the host stops showing the drawer.
    pub closed: bool,
}

/// The All Tools drawer, opened by the rail's Edit Toolbar button ([`super::ToolRail::edit_toolbar`],
/// [`super::ToolRailOutput::edit_toolbar`]) and drawn beside the rail while the host keeps it
/// open and tells the rail with [`super::ToolRail::editing`].
///
/// ```no_run
/// use craft_ui::patterns::{DrawerGroup, ToolbarDrawer, ToolSlot};
/// # fn demo(ui: &mut egui::Ui, rail_rect: egui::Rect, groups: &[DrawerGroup<&'static str>], slots: &[ToolSlot<&'static str>], open: &mut bool) {
/// let out = ToolbarDrawer::new("edit-toolbar").show(ui, rail_rect, groups, slots);
/// for edit in out.edits { /* apply to the toolbar layout, save it */ }
/// if out.closed { *open = false; }
/// # }
/// ```
///
/// - **Look:** a popup surface beside the rail (on its far side from the window edge), one column
///   of rows (icon, name, shortcut) under group headings; tools already in the toolbar are dimmed.
///   Restore Defaults at the foot.
/// - **Editing:** only while the drawer is open, as in Illustrator and Photoshop: a drawer row
///   dragged onto the rail adds the tool (between slots: a new slot; onto a slot: into its group);
///   a rail tool dragged into the drawer removes it; rail tools drag to new places. The drop
///   target shows as an insertion line or a slot highlight. The rail reads the drag from egui's
///   `DragAndDrop` payload, so the rail draws the targets.
/// - **Keyboard:** the drawer takes focus when it opens (first row); arrows move, Enter selects the
///   tool, Space picks a row up and arrows then choose its place on the rail, Enter drops, Esc
///   cancels the move or closes the drawer. Focus returns to the Edit Toolbar button on close.
/// - **AccessKit:** a `Dialog` named "Edit Toolbar"; rows are `ListItem`s with "in toolbar" as a
///   description when dimmed.
#[must_use = "show the drawer with `show`"]
pub struct ToolbarDrawer {
    id: Id,
    /// The rail this drawer edits, by the rail's id salt.
    rail: IdSalt,
    labels: Option<ToolbarDrawerLabels>,
}

/// The drawer's state between passes.
#[derive(Clone, Copy, Default)]
struct State {
    /// The row picked up with Space: its group and place in the group.
    picked: Option<(usize, usize)>,
    /// The popover's width as last drawn.
    width: f32,
}

impl ToolbarDrawer {
    /// A drawer with English labels. `id_salt` keys its state and, until `rail` is called, names
    /// the rail it edits.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        let id_salt = IdSalt::new(id_salt);
        Self { id: Id::unique("craft-ui-toolbar-drawer").with(id_salt), rail: id_salt, labels: None }
    }

    /// The id salt of the [`ToolRail`](super::ToolRail) this drawer edits, when it is not the
    /// drawer's own: the rail and the drawer tell each other the rail's places and the drop
    /// target through state keyed by it, so drawers beside several rails stay apart.
    pub fn rail(mut self, rail_id_salt: impl AsIdSalt) -> Self {
        self.rail = IdSalt::new(rail_id_salt);
        self
    }

    /// The texts, translated. Without them the installed words are used.
    pub fn labels(mut self, labels: ToolbarDrawerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the drawer beside `rail` (the rail's rect from [`super::ToolRailOutput::rect`]) with
    /// every tool in `groups`, dimming those in `toolbar`.
    // vectorcraft: crates/ui-egui/src/edit_toolbar.rs:409 `drawer`, :165 `add`, :206 `remove`,
    // :274 `drop_target`, :223 `command` (`toolbar.addTool`, `toolbar.removeTool`, `toolbar.reset`).
    pub fn show<T: Clone + PartialEq + Hash + Send + Sync + 'static>(
        self,
        ui: &mut Ui,
        rail: Rect,
        groups: &[DrawerGroup<T>],
        toolbar: &[ToolSlot<T>],
    ) -> ToolbarDrawerOutput<T> {
        let Self { id, rail: rail_salt, labels } = self;
        // Scoped by the Ui, so the same drawer drawn twice in a pass (once per theme in a variants
        // view) does not put one widget id on two layers.
        let id = ui.scope_id().with(id);
        let ctx = ui.ctx().clone();
        let labels = labels.unwrap_or_else(|| ToolbarDrawerLabels::of(ui));
        let tokens = Tokens::of(ui);
        let mut out = ToolbarDrawerOutput { edits: Vec::new(), select: None, closed: false };
        let edit = edit_state(&ctx, rail_salt);
        let state_id = id.with("state");
        let mut state = ctx.data(|d| d.get_temp::<State>(state_id)).unwrap_or_default();

        let menu_style = MenuStyle::new(&tokens, ControlSize::Md);
        let painter = RowPainter { style: &menu_style, tokens: &tokens, size: ControlSize::Md };
        let kbds: Vec<Vec<Option<Kbd>>> =
            groups.iter().map(|group| group.tools.iter().map(|tool| tool.shortcut.as_deref().map(|keys| Kbd::keys(&[keys]))).collect()).collect();
        let measuring = Columns { marks: false, icons: true, width: 0.0 };
        let widest = groups
            .iter()
            .zip(&kbds)
            .flat_map(|(group, kbds)| group.tools.iter().zip(kbds))
            .map(|(tool, kbd)| painter.width(ui, &content(tool, kbd), &measuring))
            .fold(menu_style.min_width, f32::max);
        let width = widest + ui.spacing().scroll.allocated_width();

        let screen = ctx.content_rect();
        let gap = tokens.space.gap.stack;
        let outer = if state.width > 0.0 { state.width } else { width + 2.0 * tokens.space.gap.stack };
        let x = match (rail.center().x < screen.center().x, direction::is_rtl(ui)) {
            (true, false) => rail.right() + gap,
            (true, true) => rail.right() + gap + outer,
            (false, false) => rail.left() - gap - outer,
            (false, true) => rail.left() - gap,
        };
        let first = groups.iter().position(|group| !group.tools.is_empty()).map(|g| row_id(id, g, 0));
        let in_toolbar = |tool: &T| toolbar.iter().any(|slot| slot.tools.iter().any(|t| t.id == *tool));

        let mut open = true;
        let mut picked = state.picked;
        let mut leave = false;
        let shown = Popover::new(id.with("popover"), PopoverAnchor::Point(pos2(x, rail.top())))
            .size(PopoverSize::Fit)
            .close(PopoverClose::default().outside(false).escape(false).focus_left(false))
            .label(labels.title.as_str())
            .show(ui, &mut open, |ui, pop| {
                if let Some(first) = first {
                    pop.initial_focus(first);
                }
                match picked {
                    Some(pick) => {
                        let places = drop_places(&edit.placed, edit.total);
                        let at = edit.target.and_then(|target| places.iter().position(|place| *place == target)).unwrap_or(places.len() - 1);
                        let (cancel, drop, step) = ui.input_mut(|input| {
                            let cancel = input.consume_key(Modifiers::NONE, Key::Escape);
                            let drop = input.consume_key(Modifiers::NONE, Key::Enter);
                            input.consume_key(Modifiers::NONE, Key::Space);
                            let next = input.consume_key(Modifiers::NONE, Key::ArrowDown) | input.consume_key(Modifiers::NONE, Key::ArrowRight);
                            let previous = input.consume_key(Modifiers::NONE, Key::ArrowUp) | input.consume_key(Modifiers::NONE, Key::ArrowLeft);
                            (cancel, drop, i64::from(next) - i64::from(previous))
                        });
                        let to = (at as i64 + step).clamp(0, places.len() as i64 - 1) as usize;
                        if drop && let Some(tool) = groups.get(pick.0).and_then(|group| group.tools.get(pick.1)) {
                            out.edits.push(ToolbarEdit::Add { tool: tool.clone(), at: places[at] });
                        }
                        if cancel || drop {
                            picked = None;
                            edit_update(&ctx, rail_salt, |edit| edit.target = None);
                        } else {
                            edit_update(&ctx, rail_salt, |edit| edit.target = Some(places[to]));
                        }
                    }
                    None => leave = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)),
                }
                ui.set_min_width(width);
                direction::horizontal(ui, |ui| {
                    ui.add(Text::new(labels.title.as_str()).role(TypeRole::Title));
                    ui.with_layout(direction::trailing(ui), |ui| {
                        leave |= ui.add(IconButton::new(Icon::Close, labels.close.as_str())).clicked();
                    });
                });
                ui.add(Text::new(labels.hint.as_str()).role(TypeRole::Caption).content(crate::primitives::Content::Faint));
                let room = (ui.available_height() - ui.spacing().interact_size.y - ui.spacing().item_spacing.y).max(0.0);
                let mut ids = Vec::new();
                egui::ScrollArea::vertical().id_salt(id.with("scroll")).max_height(room).auto_shrink([false, true]).show(ui, |ui| {
                    let row_width = ui.available_width();
                    let cols = Columns { marks: false, icons: true, width: row_width };
                    for (g, group) in groups.iter().enumerate() {
                        ui.add(Text::new(group.label.as_str()).role(TypeRole::BodyStrong));
                        for (t, tool) in group.tools.iter().enumerate() {
                            let rid = row_id(id, g, t);
                            let dimmed = in_toolbar(&tool.id);
                            if !dimmed
                                && picked.is_none()
                                && ctx.memory(|m| m.has_focus(rid))
                                && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Space))
                            {
                                picked = Some((g, t));
                                edit_update(&ctx, rail_salt, |edit| edit.target = Some(DropAt::NewSlot { before: edit.total }));
                            }
                            let sense = if dimmed { Sense::CLICK } else { Sense::click_and_drag() };
                            let row = content(tool, &kbds[g][t]);
                            let r = Interactive::with_id(rid, Role::ListItem, tool.label.as_str())
                                .sense(sense)
                                .state_layer(false)
                                .focus_ring(false)
                                .interact(ui, vec2(row_width, menu_style.rows.height))
                                .finish(ui);
                            if !dimmed {
                                r.dnd_set_drag_payload(ToolDrag::Drawer { group: g, index: t });
                            }
                            if r.clicked() {
                                out.select = Some(tool.id.clone());
                            }
                            if r.gained_focus() {
                                r.scroll_to_me(None);
                            }
                            if dimmed {
                                ctx.accesskit_node_builder(rid, |node| node.set_description(labels.in_toolbar.as_str()));
                            }
                            menu::row::access(&r, &row, false);
                            let active = r.hovered() || r.has_focus() || picked == Some((g, t));
                            painter.paint(ui, r.rect, &row, &cols, RowState { enabled: !dimmed, active, open: false });
                            ids.push(rid);
                        }
                    }
                });
                if picked.is_none() {
                    row_arrows(ui, &ids);
                }
                if ui.add(Button::new(labels.restore.as_str())).clicked() {
                    out.edits.push(ToolbarEdit::Reset);
                }
                let press = ctx.input(|i| i.pointer.press_origin());
                leave |= pop.pressed_outside() && !press.is_some_and(|at| rail.contains(at));
                if leave {
                    pop.close();
                }
            });
        state.picked = picked;
        let rect = shown.as_ref().map(|shown| shown.rect);
        if let Some(rect) = rect {
            state.width = rect.width();
            RegionFocus::new(&ctx).region(&ctx, Region::new("toolbarDrawer", labels.title.as_str()), rect, first);
        }

        // A drag released: onto the rail adds or moves the tool, into the drawer removes it.
        if ctx.input(|i| i.pointer.any_released())
            && let Some(drag) = DragAndDrop::payload::<ToolDrag>(&ctx)
        {
            let target = dragged_target(&ctx, rail_salt);
            DragAndDrop::clear_payload(&ctx);
            if let Some(tool) = dragged_tool(*drag, groups, toolbar) {
                let over_drawer = rect.zip(ctx.pointer_interact_pos()).is_some_and(|(rect, at)| rect.contains(at));
                match (*drag, target) {
                    (ToolDrag::Drawer { .. }, Some(at)) => out.edits.push(ToolbarEdit::Add { tool: tool.clone(), at }),
                    (ToolDrag::Rail { .. }, _) if over_drawer => out.edits.push(ToolbarEdit::Remove { tool: tool.id.clone() }),
                    (ToolDrag::Rail { slot, .. }, Some(at)) if !matches!(at, DropAt::Slot { slot: onto, .. } if onto == slot) => {
                        out.edits.push(ToolbarEdit::Move { tool: tool.id.clone(), at });
                    }
                    _ => {}
                }
            }
        }
        if let (Some(drag), Some(at)) = (DragAndDrop::payload::<ToolDrag>(&ctx), ctx.pointer_interact_pos())
            && let Some(tool) = dragged_tool(*drag, groups, toolbar)
        {
            let rail = RailStyle::new(&tokens);
            Area::new(id.with("ghost")).order(Order::Tooltip).fixed_pos(at + vec2(8.0, 8.0)).interactable(false).show(&ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(tokens.size.rail.button), Sense::hover());
                Surface::paint(ui.painter(), rect, SurfaceKind::Popup, &tokens);
                icons::paint(
                    ui,
                    Rect::from_center_size(rect.center(), Vec2::splat(rail.icon)),
                    tool.icon,
                    icon_size(ControlSize::Md),
                    tokens.color.text.normal.to_egui(),
                );
            });
        }

        if leave || shown.is_none() {
            out.closed = true;
            state = State::default();
            edit_update(&ctx, rail_salt, |edit| edit.target = None);
            if let Some(button) = edit.button {
                ctx.memory_mut(|m| m.request_focus(button));
            }
            ctx.request_repaint();
        }
        ctx.data_mut(|d| d.insert_temp(state_id, state));
        out
    }
}

fn content<'a, T>(tool: &'a RailTool<T>, kbd: &'a Option<Kbd>) -> RowContent<'a> {
    RowContent {
        label: tool.label.as_str(),
        mnemonic: None,
        icon: Some(tool.icon),
        swatch: None,
        kind: ItemKind::Plain,
        shortcut: kbd.as_ref(),
        submenu: false,
        tint: None,
        detail: None,
    }
}

fn row_id(drawer: Id, group: usize, index: usize) -> Id {
    drawer.with(("row", group, index))
}

/// The tool a drag carries.
fn dragged_tool<'a, T>(drag: ToolDrag, groups: &'a [DrawerGroup<T>], toolbar: &'a [ToolSlot<T>]) -> Option<&'a RailTool<T>> {
    match drag {
        ToolDrag::Drawer { group, index } => groups.get(group)?.tools.get(index),
        ToolDrag::Rail { slot, index } => toolbar.iter().find(|s| s.id == slot)?.tools.get(index),
    }
}

/// Up and Down move the focus between the drawer's rows; the rows' own key moves stay with egui's
/// geometric focus movement otherwise.
fn row_arrows(ui: &mut Ui, ids: &[Id]) {
    let Some(at) = ids.iter().position(|id| ui.memory(|m| m.has_focus(*id))) else {
        return;
    };
    let filter = egui::EventFilter { vertical_arrows: true, ..Default::default() };
    ui.memory_mut(|m| m.set_focus_lock_filter(ids[at], filter));
    let target = ui.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
            at.checked_add(1).filter(|to| *to < ids.len())
        } else if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
            at.checked_sub(1)
        } else {
            None
        }
    });
    if let Some(to) = target {
        ui.memory_mut(|m| m.request_focus_with_filter(ids[to], filter));
    }
}

/// The host's toolbar layout `slots` after `edit`; [`ToolbarEdit::Reset`] gives `defaults`. A
/// slot left empty goes; a new slot gets the next free [`SlotId`].
// vectorcraft: crates/ui-egui/src/state.rs:697 `ToolbarEdits` (edits kept per toolbar; None is
// as shipped).
pub fn apply_toolbar_edit<T: Clone + PartialEq>(slots: &[ToolSlot<T>], edit: &ToolbarEdit<T>, defaults: &[ToolSlot<T>]) -> Vec<ToolSlot<T>> {
    match edit {
        ToolbarEdit::Reset => defaults.to_vec(),
        ToolbarEdit::Add { tool, at } => place(slots, tool.clone(), *at),
        ToolbarEdit::Move { tool, at } => match slots.iter().flat_map(|slot| &slot.tools).find(|t| t.id == *tool) {
            Some(found) => place(slots, found.clone(), *at),
            None => slots.to_vec(),
        },
        ToolbarEdit::Remove { tool } => {
            let mut out = slots.to_vec();
            for slot in &mut out {
                slot.tools.retain(|t| t.id != *tool);
            }
            out.retain(|slot| !slot.tools.is_empty());
            out
        }
    }
}

/// `slots` with `tool` taken out of where it was and put at `at`. The places of `at` are those of
/// `slots` before the move; slots the move empties go afterwards.
fn place<T: Clone + PartialEq>(slots: &[ToolSlot<T>], tool: RailTool<T>, at: DropAt) -> Vec<ToolSlot<T>> {
    let mut out = slots.to_vec();
    let mut was: Option<(SlotId, usize)> = None;
    for slot in &mut out {
        if let Some(index) = slot.tools.iter().position(|t| t.id == tool.id) {
            slot.tools.remove(index);
            was = Some((slot.id, index));
        }
    }
    match at {
        DropAt::Slot { slot, index } => {
            let Some(target) = out.iter_mut().find(|s| s.id == slot) else {
                return slots.to_vec();
            };
            let shift = was.is_some_and(|(from, old)| from == slot && old < index);
            let index = (index - usize::from(shift)).min(target.tools.len());
            target.tools.insert(index, tool);
        }
        DropAt::NewSlot { before } => {
            let id = slots.iter().map(|slot| slot.id).max().map_or(0, |max| max + 1);
            out.insert(before.min(out.len()), ToolSlot::new(id, vec![tool]));
        }
    }
    out.retain(|slot| !slot.tools.is_empty());
    out
}

crate::words! {
    /// The drawer's labels, English by default.
    #[non_exhaustive]
    pub struct ToolbarDrawerLabels {
        title: "toolbarDrawer.title" = "Edit Toolbar";
        hint: "toolbarDrawer.hint" = "Drag tools to or from the toolbar.";
        in_toolbar: "toolbarDrawer.inToolbar" = "In toolbar";
        restore: "toolbarDrawer.restore" = "Restore Defaults";
        close: "toolbarDrawer.close" = "Close";
    }
}
