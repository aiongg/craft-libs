//! The Brush Settings section list ([`BrushSectionList`]).

use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use craft_ui::behavior::{RowInfo, Selection};
use craft_ui::controls::CheckState;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, RowEvent, ToggleColumn};
use craft_ui::localized::SharedLabels as _;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{AsIdSalt, IdSalt, Key, Modifiers, Response, Role, Ui};

use super::labels::BrushSettingsLabels;
use super::style::BrushSettingsStyle;

/// What a section row does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BrushSectionKind {
    /// A section with a pane of controls (Brush Tip Shape, Shape Dynamics, …, Brush Pose): a click
    /// shows its pane.
    #[default]
    Pane,
    /// A switch with no pane (Noise, Wet Edges, Build-up, Smoothing, Protect Texture): a click
    /// toggles it and the shown pane stays.
    Switch,
}

/// One row of the section list, built by the app each pass from its brush settings.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushSection<K> {
    /// The key events name the section by.
    pub key: K,
    /// The section's name, shown and read by a screen reader.
    pub name: String,
    /// Pane or switch.
    pub kind: BrushSectionKind,
    /// The enable box's state (a switch's state); `None`: no box (Brush Tip Shape).
    pub enabled: Option<bool>,
    /// The lock's state; `None`: the row has no lock.
    pub locked: Option<bool>,
}

impl<K> BrushSection<K> {
    /// A pane section without an enable box or lock.
    pub fn pane(key: K, name: impl Into<String>) -> Self {
        Self { key, name: name.into(), kind: BrushSectionKind::Pane, enabled: None, locked: None }
    }

    /// A switch row, on or off.
    pub fn switch(key: K, name: impl Into<String>, on: bool) -> Self {
        Self { key, name: name.into(), kind: BrushSectionKind::Switch, enabled: Some(on), locked: None }
    }

    /// Gives the row an enable box in this state.
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = Some(on);
        self
    }

    /// Gives the row a lock in this state.
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = Some(locked);
        self
    }

    /// Whether the section is off: it has a box and the box is clear. Its pane shows greyed.
    pub fn is_off(&self) -> bool {
        self.enabled == Some(false)
    }
}

/// What the section list reports: one-shot changes, in the order they happened. The app applies
/// them to its brush settings as tool options (no history state).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrushSectionEvent<K> {
    /// A pane row was clicked (or Enter, or the lead moved onto it): show its pane.
    Select(K),
    /// A pane section's enable box changed, or a click on its row turned it on.
    Enable {
        /// The section.
        key: K,
        /// The new state.
        on: bool,
    },
    /// A section's lock changed: a locked section keeps its settings when a preset is picked.
    Lock {
        /// The section.
        key: K,
        /// The new state.
        on: bool,
    },
    /// A switch row changed.
    Switch {
        /// The section.
        key: K,
        /// The new state.
        on: bool,
    },
    /// The "Brushes" link row was clicked: open the Brushes panel.
    OpenLink,
}

/// Photoshop's Brush Settings section list: an optional "Brushes" link row, the pane rows with
/// their enable boxes and locks, and after a gap the switch rows. A
/// [`List`](craft_ui::list::List) of [`ListRow`](craft_ui::list::ListRow)s with a leading
/// [`ToggleColumn::check_box`](craft_ui::list::ToggleColumn::check_box) and a lock column
/// [`at_end`](craft_ui::list::ToggleColumn::at_end) and
/// [`always_visible`](craft_ui::list::ToggleColumn::always_visible).
///
/// - **Look.** the section list width (150) wide, rows the section row (23) high; the
///   enable box, the lock and the names are the list row's own, truncated with a tooltip, Brush
///   Tip Shape in the strong weight; 4 pt after the first row and the section switch gap (6)
///   before the first switch row, as spacer rows that take room and have no AccessKit node; the
///   link row (with the brush icon) and a gap above the sections. Selected row the selected,
///   hover the hover. The link row and the switch rows can't be selected: keys move the lead
///   over them without selecting.
/// - **Pointer.** A click on a pane row reports `Select` and, if its box is clear, `Enable { on:
///   true }`; the box toggles only the flag; the lock toggles only the lock; a click on a switch row
///   toggles it (`Switch`). A press on the row (not its lock) shows the box pressed.
/// - **Keys.** One Tab stop. The list's own keys (Up, Down, Home, End, Page Up and Page Down) move
///   the lead; onto a pane row it also reports `Select`. Space toggles the lead's box or switch
///   (rows that can't be selected have no list key for it); Shift+Space its lock; Enter selects a
///   pane row and turns it on.
/// - **AccessKit.** A list named "Brush settings sections"; a pane row is a `Button` with
///   `selected`; its box a `CheckBox` "Enable {name}"; its lock a `Button` with `toggled` named
///   "Lock: {name}"; a switch row one `CheckBox`.
/// - **RTL.** Mirrors: the box at the reading start, the lock at the reading end.
#[must_use = "show the list with `show`"]
pub struct BrushSectionList<'a, K> {
    id_salt: IdSalt,
    sections: &'a [BrushSection<K>],
    selected: Option<&'a K>,
    link: bool,
    labels: Option<BrushSettingsLabels>,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> BrushSectionList<'a, K> {
    /// The list of `sections`, in order (pane rows first, then switch rows).
    pub fn new(id_salt: impl AsIdSalt, sections: &'a [BrushSection<K>]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), sections, selected: None, link: false, labels: None }
    }

    /// The pane shown: its row draws selected.
    pub fn selected(mut self, selected: Option<&'a K>) -> Self {
        self.selected = selected;
        self
    }

    /// Shows the "Brushes" link row at the top (Photoshop; reports `OpenLink`).
    pub fn link(mut self, link: bool) -> Self {
        self.link = link;
        self
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushSettingsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the list.
    pub fn show(self, ui: &mut Ui) -> BrushSectionListResponse<K> {
        ui.scope(|ui| {
            ui.set_width(BrushSettingsStyle::of(&Tokens::of(ui)).section_list_width);
            self.show_in(ui)
        })
        .inner
    }

    fn show_in(self, ui: &mut Ui) -> BrushSectionListResponse<K> {
        // pc: crates/ui-egui/src/panels/brush_panel.rs:247-349 `section_list` (row 23, box 13 at +6, names at +20, lock 18 at the end, 6 pt gap, link row)
        let Self { id_salt, sections, selected, link, labels } = self;
        let labels = labels.map(Arc::new).unwrap_or_else(|| BrushSettingsLabels::shared(ui));
        let tokens = Tokens::of(ui);
        let style = BrushSettingsStyle::of(&tokens);
        let gap = tokens.space.gap.inline;
        let slots = Slot::layout(sections, link, gap, style.section_switch_gap);
        let order: Vec<RowInfo<usize>> =
            slots.iter().enumerate().map(|(at, slot)| if matches!(slot, Slot::Gap(_)) { RowInfo::section(at) } else { RowInfo::leaf(at) }).collect();
        let heights: Vec<f32> = slots.iter().map(|slot| if let Slot::Gap(height) = slot { *height } else { style.section_row }).collect();
        let total: f32 = heights.iter().sum();
        let shown = selected.and_then(|key| {
            slots.iter().position(|slot| matches!(slot, Slot::Section(i) if sections[*i].key == *key && sections[*i].kind == BrushSectionKind::Pane))
        });

        let list_salt = (&id_salt, "list");
        let list_id = List::<(), [()]>::id_in(ui, list_salt);
        let lead_id = list_id.with("brush-section-lead");
        let remembered = ui.data(|d| d.get_temp::<Option<usize>>(lead_id)).flatten();
        let is_row = |at: &usize| slots.get(*at).is_some_and(|slot| !matches!(slot, Slot::Gap(_)));
        let first_row = slots.iter().position(|slot| !matches!(slot, Slot::Gap(_)));
        let lead = remembered.filter(is_row).or(shown).or(first_row);

        let mut events = Vec::new();
        if ui.is_enabled()
            && ui.memory(|m| m.has_focus(list_id))
            && let Some(command) = Command::take(ui)
            && let Some(Slot::Section(i)) = lead.map(|at| slots[at])
        {
            let section = &sections[i];
            let (key, on) = (section.key.clone(), section.enabled.unwrap_or(false));
            match (command, section.kind, section.locked) {
                (Command::Toggle, BrushSectionKind::Switch, _) => events.push(BrushSectionEvent::Switch { key, on: !on }),
                (Command::Toggle, BrushSectionKind::Pane, _) if section.enabled.is_some() => {
                    events.push(BrushSectionEvent::Enable { key, on: !on });
                }
                (Command::Lock, BrushSectionKind::Pane, Some(locked)) => events.push(BrushSectionEvent::Lock { key, on: !locked }),
                _ => {}
            }
        }

        let enable_name = labels.enable.replace("{name}", "{row}");
        let lock_name = labels.lock.replace("{name}", "{row}");
        let mut selection = Selection { items: shown.into_iter().collect(), lead, anchor: lead };
        let (response, list_events) = {
            {
                let list = List::new(list_salt, labels.sections.clone(), order.as_slice())
                    .selection(&mut selection)
                    .mode(craft_ui::behavior::SelectionMode::Single)
                    .allow_empty(false)
                    .heights(|at| heights[at])
                    .max_height(total.min(ui.available_height()));
                let out = list.show(ui, |at, &key| match slots[at] {
                    Slot::Link => ListRow::new(key, labels.brushes.clone()).role(Role::Button).icon(Icon::Brush).selectable(false).rule(true),
                    Slot::Gap(_) => ListRow::spacer(key),
                    Slot::Section(i) => {
                        let section = &sections[i];
                        let on = section.enabled.unwrap_or(false);
                        let mut row = ListRow::new(key, section.name.clone()).strong(i == 0 && section.kind == BrushSectionKind::Pane);
                        match section.kind {
                            BrushSectionKind::Switch => {
                                row = row.selectable(false);
                                let state = if on { CheckState::Checked } else { CheckState::Unchecked };
                                row = row.toggle(ToggleColumn::check_box("{row}", state));
                            }
                            BrushSectionKind::Pane => {
                                row = row.role(Role::Button);
                                if let Some(on) = section.enabled {
                                    let state = if on { CheckState::Checked } else { CheckState::Unchecked };
                                    row = row.toggle(ToggleColumn::check_box(enable_name.clone(), state));
                                }
                                if let Some(locked) = section.locked {
                                    let tip = if locked { &labels.unlock_tip } else { &labels.lock_tip };
                                    row = row
                                        .toggle(ToggleColumn::lock(lock_name.clone(), locked).at_end(true).always_visible(true).tooltip(tip.clone()));
                                }
                            }
                        }
                        row
                    }
                });
                (out.response, out.events)
            }
        };
        let mut clicked = None;
        for event in list_events {
            match event {
                ListEvent::Selection(change) => {
                    for at in change.selection.items {
                        if let Some(Slot::Section(i)) = slots.get(at).copied()
                            && sections[i].kind == BrushSectionKind::Pane
                            && shown != Some(at)
                        {
                            events.push(BrushSectionEvent::Select(sections[i].key.clone()));
                        }
                    }
                }
                ListEvent::Row(RowEvent::Toggle(EditEvent::Update { value, .. } | EditEvent::Set { value, .. })) => {
                    let Some(Slot::Section(i)) = slots.get(value.row).copied() else { continue };
                    let section = &sections[i];
                    let key = section.key.clone();
                    let box_column = section.enabled.is_some().then_some(0);
                    let lock_column =
                        (section.kind == BrushSectionKind::Pane && section.locked.is_some()).then_some(usize::from(box_column.is_some()));
                    if Some(value.column) == box_column {
                        events.push(match section.kind {
                            BrushSectionKind::Pane => BrushSectionEvent::Enable { key, on: value.on },
                            BrushSectionKind::Switch => BrushSectionEvent::Switch { key, on: value.on },
                        });
                    } else if Some(value.column) == lock_column {
                        events.push(BrushSectionEvent::Lock { key, on: value.on });
                    }
                }
                ListEvent::Clicked(at) | ListEvent::Activate(at) if clicked != Some(at) => {
                    clicked = Some(at);
                    match slots.get(at).copied() {
                        Some(Slot::Link) => events.push(BrushSectionEvent::OpenLink),
                        Some(Slot::Section(i)) => {
                            let section = &sections[i];
                            let key = section.key.clone();
                            match section.kind {
                                BrushSectionKind::Pane => {
                                    let select = BrushSectionEvent::Select(key.clone());
                                    if !events.contains(&select) {
                                        events.push(select);
                                    }
                                    if section.is_off() {
                                        events.push(BrushSectionEvent::Enable { key, on: true });
                                    }
                                }
                                BrushSectionKind::Switch => {
                                    events.push(BrushSectionEvent::Switch { key, on: !section.enabled.unwrap_or(false) });
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        ui.data_mut(|d| d.insert_temp(lead_id, selection.lead));
        BrushSectionListResponse { response, events }
    }
}

/// One row of the laid-out list.
#[derive(Clone, Copy)]
enum Slot {
    /// The "Brushes" link row.
    Link,
    /// Empty space of this height (a spacer row in a section position, so keys and clicks skip it).
    Gap(f32),
    /// The section at this index.
    Section(usize),
}

impl Slot {
    /// The link row and its gap, then the sections with a gap after the first row and one before
    /// the first switch row.
    fn layout<K>(sections: &[BrushSection<K>], link: bool, gap: f32, switch_gap: f32) -> Vec<Slot> {
        let mut slots = Vec::new();
        if link {
            slots.push(Slot::Link);
            slots.push(Slot::Gap(gap));
        }
        let first_switch = sections.iter().position(|s| s.kind == BrushSectionKind::Switch);
        for at in 0..sections.len() {
            if first_switch == Some(at) && at > 0 {
                slots.push(Slot::Gap(switch_gap));
            }
            slots.push(Slot::Section(at));
            if at == 0 && sections.len() > 1 && first_switch != Some(1) {
                slots.push(Slot::Gap(gap));
            }
        }
        slots
    }
}

/// The keys the list reads itself: the list has no key that toggles a row it can't select.
#[derive(Clone, Copy)]
enum Command {
    /// Space: the lead's box or switch.
    Toggle,
    /// Shift+Space: the lead's lock.
    Lock,
}

impl Command {
    /// Takes the first of these keys pressed this pass.
    fn take(ui: &Ui) -> Option<Command> {
        ui.input_mut(|i| {
            // Shift+Space first: Space alone also matches it.
            let keys = [(Modifiers::SHIFT, Key::Space, Command::Lock), (Modifiers::NONE, Key::Space, Command::Toggle)];
            keys.into_iter().find_map(|(modifiers, key, command)| i.consume_key(modifiers, key).then_some(command))
        })
    }
}

/// What [`BrushSectionList::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct BrushSectionListResponse<K> {
    /// The list.
    pub response: Response,
    /// This pass's changes, in order.
    pub events: Vec<BrushSectionEvent<K>>,
}
