//! `LayersPanel` input: eye and lock gestures, menus, renaming, keys, drops and the parts of a row.

use craft_behavior::edit_history::Step;
use craft_behavior::{EditEvent, EditHistory, Mixed, SelectionChange};
use craft_i18n::Direction;
use craft_layers::ui::mock::FakeLayers;
use craft_layers::ui::options::LayersSettings;
use craft_layers::ui::panel::{LayerColumn, LayerDrop, LayerPosition, LayerToggle, LayersEvent, LayersFooterButton, LayersPanel};
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

fn label_colors(events: &[LayersEvent<u32>]) -> Vec<(Vec<u32>, Option<LabelColor>)> {
    events
        .iter()
        .filter_map(|event| if let LayersEvent::LabelColor { rows, color, .. } = event { Some((rows.clone(), *color)) } else { None })
        .collect()
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

/// The eyes of layers 4 to 6 as a host keeps them, with the history that records the toggles it is given.
struct Eyes {
    now: Vec<bool>,
    history: EditHistory<Vec<bool>>,
    /// The state at the open gesture's `Begin`.
    begun: Vec<bool>,
}

impl Eyes {
    fn new() -> Self {
        Eyes { now: vec![true; 3], history: EditHistory::new(&vec![true; 3]), begun: vec![true; 3] }
    }

    /// Applies the toggles among `events`, as an app would.
    fn take(&mut self, events: &[LayersEvent<u32>]) {
        for toggle in toggles(events) {
            let rows: Vec<usize> = toggle.rows.iter().map(|row| *row.layer() as usize - 4).collect();
            match &toggle.edit {
                EditEvent::Begin { id, label, .. } => {
                    self.begun = self.now.clone();
                    self.history.record(&EditEvent::begin(*id, Mixed::Same(self.now.clone()), label.clone(), None));
                }
                EditEvent::Update { value, .. } => rows.iter().for_each(|row| self.now[*row] = *value),
                EditEvent::Set { after, label, .. } => {
                    let before = self.now.clone();
                    rows.iter().for_each(|row| self.now[*row] = *after);
                    self.history.record(&EditEvent::set(Mixed::Same(before), self.now.clone(), label.clone(), None));
                }
                EditEvent::Commit { id, .. } => self.history.record(&EditEvent::commit(*id, Mixed::Same(self.begun.clone()), self.now.clone())),
                EditEvent::Cancel { id, reason, before: Some(Mixed::Same(before)), .. } => {
                    rows.iter().for_each(|row| self.now[*row] = *before);
                    self.history.record(&EditEvent::cancel(*id, *reason, None));
                }
                _ => {}
            }
        }
    }
}

/// A sweep down the eyes is one undo step: one Undo through the host's history restores every row it crossed.
#[trial]
fn a_sweep_down_the_eyes_is_one_undo_step() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    let (start, end) = (node(&fixture, "Visibility Title").center(), node(&fixture, "Visibility Smart Object").center());
    let mut eyes = Eyes::new();
    fixture.frame(vec![pointer::moved(start)]);
    fixture.frame(vec![pointer::press(start)]);
    eyes.take(&take(&app));
    for step in [0.3, 0.6, 1.0] {
        fixture.frame(vec![pointer::moved(start + (end - start) * step)]);
        eyes.take(&take(&app));
    }
    fixture.frame(vec![pointer::release(end)]);
    fixture.idle(2);
    let events = take(&app);
    let sweep = toggles(&events);
    assert!(sweep.iter().all(|toggle| toggle.column == LayerColumn::Visibility), "{events:?}");
    eyes.take(&events);
    assert_eq!(eyes.now, vec![false; 3], "every crossed row is off after the sweep");
    assert!(eyes.history.can_undo(), "the sweep made a step");
    let mut state = eyes.now.clone();
    assert!(eyes.history.step(&mut state, Step::Undo));
    assert_eq!(state, vec![true; 3], "one Undo restores every crossed row");
    assert!(!eyes.history.can_undo(), "the sweep was one step");
}

/// Escape during a sweep down the eyes reports the rows it crossed as cancelled, each with the state it had before.
#[trial]
fn escape_during_an_eye_sweep_restores_the_rows_crossed() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    let (start, end) = (node(&fixture, "Visibility Title").center(), node(&fixture, "Visibility Smart Object").center());
    fixture.frame(vec![pointer::moved(start)]);
    fixture.frame(vec![pointer::press(start)]);
    fixture.frame(vec![pointer::moved(end)]);
    take(&app);
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    let events = take(&app);
    let restored = toggles(&events);
    let crossed = [LayerRowKey::Layer(4), LayerRowKey::Layer(5), LayerRowKey::Layer(6)];
    let rows: Vec<_> = restored.iter().flat_map(|toggle| toggle.rows.clone()).collect();
    assert_eq!(rows, crossed, "every row crossed is restored: {events:?}");
    assert!(
        restored.iter().all(|toggle| matches!(toggle.edit, EditEvent::Cancel { before: Some(Mixed::Same(true)), .. })),
        "each with the state it had before: {events:?}"
    );
    let gestures: Vec<_> = restored.iter().map(|toggle| toggle.edit.id()).collect();
    assert!(gestures.iter().all(Option::is_some) && gestures.windows(2).all(|pair| pair[0] == pair[1]), "one gesture: {gestures:?}");
}

/// Escape during an Alt eye sweep reports the undo of its Show Only, once and with no toggle.
#[trial]
fn escape_during_an_alt_eye_sweep_undoes_the_show_only() {
    let (mut fixture, app) = rig(FakeLayers::sample());
    let (start, end) = (node(&fixture, "Visibility Title").center(), node(&fixture, "Visibility Smart Object").center());
    fixture.hold(Modifiers::ALT);
    fixture.frame(vec![pointer::moved(start)]);
    fixture.frame(vec![pointer::press(start)]);
    fixture.frame(vec![pointer::moved(end)]);
    take(&app);
    fixture.key(Key::Escape, Modifiers::ALT);
    fixture.idle(2);
    let events = take(&app);
    let undone: Vec<_> = events.iter().filter(|event| matches!(event, LayersEvent::UndoShowOnly(_))).collect();
    assert_eq!(undone, [&LayersEvent::UndoShowOnly(LayerRowKey::Layer(4))], "events: {events:?}");
    assert!(toggles(&events).is_empty(), "events: {events:?}");
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
    for width in (6..=21).rev().map(|step| step as f32 * 20.0) {
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

/// The list item named `name`.
fn item_named(fixture: &Fixture, name: &str) -> Target {
    fixture
        .by_role(Role::TREE_ITEM)
        .into_iter()
        .find(|item| fixture.tree().name_of(item.node).as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no list item named {name:?}"))
}

/// The rig of the Smart Filters trials: the smart object's filters open at thumbnail size `size`.
fn smart_filter_rig(size: ThumbSize) -> Fixture {
    let mut tree = FakeLayers::sample();
    tree.selection = LayerSelection::default();
    rig_with(tree, |app| {
        app.tree.layer_mut(6).effects_open = true;
        app.settings.options.thumbnail_size = size;
    })
    .0
}

/// The Smart Filters header carries the filter mask's thumbnail, so it is as tall as a layer row
/// and taller than a filter row; with thumbnails off all three rows are alike (Photoshop). A row's height follows its
/// thumbnail, which paints no node of its own.
#[trial]
fn the_smart_filters_header_holds_the_shared_mask_thumbnail() {
    for size in [ThumbSize::Medium] {
        let fixture = smart_filter_rig(size);
        let height = |name: &str| item_named(&fixture, name).rect.height();
        let (header, filter, layer) = (height("Smart Filters"), height("Gaussian Blur"), height("Styled"));
        assert!(header > filter, "{size:?}: the header row holds a thumbnail: {header} against {filter}");
        assert_eq!(header, layer, "{size:?}: the filter mask thumbnail matches the layer thumbnails");
    }
    let fixture = smart_filter_rig(ThumbSize::None);
    let (header, filter) = (item_named(&fixture, "Smart Filters").rect.height(), item_named(&fixture, "Gaussian Blur").rect.height());
    assert!((header - filter).abs() < 0.5, "no thumbnail at size None: {header} against {filter}");
}

/// Right-click on the node named `label`.
fn context_click_on(fixture: &mut Fixture, label: &str) {
    let target = node(fixture, label);
    context_click(fixture, &target);
}
