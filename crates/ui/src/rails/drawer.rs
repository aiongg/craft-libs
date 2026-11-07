//! `ToolbarDrawer`: Edit Toolbar's All Tools drawer beside the tool rail, where tools move between the toolbar and the
//! drawer.

use super::tool::{DropAt, RailTool, SlotId, ToolDrag, ToolDropped, ToolSlot};
use super::words::RailWords;
use crate::buttons::{Button, IconButton};
use crate::contract::overlay::{Anchor, Dismiss, Outside, Place};
use crate::contract::{
    Bind, CollectionEvent, Gap, NoValue, Outcome, Props as _, SelectMode, Show as _, ShowCollection as _, ShowContainer as _, Sizing, clicked,
};
use crate::display::{Shade, Text};
use crate::layout_components::{Row, Stack};
use crate::list::{List, ListRow, RowInk};
use crate::popover::Popover;
use crate::primitives::IconSpec;
use crate::runtime::commands::{self, ResolvedCommand};
use crate::runtime::env::{Env, Label};
use crate::runtime::windows::os_of;
use crate::sections::DropAction;
use crate::style::TypeRole;
use craft_behavior::overlay::{Align, Side};
use craft_behavior::{DisplayOrder, IconId, ItemSource, SelectionChange, Value};
use egui::{Rect, Response, Ui};
use std::borrow::Cow;

#[cfg(feature = "testing")]
mod stories;

/// A group of the drawer: a heading over the tools that belong under it.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DrawerGroup<T> {
    /// The group's heading.
    pub label: Label,
    /// The group's tools, in the order the drawer lists them.
    pub tools: Vec<RailTool<T>>,
}

impl<T> DrawerGroup<T> {
    /// A group headed `label` that lists `tools`.
    pub fn new(label: impl Into<Label>, tools: Vec<RailTool<T>>) -> Self {
        DrawerGroup { label: label.into(), tools }
    }
}

/// A change to the toolbar that the user made in the drawer. The host applies it with [`apply_toolbar_edit`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ToolbarEdit<T> {
    /// Put the tool on the toolbar at `at`.
    #[non_exhaustive]
    Add {
        /// The tool.
        tool: RailTool<T>,
        /// Where it goes.
        at: DropAt,
    },
    /// Take the tool off the toolbar.
    #[non_exhaustive]
    Remove {
        /// The tool.
        tool: T,
    },
    /// Move a tool that is on the toolbar to `at`.
    #[non_exhaustive]
    Move {
        /// The tool.
        tool: T,
        /// Where it goes.
        at: DropAt,
    },
    /// Go back to the default toolbar.
    Reset,
}

impl<T> ToolbarEdit<T> {
    /// Put `tool` on the toolbar at `at`.
    pub fn add(tool: RailTool<T>, at: DropAt) -> Self {
        ToolbarEdit::Add { tool, at }
    }

    /// Take the tool `tool` off the toolbar.
    pub fn remove(tool: T) -> Self {
        ToolbarEdit::Remove { tool }
    }

    /// Move the toolbar's tool `tool` to `at`.
    pub fn move_to(tool: T, at: DropAt) -> Self {
        ToolbarEdit::Move { tool, at }
    }
}

/// What the drawer reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ToolbarDrawerOutput<T> {
    /// The drawer's response.
    pub response: Response,
    /// The changes made in this pass, in order.
    pub edits: Vec<ToolbarEdit<T>>,
    /// The tool of the row the user selected.
    pub select: Option<T>,
    /// The drawer closed in this pass; the host stops showing it.
    pub closed: bool,
}

impl<T: Value> Outcome for ToolbarDrawerOutput<T> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The All Tools drawer: a popover beside the rail with one list per group. A tool that is on the toolbar is dimmed.
///
/// - **Rows.** A selected row reports its tool as `select`. Enter or a double click on a tool that is not on the
///   toolbar adds it as a slot of its own at the end; on a tool that is on the toolbar it takes it off.
/// - **Drags.** A tool dragged from the rail and dropped on the drawer is taken off the toolbar. A drop on the rail
///   comes back from the rail as a [`ToolDropped`]; hand it to [`ToolbarDrawer::new`] and the drawer turns it into an
///   edit.
/// - **Closing.** Escape or the close button closes it: `closed` is set and the host stops showing it.
#[must_use = "show the drawer with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
#[pattern(probe(ToolbarDrawer<'static, u8>))]
pub struct ToolbarDrawer<'a, T: Value + 'static> {
    rail: Rect,
    groups: &'a [DrawerGroup<T>],
    toolbar: &'a [ToolSlot<T>],
    /// The side of the rail the drawer opens on.
    #[prop]
    opens: Side,
    dropped: Option<ToolDropped>,
}

impl<'a, T: Value + 'static> ToolbarDrawer<'a, T> {
    /// A drawer beside `rail` (the rail's rect), listing `groups`, over a toolbar of `toolbar` slots. `dropped` is the
    /// drag the rail took in this pass (`ToolRailOutput::dropped`); the drawer turns it into an edit.
    pub fn new(rail: Rect, groups: &'a [DrawerGroup<T>], toolbar: &'a [ToolSlot<T>], dropped: Option<ToolDropped>) -> Self {
        ToolbarDrawer { rail, groups, toolbar, opens: Side::End, dropped }
    }
}

/// The resolved commands of one group, as the source of its list.
struct Rows<'a>(&'a [ResolvedCommand]);

impl DisplayOrder<usize> for Rows<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        self.0.get(at).map(|command| Cow::Borrowed(&*command.label))
    }
}

impl ItemSource<usize> for Rows<'_> {}

/// Whether the toolbar holds the tool `id`.
fn on_toolbar<T: PartialEq>(toolbar: &[ToolSlot<T>], id: &T) -> bool {
    toolbar.iter().any(|slot| slot.tools.iter().any(|tool| tool.id == *id))
}

impl<T: Value + 'static> ToolbarDrawer<'_, T> {
    /// Shows the drawer and reports what the user changed.
    pub fn show(self, ui: &mut Ui) -> ToolbarDrawerOutput<T> {
        let ToolbarDrawer { rail, groups, toolbar, opens, dropped } = self;
        let env = Env::of(ui);
        let words = env.words::<RailWords>();
        let os = os_of(ui.ctx());
        let ctx = ui.ctx().clone();
        let resolved: Vec<Vec<ResolvedCommand>> =
            groups.iter().map(|group| group.tools.iter().map(|tool| commands::resolve(&ctx, tool.command.clone())).collect()).collect();
        let mut edits: Vec<ToolbarEdit<T>> = dropped.and_then(|dropped| drop_edit(dropped.drag, dropped.at, groups, toolbar)).into_iter().collect();
        let mut select: Option<T> = None;
        let mut open = true;
        let mut leave = false;
        let layout = env.view().keyboard_layout();
        let chord_of = |command: &ResolvedCommand| command.chord.as_ref().map(|chord| chord.display_for(os, layout));

        let popover = Popover::new(words.drawer_title.clone(), Bind::bound(&mut open))
            .overlay(|overlay| overlay.place(Place::anchored(Anchor::Rect(rail), opens, Align::Start)))
            .dismiss(Dismiss::new().outside(Outside::Keep).focus_left(false));
        let shown = popover.show(ui, |ui, _| {
            let accepts = |drag: &ToolDrag| matches!(drag, ToolDrag::Rail { .. });
            let slot = DropAction::<ToolDrag>::new(accepts).show(ui, |ui, _| {
                Stack::new().gap(Gap::Row).show(ui, |ui, _| {
                    Row::new().show(ui, |ui, _| {
                        Text::new(words.drawer_title.clone()).role(TypeRole::Title).show(ui);
                        let close = IconButton::new(IconId::new("close")).label(words.drawer_close.clone()).show(ui);
                        leave |= clicked(&close);
                    });
                    Text::new(words.drawer_hint.clone()).role(TypeRole::Caption).shade(Shade::Faint).show(ui);
                    for (g, group) in groups.iter().enumerate() {
                        if group.tools.is_empty() {
                            continue;
                        }
                        Text::new(group.label.clone()).role(TypeRole::BodyStrong).show(ui);
                        let rows = Rows(&resolved[g]);
                        let list = List::<usize>::new(group.label.clone())
                            .id(ui.scope_id().with(("drawer-group", g)))
                            .select(SelectMode::Single)
                            .sizing(Sizing::Content);
                        let out = list.show_items(ui, &rows, &mut |key, _| {
                            let tool = &group.tools[*key];
                            let command = &resolved[g][*key];
                            let mut row = ListRow::new(command.label.to_string());
                            if let Some(icon) = &command.icon {
                                row = row.icon(IconSpec::svg(IconId::from(icon.clone())));
                            }
                            if on_toolbar(toolbar, &tool.id) {
                                row.ink(RowInk::Dimmed).trailing(words.in_toolbar.clone())
                            } else if let Some(chord) = chord_of(command) {
                                row.trailing(chord)
                            } else {
                                row
                            }
                        });
                        for event in out.events {
                            match event {
                                CollectionEvent::Selection(SelectionChange::Set(keys)) => {
                                    select = keys.last().and_then(|key| group.tools.get(*key)).map(|tool| tool.id.clone());
                                }
                                CollectionEvent::Activate(key) => {
                                    let Some(tool) = group.tools.get(key) else { continue };
                                    edits.push(if on_toolbar(toolbar, &tool.id) {
                                        ToolbarEdit::Remove { tool: tool.id.clone() }
                                    } else {
                                        ToolbarEdit::Add { tool: tool.clone(), at: DropAt::new_slot(toolbar.len()) }
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                    if clicked(&Button::new(words.drawer_restore.clone()).show(ui)) {
                        edits.push(ToolbarEdit::Reset);
                    }
                });
            });
            if let Some(ToolDrag::Rail { slot: from, index }) = slot.output.dropped
                && let Some(tool) = toolbar.iter().find(|candidate| candidate.id == from).and_then(|candidate| candidate.tools.get(index))
            {
                edits.push(ToolbarEdit::Remove { tool: tool.id.clone() });
            }
        });
        let closed = leave || !open || shown.output.closed.is_some();
        ToolbarDrawerOutput { response: shown.output.response, edits, select, closed }
    }
}

/// The edit a drag dropped on the rail at `at` asks for, `None` when it changes nothing: a tool of the rail dropped on its own
/// slot stays where it is, and a drag whose tool is gone is dropped.
pub fn drop_edit<T: Value>(drag: ToolDrag, at: DropAt, groups: &[DrawerGroup<T>], toolbar: &[ToolSlot<T>]) -> Option<ToolbarEdit<T>> {
    match drag {
        ToolDrag::Drawer { group, index } => {
            let tool = groups.get(group)?.tools.get(index)?;
            Some(ToolbarEdit::Add { tool: tool.clone(), at })
        }
        ToolDrag::Rail { slot, index } => {
            let tool = toolbar.iter().find(|candidate| candidate.id == slot)?.tools.get(index)?;
            if matches!(at, DropAt::Slot { slot: onto, .. } if onto == slot) {
                return None;
            }
            Some(ToolbarEdit::Move { tool: tool.id.clone(), at })
        }
    }
}

/// The toolbar after `edit`; `defaults` is the toolbar `Reset` goes back to. A slot left with no tool goes.
pub fn apply_toolbar_edit<T: Clone + PartialEq>(slots: &[ToolSlot<T>], edit: &ToolbarEdit<T>, defaults: &[ToolSlot<T>]) -> Vec<ToolSlot<T>> {
    match edit {
        ToolbarEdit::Reset => defaults.to_vec(),
        ToolbarEdit::Add { tool, at } => place(slots, tool.clone(), *at),
        ToolbarEdit::Move { tool, at } => match slots.iter().flat_map(|slot| &slot.tools).find(|candidate| candidate.id == *tool) {
            Some(found) => place(slots, found.clone(), *at),
            None => slots.to_vec(),
        },
        ToolbarEdit::Remove { tool } => {
            let mut out = slots.to_vec();
            for slot in &mut out {
                slot.tools.retain(|candidate| candidate.id != *tool);
            }
            out.retain(|slot| !slot.tools.is_empty());
            out
        }
    }
}

fn place<T: Clone + PartialEq>(slots: &[ToolSlot<T>], tool: RailTool<T>, at: DropAt) -> Vec<ToolSlot<T>> {
    let mut out = slots.to_vec();
    let mut was: Option<(SlotId, usize)> = None;
    for slot in &mut out {
        if let Some(index) = slot.tools.iter().position(|candidate| candidate.id == tool.id) {
            slot.tools.remove(index);
            was = Some((slot.id, index));
        }
    }
    match at {
        DropAt::Slot { slot, index } => {
            let Some(target) = out.iter_mut().find(|candidate| candidate.id == slot) else {
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
