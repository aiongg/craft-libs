//! The effect list on the dialog's left.

use std::cell::Cell;

use egui::{Key, Modifiers, Ui};

use craft_theme::ControlSize;
use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::controls::{CheckState, IconButton};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, RowEvent, ThumbSize, ToggleColumn};
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Tokens, TokensOf as _};

use super::LayerStyleDialog;
use super::labels::LayerStyleLabels;
use crate::layer_style::{EffectId, EffectKind, EffectSpec, LayerStyleDraft, LayerStylePage, StyleParams};

/// A list action's change to the style, as the dialog's edit queue replays it.
#[derive(Clone, Debug)]
pub(super) enum ListOp {
    /// [`LayerStyleDraft::toggle`].
    Toggle(EffectId),
    /// [`LayerStyleDraft::add_instance`] from `defaults`.
    Add { kind: EffectKind, defaults: StyleParams },
    /// [`LayerStyleDraft::remove_instance`].
    Remove(EffectId),
}

/// What the list asked for in one pass; the dialog applies it to a copy of the draft.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum ListAction {
    /// Show this page (a row click, Enter on the lead row).
    Select(LayerStylePage),
    /// An instance's check box (a click on it, Space on the lead row):
    /// [`LayerStyleDraft::toggle`].
    Toggle(EffectId),
    /// A click on an absent effect's row, or `+`: [`LayerStyleDraft::add_instance`].
    Add(EffectKind),
    /// `−`: [`LayerStyleDraft::remove_instance`].
    Remove(EffectId),
}

/// One row of the laid-out list.
#[derive(Clone, Copy)]
enum Slot {
    /// Styles, Blending Options or an instance.
    Page(LayerStylePage),
    /// Space between the two page rows and the effects.
    Gap,
    /// An effect the layer lacks.
    Absent(EffectKind),
}

/// The keys the list reads besides the kernel list's own (↑, ↓, Home and End move the lead and
/// show the page of a selectable row).
#[derive(Clone, Copy)]
enum Command {
    /// Space: the lead's check box.
    Toggle,
    /// `+`: another instance of the lead's kind.
    Add,
    /// `−`: remove the lead instance.
    Remove,
}

impl Command {
    /// Takes the first of these keys pressed this pass.
    fn take(ui: &Ui) -> Option<Command> {
        ui.input_mut(|i| {
            let keys = [(Key::Space, Command::Toggle), (Key::Plus, Command::Add), (Key::Minus, Command::Remove)];
            keys.into_iter().find_map(|(key, command)| i.consume_key(Modifiers::NONE, key).then_some(command))
        })
    }
}

/// Draws the list, the list width wide: the Styles and Blending Options rows, then one
/// row per instance in `specs` order (several instances of a kind in the style's order), and one
/// dimmed row with an empty check box per kind the layer lacks (the text dimmed, not
/// selectable; a click adds it, turned on, #1357).
///
/// It is a library `List` of `ListRow`s at the list token's row height, the gap
/// after Blending Options a spacer row, each effect row with a check box at the row start, its
/// label in the label (muted, the text muted, while the effect is off, even when its page
/// is selected, #2288), and for kinds that [allow several](EffectKind::allows_several) a `+` icon
/// button (Sm) at the row end and a `−` before it while the kind has more than one instance.
///
/// Keys: one Tab stop; the list's own keys move the lead (↑, ↓, Home, End) and show the page of a
/// selectable row, Enter shows the page of any row; Space toggles the lead's check box (adds an
/// absent effect), `+` and `−` add and remove an instance of the lead's kind (the buttons are not
/// Tab stops). ⌘] and ⌘[ are not used: the order is fixed. AccessKit: a `ListBox` named
/// [`LayerStyleLabels::effects`] of `CheckBox` rows named by the effect, with their positions; a
/// kind with several instances numbers them ([`LayerStyleLabels::instance_name`]), so no two
/// rows, check boxes or buttons share a name; `+` and `−` are buttons named
/// [`LayerStyleLabels::add_instance`] and [`LayerStyleLabels::remove_instance`] after that name.
pub(super) fn effect_list(
    ui: &mut Ui,
    draft: &LayerStyleDraft,
    specs: &[EffectSpec],
    labels: &LayerStyleLabels,
    numbers: &LocaleNumbers,
    enabled: bool,
) -> Option<ListAction> {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:838-938 (the left column: Styles and Blending
    // Options rows, absent rows, instance rows, `+` and `−` at 8 and 22 from the row end).
    // new: the keys (PhotoCraft's rows are mouse only), `+` and `−` as keys on the lead since the
    // buttons are not Tab stops.
    ui.add_enabled_ui(enabled, |ui| show_list(ui, draft, specs, labels, numbers)).inner
}

/// The name of instance `id` of `kind`: the effect's name, numbered when the style has several
/// instances of the kind.
fn instance_name(draft: &LayerStyleDraft, labels: &LayerStyleLabels, numbers: &LocaleNumbers, kind: EffectKind, id: EffectId) -> String {
    let name = labels.effect(kind);
    let place = draft.style.instances(kind).position(|other| other == id);
    match place {
        Some(place) if draft.style.instances(kind).count() > 1 => {
            let number = numbers.format((place + 1) as f64, 0..=0);
            craft_i18n::fmt(&labels.instance_name, &[("effect", name), ("number", number.as_str())])
        }
        _ => name.to_owned(),
    }
}

fn show_list(ui: &mut Ui, draft: &LayerStyleDraft, specs: &[EffectSpec], labels: &LayerStyleLabels, numbers: &LocaleNumbers) -> Option<ListAction> {
    let tokens = Tokens::of(ui);
    let style = LayerStyleDialog::style(&tokens);
    let row_style = ListRow::<usize>::style(&tokens);
    let gap = tokens.space.gap.inline;
    let button_side = craft_ui::controls::IconButton::style(&tokens, ControlSize::Sm).side;

    let mut slots = vec![Slot::Page(LayerStylePage::Styles), Slot::Page(LayerStylePage::Blending), Slot::Gap];
    for spec in specs {
        let before = slots.len();
        slots.extend(draft.style.instances(spec.kind).map(|id| Slot::Page(LayerStylePage::Effect(id))));
        if slots.len() == before {
            slots.push(Slot::Absent(spec.kind));
        }
    }
    let order: Vec<RowInfo<usize>> =
        (0..slots.len()).map(|at| if matches!(slots[at], Slot::Gap) { RowInfo::section(at) } else { RowInfo::leaf(at) }).collect();
    let row_height = row_style.height(ThumbSize::None, false);
    let heights: Vec<f32> = slots.iter().map(|slot| if matches!(slot, Slot::Gap) { gap } else { row_height }).collect();
    let total: f32 = heights.iter().sum();
    let shown = slots.iter().position(|slot| matches!(slot, Slot::Page(page) if *page == draft.page));
    let kind_of = |slot: &Slot| match slot {
        Slot::Page(LayerStylePage::Effect(id)) => draft.style.effect(*id).map(|e| e.kind),
        Slot::Absent(kind) => Some(*kind),
        _ => None,
    };

    let list_id = List::<(), [()]>::id_in(ui, "effects");
    let lead_id = list_id.with("lead");
    let remembered = ui.data(|d| d.get_temp::<usize>(lead_id));
    let is_row = |at: &usize| slots.get(*at).is_some_and(|slot| !matches!(slot, Slot::Gap));
    let lead = remembered.filter(is_row).or(shown).unwrap_or(0);
    let mut action = None;
    if ui.is_enabled()
        && ui.memory(|m| m.has_focus(list_id))
        && let Some(command) = Command::take(ui)
    {
        match command {
            Command::Toggle => {
                action = match slots[lead] {
                    Slot::Page(LayerStylePage::Effect(id)) => Some(ListAction::Toggle(id)),
                    Slot::Absent(kind) => Some(ListAction::Add(kind)),
                    _ => None,
                };
            }
            Command::Add => action = kind_of(&slots[lead]).filter(|kind| kind.allows_several()).map(ListAction::Add),
            Command::Remove => {
                if let Slot::Page(LayerStylePage::Effect(id)) = slots[lead] {
                    let several = kind_of(&slots[lead]).is_some_and(|kind| draft.style.instances(kind).count() > 1);
                    action = several.then_some(ListAction::Remove(id));
                }
            }
        }
    }

    // The buttons report through a cell: the row builders only hold shared borrows.
    let pressed = Cell::new(None::<ListAction>);
    let mut selection = Selection { items: shown.into_iter().collect(), lead: Some(lead), anchor: Some(lead) };
    ui.set_width(style.list_width);
    let out = {
        {
            let list = List::new("effects", labels.effects.as_str(), order.as_slice())
                .selection(&mut selection)
                .mode(SelectionMode::Single)
                .allow_empty(false)
                .heights(|at| heights[at])
                .max_height(total.min(ui.available_height()));
            list.show(ui, |at, &key| {
                let mut row = match slots[at] {
                    Slot::Gap => return ListRow::spacer(key),
                    Slot::Page(LayerStylePage::Styles) => return ListRow::new(key, labels.styles.as_str()),
                    Slot::Page(LayerStylePage::Blending) => return ListRow::new(key, labels.blending_options.as_str()),
                    Slot::Page(LayerStylePage::Effect(id)) => {
                        let Some(instance) = draft.style.effect(id) else { return ListRow::new(key, "") };
                        let state = if instance.on { CheckState::Checked } else { CheckState::Unchecked };
                        let name = instance_name(draft, labels, numbers, instance.kind, id);
                        ListRow::new(key, labels.effect(instance.kind))
                            .accessible_name(name.as_str())
                            .toggle(ToggleColumn::check_box(name, state))
                            .muted(!instance.on)
                    }
                    Slot::Absent(kind) => ListRow::new(key, labels.effect(kind))
                        .toggle(ToggleColumn::check_box(labels.effect(kind), CheckState::Unchecked))
                        .dimmed(true)
                        .selectable(false),
                };
                if let Slot::Page(LayerStylePage::Effect(id)) = slots[at]
                    && let Some(kind) = kind_of(&slots[at]).filter(|kind| kind.allows_several())
                {
                    let name = instance_name(draft, labels, numbers, kind, id);
                    let add_label = craft_i18n::fmt(&labels.add_instance, &[("effect", name.as_str())]);
                    let remove_label = craft_i18n::fmt(&labels.remove_instance, &[("effect", name.as_str())]);
                    let (pressed, side) = (&pressed, button_side);
                    row = row.trailing_item(side, move |ui| {
                        let button = IconButton::new(Icon::Plus, add_label).size(ControlSize::Sm).focusable(false).id_salt(("add", at));
                        if ui.add(button).clicked() {
                            pressed.set(Some(ListAction::Add(kind)));
                        }
                    });
                    if draft.style.instances(kind).count() > 1 {
                        row = row.trailing_item(side, move |ui| {
                            let button = IconButton::new(Icon::Minus, remove_label).size(ControlSize::Sm).focusable(false).id_salt(("remove", at));
                            if ui.add(button).clicked() {
                                pressed.set(Some(ListAction::Remove(id)));
                            }
                        });
                    }
                }
                row
            })
        }
    };

    let mut clicked = None;
    let mut from_list = None;
    for event in out.events {
        match event {
            ListEvent::Row(RowEvent::Toggle(EditEvent::Update { value, .. } | EditEvent::Set { value, .. })) if from_list.is_none() => {
                from_list = match slots.get(value.row).copied() {
                    Some(Slot::Page(LayerStylePage::Effect(id))) => Some(ListAction::Toggle(id)),
                    Some(Slot::Absent(kind)) => Some(ListAction::Add(kind)),
                    _ => None,
                };
            }
            ListEvent::Clicked(at) | ListEvent::Activate(at) if clicked != Some(at) && from_list.is_none() => {
                clicked = Some(at);
                from_list = match slots.get(at).copied() {
                    Some(Slot::Page(page)) => Some(ListAction::Select(page)),
                    Some(Slot::Absent(kind)) => Some(ListAction::Add(kind)),
                    _ => None,
                };
            }
            _ => {}
        }
    }
    // The kernel list selects a selectable row on ↑, ↓, Home and End: that row's page is shown.
    if from_list.is_none()
        && let Some(&at) = selection.items.first()
        && Some(at) != shown
        && let Some(Slot::Page(page)) = slots.get(at).copied()
    {
        from_list = Some(ListAction::Select(page));
    }
    let lead = selection.lead.unwrap_or(lead);
    ui.data_mut(|d| d.insert_temp(lead_id, lead));
    pressed.take().or(action).or(from_list)
}
