//! `LayersPanel` input: eye and lock gestures, menus, renaming, keys, drops and the parts of a row.

use craft_behavior::{EditEvent, Mixed, SelectionChange};
use craft_i18n::Direction;
use craft_layers::ui::mock::FakeLayers;
use craft_layers::ui::options::LayersSettings;
use craft_layers::ui::panel::{
    ActivateOn, LayerColumn, LayerDrop, LayerPosition, LayerToggle, LayersEvent, LayersFooterButton, LayersPanel, LayersPanelCommands,
};
use craft_layers::ui::tree::{LayerInfo, LayerKind, LayerLocks, LayerPart, LayerRowKey, LayerSelection, MaskInfo};
use craft_theme::LabelColor;
use craft_ui::contract::{Entry, Role};
use craft_ui::list::ThumbSize;
use craft_ui::testing::{Fixture, Target, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The app around the panel: the tree it shows, what it reported, and the rename the row menu asked for.
struct App {
    tree: FakeLayers,
    settings: LayersSettings,
    lock_column: bool,
    events: Vec<LayersEvent<u32>>,
    rename: Option<u32>,
}

/// A size of `width` by `height` points.
const fn by(width: f32, height: f32) -> Vec2 {
    Vec2::new(width, height)
}

/// The room of the panel: wide enough for a row's parts, tall enough for the sample tree and the footer, and inside the
/// 800 by 600 screen of a fixture.
const PANEL: Vec2 = by(420.0, 560.0);

/// A pointer move that starts a drag.
const DRAG_STEP: Vec2 = by(0.0, 8.0);

type Shared = Rc<RefCell<App>>;

/// Applies what the panel reported to the tree, as an app would.
fn apply(tree: &mut FakeLayers, event: &LayersEvent<u32>) {
    match event {
        LayersEvent::Select(select) => {
            let part = select.part.unwrap_or(LayerPart::Pixels);
            match &select.change {
                SelectionChange::Set(layers) => tree.selection = LayerSelection::new(layers.clone(), layers.last().copied(), part),
                SelectionChange::Toggle(layer) => {
                    if tree.selection.layers.contains(layer) {
                        tree.selection.layers.retain(|held| held != layer);
                    } else {
                        tree.selection.layers.push(*layer);
                    }
                    tree.selection.active = Some(*layer);
                }
                SelectionChange::Clear => tree.selection = LayerSelection::default(),
                _ => {}
            }
        }
        LayersEvent::Open(change) => {
            for layer in tree.layers.iter_mut().filter(|layer| layer.id == change.key || (change.all && layer.info.kind == LayerKind::Group)) {
                layer.open = change.open;
            }
        }
        LayersEvent::FxOpen { layer, open, all, .. } => {
            for held in tree.layers.iter_mut().filter(|held| held.id == *layer || *all) {
                held.effects_open = *open;
            }
        }
        _ => {}
    }
}

fn show(ui: &mut Ui, app: &mut App) {
    let App { tree, settings, lock_column, events, rename } = app;
    let mut panel = LayersPanel::new(&*tree)
        .settings(&*settings)
        .lock_column(*lock_column)
        .row_menu(|_| vec![Entry::new(String::from("renameLayer"), String::from("Rename Layer…"))]);
    if let Some(layer) = rename.take() {
        panel = panel.start_rename(layer);
    }
    let out = panel.show(ui);
    for event in &out.events {
        apply(tree, event);
        if let LayersEvent::Menu { target, command, .. } = event
            && command == "renameLayer"
        {
            *rename = Some(*target.row.layer());
        }
    }
    events.extend(out.events);
}

/// The panel over `tree` after `setup` changed the app, in a room tall enough for every row.
fn rig_with(tree: FakeLayers, setup: impl FnOnce(&mut App)) -> (Fixture, Shared) {
    let app = Rc::new(RefCell::new(App { tree, settings: LayersSettings::default(), lock_column: false, events: Vec::new(), rename: None }));
    setup(&mut app.borrow_mut());
    let shared = Rc::clone(&app);
    let mut fixture = Fixture::new(move |ui| show(ui, &mut shared.borrow_mut()));
    fixture.room(PANEL);
    fixture.settle();
    (fixture, app)
}

fn rig(tree: FakeLayers) -> (Fixture, Shared) {
    rig_with(tree, |_| {})
}

fn take(app: &Shared) -> Vec<LayersEvent<u32>> {
    std::mem::take(&mut app.borrow_mut().events)
}

fn select_layers(fixture: &mut Fixture, app: &Shared, layers: &[u32]) {
    app.borrow_mut().tree.selection = LayerSelection::new(layers.to_vec(), layers.first().copied(), LayerPart::Pixels);
    fixture.idle(2);
}

/// The node named `label`.
fn node(fixture: &Fixture, label: &str) -> Target {
    fixture.by_label(label).unwrap_or_else(|| {
        let names: Vec<_> = fixture.tree().keys().into_iter().filter_map(|key| fixture.tree().name_of(key)).collect();
        panic!("no node named {label:?}; the tree has {names:?}")
    })
}

/// The list row named `name`.
fn row(fixture: &Fixture, name: &str) -> Target {
    node(fixture, name)
}

/// The middle of the painted name of a row.
fn name_at(fixture: &Fixture, name: &str) -> Pos2 {
    fixture.texts().iter().find(|text| text.text == name).unwrap_or_else(|| panic!("{name:?} is not painted")).rect.center()
}

/// The point `share` of the way down `target`, in the middle of its width.
fn at(target: &Target, share: f32) -> Pos2 {
    Pos2::new(target.rect.center().x, target.rect.top() + target.rect.height() * share)
}

/// Right-click on `target`, then let the menu open.
fn context_click(fixture: &mut Fixture, target: &Target) {
    let pos = target.center();
    fixture.frame(vec![pointer::moved(pos)]);
    fixture.frame(vec![pointer::secondary(pos, true)]);
    fixture.frame(vec![pointer::secondary(pos, false)]);
    fixture.idle(3);
}

fn double_click(fixture: &mut Fixture, pos: Pos2) {
    fixture.frame(vec![pointer::moved(pos)]);
    for _ in 0..2 {
        fixture.frame(vec![pointer::press(pos)]);
        fixture.frame(vec![pointer::release(pos)]);
    }
    fixture.idle(2);
}

/// Press at `from`, move past the drag threshold and on to `to`, release there.
fn drag_to(fixture: &mut Fixture, from: Pos2, to: Pos2) {
    fixture.idle(6);
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    fixture.frame(vec![pointer::moved(from + DRAG_STEP)]);
    fixture.frame(vec![pointer::moved(to)]);
    fixture.frame(vec![pointer::moved(to)]);
    fixture.frame(vec![pointer::release(to)]);
    fixture.idle(2);
}

fn toggles(events: &[LayersEvent<u32>]) -> Vec<&LayerToggle<u32>> {
    events.iter().filter_map(|event| if let LayersEvent::Toggle(toggle) = event { Some(toggle) } else { None }).collect()
}

fn selects(events: &[LayersEvent<u32>]) -> Vec<SelectionChange<u32>> {
    events.iter().filter_map(|event| if let LayersEvent::Select(select) = event { Some(select.change.clone()) } else { None }).collect()
}

fn set(layers: &[u32]) -> SelectionChange<u32> {
    SelectionChange::Set(layers.to_vec())
}

fn opens(events: &[LayersEvent<u32>]) -> Vec<(u32, bool, bool)> {
    events.iter().filter_map(|event| if let LayersEvent::Open(change) = event { Some((change.key, change.open, change.all)) } else { None }).collect()
}

fn fx_opens(events: &[LayersEvent<u32>]) -> Vec<(u32, bool, bool)> {
    events
        .iter()
        .filter_map(|event| if let LayersEvent::FxOpen { layer, open, all, .. } = event { Some((*layer, *open, *all)) } else { None })
        .collect()
}

fn renames(events: &[LayersEvent<u32>]) -> Vec<(u32, String)> {
    events.iter().filter_map(|event| if let LayersEvent::Rename { layer, text, .. } = event { Some((*layer, text.clone())) } else { None }).collect()
}

fn label_colors(events: &[LayersEvent<u32>]) -> Vec<(Vec<u32>, Option<LabelColor>)> {
    events
        .iter()
        .filter_map(|event| if let LayersEvent::LabelColor { rows, color, .. } = event { Some((rows.clone(), *color)) } else { None })
        .collect()
}

fn mask_links(events: &[LayersEvent<u32>]) -> Vec<(u32, LayerPart, bool)> {
    events
        .iter()
        .filter_map(|event| if let LayersEvent::MaskLink { layer, part, linked, .. } = event { Some((*layer, *part, *linked)) } else { None })
        .collect()
}

fn activations(events: &[LayersEvent<u32>]) -> Vec<(LayerRowKey<u32>, ActivateOn)> {
    events.iter().filter_map(|event| if let LayersEvent::Activate { row, on, .. } = event { Some((row.clone(), *on)) } else { None }).collect()
}

fn menus(events: &[LayersEvent<u32>]) -> Vec<LayerRowKey<u32>> {
    events.iter().filter_map(|event| if let LayersEvent::Menu { target, .. } = event { Some(target.row.clone()) } else { None }).collect()
}

fn footer_drops(events: &[LayersEvent<u32>]) -> Vec<(LayersFooterButton, Vec<LayerRowKey<u32>>)> {
    events
        .iter()
        .filter_map(|event| if let LayersEvent::FooterDrop { button, rows, .. } = event { Some((*button, rows.clone())) } else { None })
        .collect()
}

/// The layers, target, place and copy flag of the first layer drop reported.
fn layer_drop(events: &[LayersEvent<u32>]) -> Option<(Vec<u32>, u32, LayerPosition, bool)> {
    events.iter().find_map(|event| match event {
        LayersEvent::Drop(EditEvent::Set { after, .. } | EditEvent::Commit { after, .. }) => match after {
            LayerDrop::Layers(mv) => Some((mv.layers.clone(), mv.target, mv.position, mv.copy)),
            _ => None,
        },
        _ => None,
    })
}

/// A sweep down the eyes is one gesture that reports each row it crosses, in order, with the first row's new state.
#[trial]
fn a_sweep_down_the_eyes_is_one_gesture() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    let (start, end) = (node(&fixture, "Visibility Title").center(), node(&fixture, "Visibility Smart Object").center());
    fixture.frame(vec![pointer::moved(start)]);
    fixture.frame(vec![pointer::press(start)]);
    for step in [0.3, 0.6, 1.0] {
        fixture.frame(vec![pointer::moved(start + (end - start) * step)]);
    }
    fixture.frame(vec![pointer::release(end)]);
    fixture.idle(2);
    let events = take(&app);
    let sweep = toggles(&events);
    assert_eq!(sweep.len(), events.len(), "nothing but toggles: {events:?}");
    let rows: Vec<_> = sweep.iter().flat_map(|toggle| toggle.rows.clone()).collect();
    let crossed = [LayerRowKey::Layer(4), LayerRowKey::Layer(5), LayerRowKey::Layer(6)];
    assert_eq!(rows, crossed, "every row crossed, in order: {events:?}");
    assert!(sweep.iter().all(|toggle| toggle.column == LayerColumn::Visibility), "{events:?}");
    assert!(sweep.iter().all(|toggle| matches!(toggle.edit, EditEvent::Set { after: false, .. })), "the first row's new state: {events:?}");
    let ids: Vec<_> = sweep.iter().map(|toggle| toggle.edit.id()).collect();
    assert!(ids.windows(2).all(|pair| pair[0] == pair[1]), "one gesture: {ids:?}");
}

/// A pick in the eye's colour menu colours every selected layer, and a row outside the selection alone.
#[trial]
fn the_eye_colour_menu_colours_every_selected_layer() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    select_layers(&mut fixture, &app, &[4, 5]);
    context_click_on(&mut fixture, "Visibility Title");
    let opened = app.borrow().events.clone();
    fixture.click(&node(&fixture, "Red"));
    fixture.idle(2);
    let events = take(&app);
    assert_eq!(label_colors(&events), [(vec![4, 5], Some(LabelColor::Red))], "events at the menu: {opened:?}, after the pick: {events:?}");

    context_click_on(&mut fixture, "Visibility Hidden");
    fixture.click(&node(&fixture, "Blue"));
    fixture.idle(2);
    let events = take(&app);
    assert_eq!(label_colors(&events), [(vec![8], Some(LabelColor::Blue))], "a row outside the selection");
    assert!(selects(&events).is_empty(), "the colour menu does not select: {events:?}");
}

/// The eye menu checks the colour the selected layers share, and none when they differ.
#[trial]
fn the_eye_menu_checks_only_a_colour_the_rows_share() {
    let mut tree = FakeLayers::sample();
    tree.layer_mut(4).info.label = Some(LabelColor::Red);
    tree.layer_mut(5).info.label = Some(LabelColor::Red);
    let (mut fixture, app) = rig(tree);
    select_layers(&mut fixture, &app, &[4, 5]);
    let checked = |fixture: &Fixture| matches!(fixture.tree().toggled_of(node(fixture, "Red").node), Some(Mixed::Same(true)));
    context_click_on(&mut fixture, "Visibility Title");
    assert!(checked(&fixture), "the shared colour is checked");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(3);

    app.borrow_mut().tree.layer_mut(5).info.label = Some(LabelColor::Blue);
    fixture.idle(2);
    context_click_on(&mut fixture, "Visibility Title");
    assert!(!checked(&fixture), "differing colours: none is checked");
}

/// A right-click on the eye of an effects row opens the row menu for that row; only a layer's eye opens the colour menu.
#[trial]
fn a_right_click_on_an_effects_eye_opens_the_row_menu_for_that_row() {
    let (mut fixture, app) = rig_with(FakeLayers::sample(), |app| app.tree.layer_mut(7).effects_open = true);
    context_click_on(&mut fixture, "Visibility Drop Shadow");
    assert!(fixture.by_label("Red").is_none(), "no colour menu");
    fixture.click(&node(&fixture, "Rename Layer…"));
    fixture.idle(3);
    assert_eq!(menus(&take(&app)), [LayerRowKey::Effects(7, Some(0))]);
}

/// A click on the lock indicator reports the lock turned off, and selects nothing.
#[trial]
fn the_lock_indicator_unlocks_without_selecting() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    fixture.click(&node(&fixture, "Unlock Background"));
    fixture.idle(2);
    let events = take(&app);
    let [LayersEvent::Toggle(toggle)] = events.as_slice() else { panic!("one toggle and no select: {events:?}") };
    assert_eq!((&toggle.rows[..], toggle.column), (&[LayerRowKey::Layer(9)][..], LayerColumn::Lock));
    assert!(matches!(&toggle.edit, EditEvent::Set { after: false, .. }), "{:?}", toggle.edit);
}

/// A click on a mask's chain asks for the opposite link state.
#[trial]
fn a_click_on_a_mask_chain_reports_the_link_change() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    fixture.click(&node(&fixture, "Unlink layer mask of Masked"));
    fixture.idle(2);
    assert!(mask_links(&take(&app)).contains(&(3, LayerPart::LayerMask, false)));

    app.borrow_mut().tree.layer_mut(3).info.masks[0] = MaskInfo::new(LayerPart::LayerMask, true, false);
    fixture.idle(2);
    fixture.click(&node(&fixture, "Link layer mask of Masked"));
    fixture.idle(2);
    assert!(mask_links(&take(&app)).contains(&(3, LayerPart::LayerMask, true)));
}

/// Opens the rename field of `name` by a double click on it.
fn begin_rename(fixture: &mut Fixture, app: &Shared, name: &str) {
    let pos = name_at(fixture, name);
    double_click(fixture, pos);
    assert!(!fixture.by_role(Role::TEXT_INPUT).is_empty(), "{name} is being renamed");
    take(app);
}

/// Replaces the text of the rename field.
fn retype(fixture: &mut Fixture, text: &str) {
    fixture.key(Key::A, Modifiers::COMMAND);
    if text.is_empty() {
        fixture.key(Key::Backspace, Modifiers::NONE);
    } else {
        fixture.type_text(text);
    }
}

/// Enter and Tab commit the trimmed name, Esc cancels, and an empty or unchanged name reports nothing.
#[trial]
fn rename_rules_enter_tab_escape_empty_unchanged() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    begin_rename(&mut fixture, &app, "Title");
    retype(&mut fixture, "  Heading ");
    fixture.key(Key::Enter, Modifiers::NONE);
    let events = take(&app);
    assert_eq!(renames(&events), [(4, String::from("Heading"))], "Enter commits the trimmed text; events: {events:?}");
    assert!(fixture.by_role(Role::TEXT_INPUT).is_empty());

    begin_rename(&mut fixture, &app, "Title");
    retype(&mut fixture, "Tabbed");
    fixture.key(Key::Tab, Modifiers::NONE);
    assert_eq!(renames(&take(&app)), [(4, String::from("Tabbed"))], "Tab commits");
    assert!(!fixture.by_role(Role::TEXT_INPUT).is_empty(), "and continues on the next renamable row");
    fixture.key(Key::Escape, Modifiers::NONE);
    assert!(renames(&take(&app)).is_empty(), "Esc cancels the continued edit");
    assert!(fixture.by_role(Role::TEXT_INPUT).is_empty());

    begin_rename(&mut fixture, &app, "Title");
    retype(&mut fixture, "Discarded");
    fixture.key(Key::Escape, Modifiers::NONE);
    assert!(renames(&take(&app)).is_empty(), "Esc reports nothing");
    assert!(fixture.by_role(Role::TEXT_INPUT).is_empty());

    begin_rename(&mut fixture, &app, "Title");
    retype(&mut fixture, "");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert!(renames(&take(&app)).is_empty(), "an empty name reports nothing");

    begin_rename(&mut fixture, &app, "Title");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert!(renames(&take(&app)).is_empty(), "an unchanged name reports nothing");
}

/// The arrow keys walk the rows, Left and Right close and open a group, Enter activates.
#[trial]
fn keys_walk_rows_open_groups_and_activate() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    fixture.click(&row(&fixture, "Title"));
    take(&app);
    let mut walked = Vec::new();
    for key in [Key::ArrowDown, Key::ArrowDown, Key::ArrowUp, Key::Home] {
        fixture.key(key, Modifiers::NONE);
        walked.extend(selects(&take(&app)));
    }
    assert_eq!(walked, [set(&[5]), set(&[6]), set(&[5]), set(&[1])]);

    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    assert!(opens(&take(&app)).contains(&(1, false, false)), "Left closes the group");
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert!(opens(&take(&app)).contains(&(1, true, false)), "Right opens it");

    fixture.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(selects(&take(&app)), [set(&[2])], "down goes into the opened group");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert!(activations(&take(&app)).contains(&(LayerRowKey::Layer(2), ActivateOn::Row)));
}

/// The effects commands act on the active layer when it has effects; Alt on the disclosure reaches every layer.
#[trial]
fn effects_commands_and_the_disclosure_report_fx_open() {
    let mut tree = FakeLayers::sample();
    tree.selection = LayerSelection::new(vec![7], Some(7), LayerPart::Pixels);
    let commands = [LayersPanelCommands::ShowEffects, LayersPanelCommands::ShowAllEffects, LayersPanelCommands::HideEffects];
    let events: Vec<_> = commands.into_iter().filter_map(|command| command.event(&tree, "Effects")).collect();
    assert_eq!(fx_opens(&events), [(7, true, false), (7, true, true), (7, false, false)]);

    tree.selection = LayerSelection::new(vec![4], Some(4), LayerPart::Pixels);
    assert!(commands.into_iter().all(|command| command.event(&tree, "Effects").is_none()), "a layer without effects reports nothing");

    let (mut fixture, app) = rig(FakeLayers::sample());
    fixture.click(&node(&fixture, "Expand effects Styled"));
    assert!(fx_opens(&take(&app)).contains(&(7, true, false)));
    fixture.hold(Modifiers::ALT);
    fixture.click(&node(&fixture, "Collapse effects Styled"));
    assert!(fx_opens(&take(&app)).contains(&(7, false, true)), "Alt reaches every layer");
}

/// Alt-click on a group's disclosure closes or opens every group.
#[trial]
fn alt_click_on_a_group_disclosure_reaches_every_group() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    fixture.hold(Modifiers::ALT);
    fixture.click(&node(&fixture, "Collapse Group 1"));
    let events = take(&app);
    assert!(opens(&events).contains(&(1, false, true)), "events: {events:?}");
}

/// A click on the empty space below the last row clears the selection.
#[trial]
fn a_click_below_the_rows_clears_the_selection() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    let last = row(&fixture, "Background");
    let below = Pos2::new(last.rect.center().x, last.rect.bottom() + 20.0);
    let list = fixture.by_role(Role::LIST).into_iter().find(|list| list.rect.contains(below));
    assert!(list.is_some(), "there is empty space under the rows");
    fixture.click_at(below);
    fixture.idle(2);
    assert_eq!(selects(&take(&app)), [SelectionChange::Clear]);
}

/// Where a layer dragged from `from` and released at `to` lands.
fn dropped(fixture: &mut Fixture, app: &Shared, from: Pos2, to: Pos2) -> Option<(Vec<u32>, u32, LayerPosition, bool)> {
    take(app);
    drag_to(fixture, from, to);
    layer_drop(&take(app))
}

/// A drop is reported one way: into an open group at its top, else before the row it lands above, and after the last row only
/// when nothing follows. The middle band of a group row and the lower part of its header both land at the top of the group.
#[trial]
fn drop_zones_into_before_and_after_the_last_row() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    let from = row(&fixture, "Hidden").center();
    let group = row(&fixture, "Group 1");
    for (share, target, position) in [(0.5, 1, LayerPosition::Into), (0.15, 1, LayerPosition::Before), (0.85, 1, LayerPosition::Into)] {
        let to = at(&group, share);
        assert_eq!(dropped(&mut fixture, &app, from, to), Some((vec![8], target, position, false)), "Group 1 at {share}");
    }
    let title = row(&fixture, "Title");
    let to = at(&title, 0.75);
    assert_eq!(
        dropped(&mut fixture, &app, from, to),
        Some((vec![8], 5, LayerPosition::Before, false)),
        "the lower half of Title lands above the next row"
    );
    let last = row(&fixture, "Background");
    let below = Pos2::new(last.rect.center().x, last.rect.bottom() + 20.0);
    assert_eq!(dropped(&mut fixture, &app, from, below), Some((vec![8], 9, LayerPosition::After, false)), "the space below the last row");
}

/// Alt during a drag copies the layer instead of moving it.
#[trial]
fn an_alt_drag_copies() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    let from = row(&fixture, "Hidden").center();
    let to = at(&row(&fixture, "Title"), 0.75);
    fixture.hold(Modifiers::ALT);
    assert_eq!(dropped(&mut fixture, &app, from, to), Some((vec![8], 5, LayerPosition::Before, true)));
}

/// How far right of a mask's chain its thumbnail starts to hold the pointer.
const INTO_THUMBNAIL: f32 = 4.0;

/// Layer rows, effects rows and a mask dropped on the footer buttons report a footer drop; New Layer and New Group refuse effects and masks.
#[trial]
fn rows_effects_and_masks_dropped_on_footer_buttons() {
    let (mut fixture, app) = rig_with(FakeLayers::sample(), |app| app.tree.layer_mut(7).effects_open = true);
    let button = |fixture: &Fixture, label: &str| node(fixture, label).center();
    let (delete, new_layer, new_group) = ("Delete Layer", "Create a new layer", "Create a new group");
    let drop_on = |fixture: &mut Fixture, from: Pos2, label: &str| {
        take(&app);
        let to = button(fixture, label);
        drag_to(fixture, from, to);
        footer_drops(&take(&app))
    };

    let title = row(&fixture, "Title").center();
    for (label, which) in [(delete, LayersFooterButton::Delete), (new_layer, LayersFooterButton::NewLayer), (new_group, LayersFooterButton::NewGroup)]
    {
        assert_eq!(drop_on(&mut fixture, title, label), [(which, vec![LayerRowKey::Layer(4)])], "a layer row on {label}");
    }

    let effect = row(&fixture, "Drop Shadow").center();
    assert_eq!(drop_on(&mut fixture, effect, delete), [(LayersFooterButton::Delete, vec![LayerRowKey::Effects(7, Some(0))])]);
    for label in [new_layer, new_group] {
        assert!(drop_on(&mut fixture, effect, label).is_empty(), "an effect on {label}");
    }

    let chain = node(&fixture, "Unlink layer mask of Masked").rect;
    let mask = Pos2::new(chain.max.x + INTO_THUMBNAIL, chain.center().y);
    assert_eq!(drop_on(&mut fixture, mask, delete), [(LayersFooterButton::Delete, vec![LayerRowKey::Mask(3, LayerPart::LayerMask)])]);
    for label in [new_layer, new_group] {
        assert!(drop_on(&mut fixture, mask, label).is_empty(), "a mask on {label}");
    }
}

/// One layer, `edit` applied to its info, active and unlocked.
fn one(name: &str, kind: LayerKind, edit: impl FnOnce(&mut LayerInfo)) -> FakeLayers {
    let mut tree = FakeLayers::sample();
    tree.layers.retain(|layer| layer.id == 4);
    let layer = &mut tree.layers[0];
    layer.parent = None;
    layer.info = LayerInfo::new(name, kind);
    edit(&mut layer.info);
    tree.selection = LayerSelection::new(vec![4], Some(4), LayerPart::Pixels);
    tree
}

/// A layer with both masks, a link, effects, a lock and a blend label.
fn busy() -> FakeLayers {
    one("Busy", LayerKind::Pixel, |info| {
        info.masks = vec![MaskInfo::new(LayerPart::LayerMask, true, true), MaskInfo::new(LayerPart::VectorMask, true, false)];
        info.linked = true;
        info.has_effects = true;
        info.locks = LayerLocks::default();
        info.locks.all = true;
        info.blend_label = Some(String::from("Multiply"));
    })
}

/// A row's parts follow each other in reading order, mirrored right to left: eye, lock column, thumbnails with their chains, name, effects disclosure, lock.
#[trial]
fn row_parts_follow_reading_order_in_both_directions() {
    for direction in [Direction::Ltr, Direction::Rtl] {
        let (mut fixture, _app) = rig_with(busy(), |app| app.lock_column = true);
        fixture.direction(direction).idle(4);
        let start = |rect: Rect| if direction == Direction::Ltr { rect.min.x } else { -rect.max.x };
        let parts = [
            start(node(&fixture, "Visibility Busy").rect),
            start(node(&fixture, "Lock Busy").rect),
            start(node(&fixture, "Unlink layer mask of Busy").rect),
            start(node(&fixture, "Link vector mask of Busy").rect),
            start(fixture.texts().iter().find(|text| text.text == "Busy").expect("the name is painted").rect),
            start(node(&fixture, "Expand effects Busy").rect),
            start(node(&fixture, "Unlock Busy").rect),
        ];
        assert!(parts.windows(2).all(|pair| pair[0] < pair[1]), "{direction:?}: {parts:?}");
    }
}

/// The blend label drops out before the other indicators when the row gets narrow.
#[trial]
fn the_blend_label_drops_before_the_other_indicators() {
    let (mut fixture, _app) = rig(busy());
    let mut shown = Vec::new();
    for width in (6..=39).rev().map(|step| step as f32 * 20.0) {
        fixture.room(Vec2::new(width, PANEL.y));
        fixture.idle(4);
        let has = |word: &str| fixture.texts().iter().any(|text| text.text == word);
        shown.push((width, has("Multiply"), has("fx")));
    }
    assert!(shown.first().is_some_and(|(_, label, fx)| *label && *fx), "both show with room: {shown:?}");
    assert!(shown.iter().all(|(_, label, fx)| !*label || *fx), "the fx mark never drops before the label: {shown:?}");
    assert!(shown.iter().any(|(_, label, fx)| !*label && *fx), "the label drops while the fx mark stays: {shown:?}");
}

/// The shapes painted with the target on `part`, and the fixture.
fn painted_with_target(part: LayerPart) -> (Vec<Rect>, Fixture) {
    let mut tree = busy();
    tree.layers[0].info.locks = LayerLocks::default();
    tree.selection.target = part;
    let (mut fixture, _app) = rig(tree);
    fixture.idle(4);
    (fixture.shape_bounds().to_vec(), fixture)
}

/// The middle of the shapes of `bounds` that `base` lacks.
fn added_centre(bounds: &[Rect], base: &[Rect]) -> Pos2 {
    let added: Vec<Rect> = bounds.iter().filter(|rect| !base.contains(rect)).copied().collect();
    assert!(!added.is_empty(), "the target paints brackets");
    added.into_iter().fold(Rect::NOTHING, |hull, rect| hull.union(rect)).center()
}

/// The active row frames its target thumbnail, and the brackets move from the pixels to each mask in reading order.
#[trial]
fn target_brackets_frame_the_targeted_thumbnail() {
    let (none, _) = painted_with_target(LayerPart::FilterMask);
    let (pixels, fixture) = painted_with_target(LayerPart::Pixels);
    let (layer_mask, _) = painted_with_target(LayerPart::LayerMask);
    let (vector_mask, _) = painted_with_target(LayerPart::VectorMask);
    let chains = [node(&fixture, "Unlink layer mask of Busy").center().x, node(&fixture, "Link vector mask of Busy").center().x];
    let framed = [added_centre(&pixels, &none).x, added_centre(&layer_mask, &none).x, added_centre(&vector_mask, &none).x];
    assert!(framed[0] < chains[0] && chains[0] < framed[1] && framed[1] < chains[1] && chains[1] < framed[2], "{framed:?} around {chains:?}");
}

/// Effects rows sit under their layer in order, no taller than a layer row, with their eye in the eye column of the layers.
#[trial]
fn effect_rows_sit_under_their_layer() {
    let mut tree = FakeLayers::sample();
    tree.selection = LayerSelection::default();
    let (fixture, _app) = rig_with(tree, |app| app.tree.layer_mut(7).effects_open = true);
    let tops: Vec<f32> = ["Styled", "Effects", "Drop Shadow", "Stroke", "Hidden"].iter().map(|name| row(&fixture, name).rect.min.y).collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "the rows follow their layer in order: {tops:?}");
    assert!(row(&fixture, "Drop Shadow").rect.height() <= row(&fixture, "Styled").rect.height(), "an effects row is no taller than its layer");
    let (layer_eye, effect_eye) = (node(&fixture, "Visibility Styled").rect, node(&fixture, "Visibility Drop Shadow").rect);
    assert_eq!(effect_eye.min.x, layer_eye.min.x, "one eye column for layers and effects");
}

/// The heights of the Smart Filters header row and of a filter row at thumbnail size `size`.
fn smart_filter_heights(size: ThumbSize) -> (f32, f32) {
    let mut tree = FakeLayers::sample();
    tree.selection = LayerSelection::default();
    let (fixture, _app) = rig_with(tree, |app| {
        app.tree.layer_mut(6).effects_open = true;
        app.settings.options.thumbnail_size = size;
    });
    (row(&fixture, "Smart Filters").rect.height(), row(&fixture, "Gaussian Blur").rect.height())
}

/// The Smart Filters header carries the filter mask's thumbnail unless thumbnails are off; a filter row never does.
#[trial]
fn the_smart_filters_header_holds_the_shared_mask_thumbnail() {
    let (header, item) = smart_filter_heights(ThumbSize::Medium);
    assert!(header > item, "the header row holds a thumbnail: {header} against {item}");
    let (header, item) = smart_filter_heights(ThumbSize::None);
    assert!((header - item).abs() < 0.5, "no thumbnail at size None: {header} against {item}");
}

/// Right-click on the node named `label`.
fn context_click_on(fixture: &mut Fixture, label: &str) {
    let target = node(fixture, label);
    context_click(fixture, &target);
}
