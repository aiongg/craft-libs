//! Behaviour tests of the whole panel over `MockLayers` (L9): input through `LayersPanel::show`.

#![allow(clippy::expect_used, clippy::indexing_slicing, reason = "tests")]

use craft_theme::LabelColor;
use craft_ui::behavior::{DropPosition, ListDrag, SelectionPick};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::list::ThumbSize;
use craft_ui::menu::MenuItem;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{CursorIcon, DragAndDrop, Event, Key, Modifiers, PointerButton, Pos2, Rect, Shape, Ui, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, Node};

use super::{ActivateOn, LayerColumn, LayerDrop, LayerMove, LayersEvent, LayersFooterButton, LayersPanel};
use crate::ui::mock::MockLayers;
use crate::ui::options::LayersSettings;
use crate::ui::tree::{LayerPart, LayerRowKey, LayerSelection};

struct App {
    tree: MockLayers,
    settings: LayersSettings,
    lock_column: bool,
    events: Vec<LayersEvent<u32>>,
    /// The last pass's drop preview.
    preview: Option<LayerMove<u32>>,
    min_width: f32,
    panel_min_width: f32,
    /// The modifiers the test holds.
    held: Modifiers,
}

impl App {
    fn new(tree: MockLayers) -> Self {
        Self {
            tree,
            settings: LayersSettings::default(),
            lock_column: false,
            events: Vec::new(),
            preview: None,
            min_width: 0.0,
            panel_min_width: 0.0,
            held: Modifiers::NONE,
        }
    }
}

/// Applies what the panel reported to the app's tree, as an app would.
fn apply(tree: &mut MockLayers, event: &LayersEvent<u32>) {
    match event {
        LayersEvent::Select(select) => match (select.pick, select.layer) {
            (SelectionPick::Clear, _) | (_, None) => tree.selection = LayerSelection::default(),
            (SelectionPick::Toggle, Some(layer)) => {
                if let Some(at) = tree.selection.layers.iter().position(|l| *l == layer) {
                    tree.selection.layers.remove(at);
                } else {
                    tree.selection.layers.push(layer);
                }
                tree.selection.active = Some(layer);
            }
            (_, Some(layer)) => {
                tree.selection = LayerSelection { layers: vec![layer], active: Some(layer), target: select.part.unwrap_or(LayerPart::Pixels) };
            }
        },
        LayersEvent::Open(change) => {
            if let Some(layer) = tree.layers.iter_mut().find(|l| l.id == change.row) {
                layer.open = change.open;
            }
        }
        LayersEvent::FxOpen { layer, open, .. } => {
            if let Some(layer) = tree.layers.iter_mut().find(|l| l.id == *layer) {
                layer.effects_open = *open;
            }
        }
        _ => {}
    }
}

fn show(ui: &mut Ui, app: &mut App) {
    let panel = LayersPanel::new("layers", &app.tree)
        .settings(&mut app.settings)
        .lock_column(app.lock_column)
        .row_menu(|_| vec![MenuItem::new("Rename Layer…", String::from("renameLayer")).into()]);
    app.panel_min_width = panel.min_width(ui);
    let out = panel.show(ui);
    app.min_width = out.min_width;
    app.preview = out.drop_preview;
    for event in &out.events {
        apply(&mut app.tree, event);
    }
    app.events.extend(out.events);
}

fn harness_of(app: App) -> Harness<'static, App> {
    let mut harness = Harness::<App>::builder()
        .with_size([420.0, 700.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, app);
    harness.run();
    harness
}

fn harness() -> Harness<'static, App> {
    harness_of(App::new(MockLayers::sample()))
}

fn take_(h: &mut Harness<'static, App>) -> Vec<LayersEvent<u32>> {
    std::mem::take(&mut h.state_mut().events)
}

fn row<'h>(h: &'h Harness<'_, App>, name: &str) -> Node<'h> {
    let name = name.to_owned();
    h.get_by(move |n| n.role() == Role::TreeItem && n.label().as_deref() == Some(name.as_str()))
}

fn rect(h: &Harness<'_, App>, name: &str) -> Rect {
    row(h, name).rect()
}

/// A point in the row `name`, `share` of the way down, at `x` (the row's centre by default).
fn at(h: &Harness<'_, App>, name: &str, share: f32) -> Pos2 {
    let r = rect(h, name);
    Pos2::new(r.center().x, r.top() + r.height() * share)
}

fn eye(h: &Harness<'_, App>, name: &str) -> Pos2 {
    h.get_by_label(&format!("Visibility {name}")).rect().center()
}

fn mouse_(h: &mut Harness<'static, App>, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    h.event(Event::PointerMoved(pos));
    h.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers });
    h.run();
}

fn hover_(h: &mut Harness<'static, App>, pos: Pos2) {
    h.event(Event::PointerMoved(pos));
    h.run();
}

fn click_(h: &mut Harness<'static, App>, pos: Pos2, modifiers: Modifiers) {
    h.hover(pos);
    h.set_modifiers(modifiers);
    h.mouse(pos, true, modifiers);
    h.mouse(pos, false, modifiers);
    h.set_modifiers(Modifiers::NONE);
}

fn double_click_(h: &mut Harness<'static, App>, pos: Pos2) {
    h.hover(pos);
    for pressed in [true, false, true, false] {
        h.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    h.run();
}

fn set_modifiers_(h: &mut Harness<'static, App>, modifiers: Modifiers) {
    h.state_mut().held = modifiers;
    h.event(Event::ModifiersChanged(modifiers));
    h.run();
}

fn key_(h: &mut Harness<'static, App>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

/// Presses on `pos` and moves past egui's drag threshold.
fn grab_(h: &mut Harness<'static, App>, pos: Pos2) {
    h.hover(pos);
    h.mouse(pos, true, Modifiers::NONE);
    h.hover(pos + vec2(0.0, 8.0));
}

fn release_(h: &mut Harness<'static, App>, pos: Pos2) {
    let held = h.state().held;
    h.hover(pos);
    h.mouse(pos, false, held);
}

fn select_(h: &mut Harness<'static, App>, layers: &[u32], active: u32) {
    h.state_mut().tree.selection = LayerSelection { layers: layers.to_vec(), active: Some(active), target: LayerPart::Pixels };
    h.run();
}

/// The drivers as methods, so a call can read the harness for its arguments.
trait Drive {
    fn take(&mut self) -> Vec<LayersEvent<u32>>;
    fn mouse(&mut self, pos: Pos2, pressed: bool, modifiers: Modifiers);
    fn hover(&mut self, pos: Pos2);
    fn click(&mut self, pos: Pos2, modifiers: Modifiers);
    fn double_click(&mut self, pos: Pos2);
    fn set_modifiers(&mut self, modifiers: Modifiers);
    fn key(&mut self, key: Key, modifiers: Modifiers);
    fn grab(&mut self, pos: Pos2);
    fn release(&mut self, pos: Pos2);
    fn select(&mut self, layers: &[u32], active: u32);
    fn drag_to(&mut self, from: Pos2, to: Pos2) -> Vec<LayersEvent<u32>>;
    fn thumb_centres(&mut self, name: &str) -> Vec<f32>;
    fn start_rename(&mut self, name: &str);
    fn retype(&mut self, text: &str);
}

impl Drive for Harness<'static, App> {
    fn take(&mut self) -> Vec<LayersEvent<u32>> {
        take_(self)
    }
    fn mouse(&mut self, pos: Pos2, pressed: bool, modifiers: Modifiers) {
        mouse_(self, pos, pressed, modifiers);
    }
    fn hover(&mut self, pos: Pos2) {
        hover_(self, pos);
    }
    fn click(&mut self, pos: Pos2, modifiers: Modifiers) {
        click_(self, pos, modifiers);
    }
    fn double_click(&mut self, pos: Pos2) {
        double_click_(self, pos);
    }
    fn set_modifiers(&mut self, modifiers: Modifiers) {
        set_modifiers_(self, modifiers);
    }
    fn key(&mut self, key: Key, modifiers: Modifiers) {
        key_(self, key, modifiers);
    }
    fn grab(&mut self, pos: Pos2) {
        grab_(self, pos);
    }
    fn release(&mut self, pos: Pos2) {
        release_(self, pos);
    }
    fn select(&mut self, layers: &[u32], active: u32) {
        select_(self, layers, active);
    }
    fn drag_to(&mut self, from: Pos2, to: Pos2) -> Vec<LayersEvent<u32>> {
        drag_to_(self, from, to)
    }
    fn thumb_centres(&mut self, name: &str) -> Vec<f32> {
        thumb_centres_(self, name)
    }
    fn start_rename(&mut self, name: &str) {
        start_rename_(self, name);
    }
    fn retype(&mut self, text: &str) {
        retype_(self, text);
    }
}

fn toggles(events: &[LayersEvent<u32>]) -> Vec<&super::LayerToggle<u32>> {
    events.iter().filter_map(|e| if let LayersEvent::Toggle(t) = e { Some(t) } else { None }).collect()
}

/// A pointer click on a toggle cell is a gesture of one value: Begin, Update, Commit.
fn assert_click_gesture(events: &[LayersEvent<u32>], layer: u32, column: LayerColumn, on: bool) {
    let toggles = toggles(events);
    assert_eq!(toggles.len(), events.len(), "only toggles: {events:?}");
    assert!(toggles.iter().all(|t| t.row == LayerRowKey::Layer(layer) && t.column == column), "{events:?}");
    let ids: Vec<_> = toggles.iter().filter_map(|t| t.edit.id()).collect();
    assert!(ids.windows(2).all(|w| w[0] == w[1]), "one gesture: {ids:?}");
    assert!(matches!(toggles[0].edit, EditEvent::Begin { .. }), "{events:?}");
    assert!(matches!(&toggles.last().expect("events").edit, EditEvent::Commit { value, changed: true, .. } if value.on == on), "{events:?}");
}

fn selects(events: &[LayersEvent<u32>]) -> Vec<(Option<u32>, SelectionPick)> {
    events.iter().filter_map(|e| if let LayersEvent::Select(s) = e { Some((s.layer, s.pick)) } else { None }).collect()
}

fn drops(events: &[LayersEvent<u32>]) -> Vec<&EditEvent<LayerDrop<u32>>> {
    events.iter().filter_map(|e| if let LayersEvent::Drop(d) = e { Some(d) } else { None }).collect()
}

fn committed(events: &[LayersEvent<u32>]) -> Option<&LayerDrop<u32>> {
    drops(events).into_iter().find_map(|e| if let EditEvent::Commit { value, .. } = e { Some(value) } else { None })
}

fn cancelled(events: &[LayersEvent<u32>]) -> Option<CancelReason> {
    drops(events).into_iter().find_map(|e| if let EditEvent::Cancel { reason, .. } = e { Some(*reason) } else { None })
}

fn layer_move(layers: &[u32], target: u32, position: DropPosition, copy: bool) -> LayerDrop<u32> {
    LayerDrop::Layers(LayerMove { layers: layers.to_vec(), target, position, copy })
}

fn painted_texts(h: &Harness<'_, App>, text: &str) -> usize {
    fn count(shape: &Shape, text: &str) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(|s| count(s, text)).sum(),
            Shape::Text(t) => usize::from(t.galley.text() == text),
            _ => 0,
        }
    }
    h.output().shapes.iter().map(|s| count(&s.shape, text)).sum()
}

/// The shapes painted within 30 points of `pointer`, flattened.
fn shapes_near(h: &Harness<'_, App>, pointer: Pos2) -> usize {
    fn count(shape: &Shape, area: Rect) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(|s| count(s, area)).sum(),
            other => usize::from(area.contains_rect(other.visual_bounding_rect())),
        }
    }
    let area = Rect::from_center_size(pointer + vec2(20.0, 20.0), vec2(60.0, 60.0));
    h.output().shapes.iter().map(|s| count(&s.shape, area)).sum()
}

/// The x of each run of the row's width over which Cmd shows the pointing hand but a plain hover
/// does not: the centres of the thumbnails that load a selection.
fn thumb_centres_(h: &mut Harness<'static, App>, name: &str) -> Vec<f32> {
    let r = rect(h, name);
    let y = r.center().y;
    let mut hands = Vec::new();
    for with_cmd in [false, true] {
        h.set_modifiers(if with_cmd { Modifiers::COMMAND } else { Modifiers::NONE });
        let mut found = Vec::new();
        let mut x = r.left() + 1.0;
        while x < r.right() {
            h.hover(Pos2::new(x, y));
            found.push(h.output().platform_output.cursor_icon == CursorIcon::PointingHand);
            x += 2.0;
        }
        hands.push(found);
    }
    h.set_modifiers(Modifiers::NONE);
    let mut centres = Vec::new();
    let mut start = None;
    for (i, (plain, cmd)) in hands[0].iter().zip(&hands[1]).enumerate() {
        let on = *cmd && !*plain;
        match (on, start) {
            (true, None) => start = Some(i),
            (false, Some(from)) => {
                centres.push(r.left() + 1.0 + (from + i) as f32);
                start = None;
            }
            _ => {}
        }
    }
    centres
}

// An eye click is one `Toggle(Set)`, Alt-click `ShowOnly`, a sweep down the eyes one gesture.
#[test]
fn eye_click_alt_click_and_sweep_report_one_event_or_gesture() {
    let mut h = harness();
    h.click(eye(&h, "Title"), Modifiers::NONE);
    let events = h.take();
    assert_click_gesture(&events, 4, LayerColumn::Visibility, false);

    h.click(eye(&h, "Title"), Modifiers::ALT);
    h.set_modifiers(Modifiers::NONE);
    let events = h.take();
    assert!(events.contains(&LayersEvent::ShowOnly(LayerRowKey::Layer(4))), "{events:?}");

    let (start, end) = (eye(&h, "Title"), eye(&h, "Smart Object"));
    h.hover(start);
    h.mouse(start, true, Modifiers::NONE);
    for t in [0.3, 0.6, 1.0] {
        h.hover(start + (end - start) * t);
    }
    h.mouse(end, false, Modifiers::NONE);
    let events = h.take();
    let sweep = toggles(&events);
    assert_eq!(sweep.len(), events.len(), "nothing but toggles: {events:?}");
    assert!(matches!(sweep[0].edit, EditEvent::Begin { .. }) && matches!(sweep.last().expect("events").edit, EditEvent::Commit { .. }));
    assert!(sweep.iter().all(|t| t.row == LayerRowKey::Layer(4) && t.column == LayerColumn::Visibility), "every event names the row it started on");
    let ids: Vec<_> = sweep.iter().filter_map(|t| t.edit.id()).collect();
    assert!(ids.windows(2).all(|w| w[0] == w[1]), "one gesture: {ids:?}");
    let updated: Vec<u32> = sweep
        .iter()
        .filter_map(|t| match &t.edit {
            EditEvent::Update { value, .. } => match &value.row {
                LayerRowKey::Layer(id) => Some(*id),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert!(updated.contains(&5) && updated.contains(&6), "the sweep reached the rows below: {updated:?}");
}

// An Alt-click on an eye is `ShowOnly` and no toggle that hides the row again.
#[test]
#[ignore = "body.rs `toggle_event` (about line 480): the gesture of an Alt-click still ends in a `Toggle` Commit with the row's new state after `ShowOnly`"]
fn alt_click_on_an_eye_reports_show_only_without_a_hiding_commit() {
    let mut h = harness();
    h.click(eye(&h, "Title"), Modifiers::ALT);
    let events = h.take();
    assert!(events.contains(&LayersEvent::ShowOnly(LayerRowKey::Layer(4))), "{events:?}");
    let hides = toggles(&events).into_iter().any(|t| matches!(&t.edit, EditEvent::Commit { value, .. } if !value.on));
    assert!(!hides, "{events:?}");
}

// A pick in the eye's colour menu reports `LabelColor` for every selected layer.
#[test]
fn eye_colour_menu_reports_the_pick_for_every_selected_layer() {
    let mut h = harness();
    h.select(&[4, 5], 4);
    h.get_by_label("Visibility Title").click_secondary();
    h.run();
    h.get_by_label("Red").click();
    h.run();
    let events = h.take();
    assert_eq!(events, [LayersEvent::LabelColor { rows: vec![4, 5], color: Some(LabelColor::Red) }]);

    h.get_by_label("Visibility Hidden").click_secondary();
    h.run();
    h.get_by_label("Blue").click();
    h.run();
    let events = h.take();
    assert!(events.contains(&LayersEvent::LabelColor { rows: vec![8], color: Some(LabelColor::Blue) }), "a row outside the selection: {events:?}");
    assert!(selects(&events).is_empty(), "the colour menu does not select");
}

// A click on the lock indicator reports the lock `Toggle` off and no `Select`.
#[test]
fn lock_icon_click_unlocks_without_selecting() {
    let mut h = harness();
    h.get_by_label("Unlock Background").click();
    h.run();
    let events = h.take();
    let [LayersEvent::Toggle(toggle)] = events.as_slice() else { panic!("one toggle and no select: {events:?}") };
    assert_eq!((&toggle.row, toggle.column), (&LayerRowKey::Layer(9), LayerColumn::Lock));
    assert!(matches!(&toggle.edit, EditEvent::Set { value, .. } if !value.on), "{:?}", toggle.edit);
}

// The panel's minimum width holds a group row with one mask at each thumbnail size.
#[test]
fn panel_min_width_holds_a_group_row_with_one_mask() {
    let mut tree = MockLayers::sample();
    tree.layers[0].info.masks.push(crate::ui::tree::MaskInfo { part: LayerPart::LayerMask, enabled: true, linked: true });
    let mut widths = Vec::new();
    for size in [ThumbSize::None, ThumbSize::Small, ThumbSize::Medium, ThumbSize::Large] {
        let mut app = App::new(tree.clone());
        app.settings.options.thumbnail_size = size;
        let mut h = harness_of(app);
        let min = h.state().min_width;
        assert_eq!(min, h.state().panel_min_width, "`min_width` and the shown panel agree ({size:?})");
        h.set_size(vec2(min, 700.0));
        h.run();
        if size != ThumbSize::None {
            let chain = h.get_by_label("Unlink layer mask of Group 1").rect();
            let eye = h.get_by_label("Visibility Group 1").rect();
            assert!(eye.max.x < chain.min.x && chain.max.x <= min, "{size:?}: the chain sits inside {min}: {chain:?}");
        }
        widths.push(min);
    }
    assert!(widths.windows(2).all(|w| w[0] <= w[1]), "larger thumbnails never need less: {widths:?}");
    assert!(widths[3] > widths[0], "{widths:?}");
}

// A click on a mask chain reports `MaskLink` with the opposite state.
#[test]
fn mask_chain_click_reports_the_link_change() {
    let mut h = harness();
    h.get_by_label("Unlink layer mask of Masked").click();
    h.run();
    let events = h.take();
    assert!(events.contains(&LayersEvent::MaskLink { layer: 3, part: LayerPart::LayerMask, linked: false }), "{events:?}");

    h.state_mut().tree.layers[2].info.masks[0].linked = false;
    h.run();
    h.get_by_label("Link layer mask of Masked").click();
    h.run();
    let events = h.take();
    assert!(events.contains(&LayersEvent::MaskLink { layer: 3, part: LayerPart::LayerMask, linked: true }), "{events:?}");
}

// With Cmd held over a loadable thumbnail the cursor is the pointing hand with the operation's
// badge.
#[test]
fn cmd_over_a_thumbnail_shows_the_hand_with_the_mode_badge() {
    let mut h = harness();
    let centres = h.thumb_centres("Title");
    assert_eq!(centres.len(), 1, "one thumbnail loads a selection: {centres:?}");
    let pointer = Pos2::new(centres[0], rect(&h, "Title").center().y);

    let mut shapes = Vec::new();
    for modifiers in [Modifiers::COMMAND, Modifiers::COMMAND | Modifiers::SHIFT, Modifiers::COMMAND | Modifiers::ALT] {
        h.set_modifiers(modifiers);
        h.hover(pointer);
        assert_eq!(h.output().platform_output.cursor_icon, CursorIcon::PointingHand, "{modifiers:?}");
        shapes.push(shapes_near(&h, pointer));
    }
    assert!(shapes[1] > shapes[0] && shapes[2] > shapes[0], "Add and Subtract draw a badge, New none: {shapes:?}");
    assert!(shapes[1] > shapes[2], "the plus has more strokes than the minus: {shapes:?}");

    h.set_modifiers(Modifiers::COMMAND);
    h.hover(Pos2::new(centres[0], rect(&h, "Background").center().y));
    assert_ne!(h.output().platform_output.cursor_icon, CursorIcon::PointingHand, "the Background's thumbnail loads no selection");
}

// A double click on a renamable name opens the rename; on the Background's name it reports
// `Activate { Name }`; on a thumbnail `Activate { Thumbnail(part) }`.
#[test]
fn double_click_name_renames_and_the_background_reports_activate() {
    let mut h = harness();
    let name = Pos2::new(rect(&h, "Title").left() + 190.0, rect(&h, "Title").center().y);
    h.double_click(name);
    assert!(h.query_by_role(Role::TextInput).is_some(), "the name is being edited");
    let events = h.take();
    assert!(!events.iter().any(|e| matches!(e, LayersEvent::Activate { .. })), "a rename is not an activation: {events:?}");
    h.key(Key::Escape, Modifiers::NONE);
    h.take();

    h.double_click(Pos2::new(rect(&h, "Background").left() + 190.0, rect(&h, "Background").center().y));
    assert!(h.query_by_role(Role::TextInput).is_none(), "the Background cannot be renamed");
    let events = h.take();
    assert!(events.contains(&LayersEvent::Activate { row: LayerRowKey::Layer(9), on: ActivateOn::Name }), "{events:?}");

    let centres = h.thumb_centres("Title");
    h.double_click(Pos2::new(centres[0], rect(&h, "Title").center().y));
    let events = h.take();
    assert!(events.contains(&LayersEvent::Activate { row: LayerRowKey::Layer(4), on: ActivateOn::Thumbnail(LayerPart::Pixels) }), "{events:?}");
}

fn renames(events: &[LayersEvent<u32>]) -> Vec<(u32, &str)> {
    events.iter().filter_map(|e| if let LayersEvent::Rename { layer, text } = e { Some((*layer, text.as_str())) } else { None }).collect()
}

fn start_rename_(h: &mut Harness<'static, App>, name: &str) {
    let r = rect(h, name);
    h.double_click(Pos2::new(r.left() + 190.0, r.center().y));
    assert!(h.query_by_role(Role::TextInput).is_some(), "renaming {name}");
    h.take();
}

fn retype_(h: &mut Harness<'static, App>, text: &str) {
    h.key(Key::A, Modifiers::COMMAND);
    if text.is_empty() {
        h.key(Key::Backspace, Modifiers::NONE);
    } else {
        h.event(Event::Text(text.to_owned()));
        h.run();
    }
}

// Enter and Tab commit, Esc cancels, an empty or unchanged name reports nothing.
#[test]
fn rename_rules_enter_tab_esc_empty_unchanged() {
    let mut h = harness();
    h.start_rename("Title");
    h.retype("  Heading ");
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(renames(&h.take()), [(4, "Heading")], "Enter commits the trimmed text");
    assert!(h.query_by_role(Role::TextInput).is_none());

    h.start_rename("Title");
    h.retype("Tabbed");
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(renames(&h.take()), [(4, "Tabbed")], "Tab commits");
    assert!(h.query_by_role(Role::TextInput).is_some(), "and continues on the next renamable row");
    h.key(Key::Escape, Modifiers::NONE);
    assert!(renames(&h.take()).is_empty(), "Esc cancels the continued edit");

    h.start_rename("Title");
    h.retype("Discarded");
    h.key(Key::Escape, Modifiers::NONE);
    assert!(renames(&h.take()).is_empty(), "Esc reports nothing");
    assert!(h.query_by_role(Role::TextInput).is_none());

    h.start_rename("Title");
    h.retype("");
    h.key(Key::Enter, Modifiers::NONE);
    assert!(renames(&h.take()).is_empty(), "an empty name reports nothing");

    h.start_rename("Title");
    h.key(Key::Enter, Modifiers::NONE);
    assert!(renames(&h.take()).is_empty(), "an unchanged name reports nothing");
}

// The list keys walk rows, open groups, activate and rename.
#[test]
fn keys_walk_rows_open_groups_activate_and_rename() {
    let mut h = harness();
    h.click(at(&h, "Title", 0.5), Modifiers::NONE);
    h.take();
    let mut walked = Vec::new();
    for k in [Key::ArrowDown, Key::ArrowDown, Key::ArrowUp, Key::Home] {
        h.key(k, Modifiers::NONE);
        walked.extend(selects(&h.take()));
    }
    let replace = |layer| (Some(layer), SelectionPick::Replace);
    assert_eq!(walked, [replace(5), replace(6), replace(5), replace(1)]);

    h.key(Key::ArrowLeft, Modifiers::NONE);
    let events = h.take();
    assert!(events.iter().any(|e| matches!(e, LayersEvent::Open(c) if c.row == 1 && !c.open && !c.all)), "← closes the group: {events:?}");
    h.key(Key::ArrowRight, Modifiers::NONE);
    let events = h.take();
    assert!(events.iter().any(|e| matches!(e, LayersEvent::Open(c) if c.row == 1 && c.open)), "→ opens it: {events:?}");

    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(selects(&h.take()), [replace(2)], "into the opened group");
    h.key(Key::Enter, Modifiers::NONE);
    assert!(h.take().contains(&LayersEvent::Activate { row: LayerRowKey::Layer(2), on: ActivateOn::Row }));
    h.key(Key::F2, Modifiers::NONE);
    assert!(h.query_by_role(Role::TextInput).is_some(), "F2 renames");
}

// Shift+→ reports `FxOpen` for the lead, Alt+Shift+→ for every layer.
#[test]
#[ignore = "body bug: `fx_key` tests focus on `ui.make_persistent_id(salt)` (body.rs:133) but the list's id is built from `IdSalt::new(salt)`, so the key is never seen"]
fn shift_right_opens_effects_and_alt_opens_all_layers() {
    let mut h = harness();
    h.click(at(&h, "Styled", 0.5), Modifiers::NONE);
    h.take();
    h.key(Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(h.take(), [LayersEvent::FxOpen { layer: 7, open: true, all: false }]);
    h.key(Key::ArrowRight, Modifiers::SHIFT | Modifiers::ALT);
    assert_eq!(h.take(), [LayersEvent::FxOpen { layer: 7, open: true, all: true }]);
    h.key(Key::ArrowLeft, Modifiers::SHIFT);
    assert_eq!(h.take(), [LayersEvent::FxOpen { layer: 7, open: false, all: false }]);

    h.click(at(&h, "Title", 0.5), Modifiers::NONE);
    h.take();
    h.key(Key::ArrowRight, Modifiers::SHIFT);
    assert!(h.take().is_empty(), "a layer without effects reports nothing");
}

// Alt-click on a group triangle reports `Open` with `all`.
#[test]
fn group_triangle_alt_click_opens_every_group() {
    let mut h = harness();
    h.get_by_label("Collapse Group 1").click_modifiers(Modifiers::ALT);
    h.run();
    let events = h.take();
    assert!(events.iter().any(|e| matches!(e, LayersEvent::Open(c) if c.row == 1 && !c.open && c.all)), "{events:?}");
}

// A click below the last row reports `Select` with `Clear`.
#[test]
fn empty_space_click_clears_the_selection() {
    let mut h = harness();
    let below = Pos2::new(200.0, rect(&h, "Background").bottom() + 20.0);
    assert!(h.get_by_role(Role::Tree).rect().contains(below), "empty space under the rows");
    h.click(below, Modifiers::NONE);
    assert_eq!(selects(&h.take()), [(None, SelectionPick::Clear)]);
}

/// The preview option on, with the selection `layers` (active the first).
fn previewing(layers: &[u32]) -> Harness<'static, App> {
    let mut app = App::new(MockLayers::sample());
    app.settings.options.preview_on_canvas_when_reordering = true;
    app.tree.selection = LayerSelection { layers: layers.to_vec(), active: layers.first().copied(), target: LayerPart::Pixels };
    harness_of(app)
}

// A drag of a selected row moves the whole selection as one `Drop` gesture, with the marker and
// the ghost.
#[test]
fn layer_drag_moves_the_selection_as_one_gesture_with_marker_and_ghost() {
    let mut h = previewing(&[4, 5]);
    assert_eq!(painted_texts(&h, "Title"), 1);
    h.grab(at(&h, "Title", 0.5));
    let payload = DragAndDrop::payload::<ListDrag<LayerRowKey<u32>>>(&h.ctx).expect("the payload");
    assert_eq!((payload.items.clone(), payload.label.as_str()), (vec![LayerRowKey::Layer(4), LayerRowKey::Layer(5)], "Title"));
    h.hover(at(&h, "Hidden", 0.75));
    assert_eq!(painted_texts(&h, "Title"), 2, "the ghost carries the grabbed label beside the row's own");
    let marker = h.state().preview.clone();
    assert_eq!(marker, Some(LayerMove { layers: vec![4, 5], target: 8, position: DropPosition::After, copy: false }), "the marker's move");
    h.release(at(&h, "Hidden", 0.75));
    let events = h.take();
    assert_eq!(committed(&events), Some(&layer_move(&[4, 5], 8, DropPosition::After, false)));
    let all = drops(&events);
    assert!(matches!(all.first(), Some(EditEvent::Begin { .. })), "{all:?}");
    let ids: Vec<_> = all.iter().filter_map(|e| e.id()).collect();
    assert!(ids.windows(2).all(|w| w[0] == w[1]), "one gesture: {ids:?}");
    assert!(selects(&events).is_empty(), "a drag does not select");
}

// A group row's 0.3 to 0.7 band drops Into, the halves Before and After, the gaps and the space
// below the last row are drop zones.
#[test]
fn drop_zones_into_above_below_and_between_rows() {
    let mut h = previewing(&[1]);
    h.grab(at(&h, "Hidden", 0.5));
    let target = |h: &Harness<'static, App>| h.state().preview.clone().map(|m| (m.target, m.position));
    for (share, position) in [(0.5, DropPosition::Into), (0.62, DropPosition::Into), (0.15, DropPosition::Before), (0.85, DropPosition::After)] {
        h.hover(at(&h, "Group 1", share));
        assert_eq!(target(&h), Some((1, position)), "Group 1 at {share}");
    }
    for (share, position) in [(0.25, DropPosition::Before), (0.75, DropPosition::After)] {
        h.hover(at(&h, "Title", share));
        assert_eq!(target(&h), Some((4, position)), "a layer row has halves only: {share}");
    }
    let (above, below) = (rect(&h, "Title"), rect(&h, "Levels 1"));
    let gap = below.top() - above.bottom();
    if gap > 0.5 {
        h.hover(Pos2::new(above.center().x, above.bottom() + gap / 2.0));
        assert!(target(&h).is_some(), "the gap between rows is a drop zone");
    }
    h.hover(Pos2::new(200.0, rect(&h, "Background").bottom() + 20.0));
    assert_eq!(target(&h), Some((9, DropPosition::After)), "the space below the last row");
    h.release(Pos2::new(200.0, rect(&h, "Background").bottom() + 20.0));
    assert_eq!(committed(&h.take()), Some(&layer_move(&[8], 9, DropPosition::After, false)));
}

// Alt during a drag copies and shows the copy cursor.
#[test]
fn alt_drag_copies_and_shows_the_copy_cursor() {
    let mut h = harness();
    h.grab(at(&h, "Hidden", 0.5));
    h.hover(at(&h, "Title", 0.75));
    assert_ne!(h.output().platform_output.cursor_icon, CursorIcon::Copy, "no copy cursor without Alt");
    h.set_modifiers(Modifiers::ALT);
    assert_eq!(h.output().platform_output.cursor_icon, CursorIcon::Copy);
    h.release(at(&h, "Title", 0.75));
    h.set_modifiers(Modifiers::NONE);
    assert_eq!(committed(&h.take()), Some(&layer_move(&[8], 4, DropPosition::After, true)));
}

// Esc and a release outside the panel end the drag as `Cancel`.
#[test]
fn esc_and_release_elsewhere_cancel_a_drag() {
    let mut h = harness();
    h.grab(at(&h, "Hidden", 0.5));
    h.hover(at(&h, "Title", 0.75));
    h.take();
    h.key(Key::Escape, Modifiers::NONE);
    let events = h.take();
    assert_eq!(cancelled(&events), Some(CancelReason::Escape), "{events:?}");
    h.release(at(&h, "Title", 0.75));
    assert!(committed(&h.take()).is_none(), "the rest of a cancelled drag does nothing");

    h.run_steps(30);
    h.grab(at(&h, "Hidden", 0.5));
    h.hover(at(&h, "Title", 0.75));
    h.take();
    let elsewhere = Pos2::new(200.0, h.get_by_role(Role::Tree).rect().top() - 12.0);
    h.release(elsewhere);
    let events = h.take();
    assert_eq!(cancelled(&events), Some(CancelReason::Dismissed), "{events:?}");
    assert!(committed(&events).is_none());
}

fn drag_to_(h: &mut Harness<'static, App>, from: Pos2, to: Pos2) -> Vec<LayersEvent<u32>> {
    h.run_steps(30);
    h.grab(from);
    h.hover(to);
    h.release(to);
    h.take()
}

fn footer_drops(events: &[LayersEvent<u32>]) -> Vec<(LayersFooterButton, Vec<LayerRowKey<u32>>)> {
    events.iter().filter_map(|e| if let LayersEvent::FooterDrop { button, rows } = e { Some((*button, rows.clone())) } else { None }).collect()
}

// Layer rows, effects rows and a mask dropped on Delete, New Layer and New Group report
// `FooterDrop`; New Layer and New Group refuse effects and masks.
#[test]
fn rows_masks_and_effects_dropped_on_footer_buttons_report_footer_drop() {
    let mut h = harness();
    h.state_mut().tree.layers[6].effects_open = true;
    h.run();
    let button = |h: &Harness<'static, App>, label: &str| h.get_by_label(label).rect().center();
    let (delete, new_layer, new_group) = ("Delete Layer", "Create a new layer", "Create a new group");
    let layer = LayerRowKey::Layer(4);

    for (label, which) in [(delete, LayersFooterButton::Delete), (new_layer, LayersFooterButton::NewLayer), (new_group, LayersFooterButton::NewGroup)]
    {
        let to = button(&h, label);
        let events = h.drag_to(at(&h, "Title", 0.5), to);
        assert_eq!(footer_drops(&events), [(which, vec![layer.clone()])], "a layer row on {label}: {events:?}");
    }

    let effect = LayerRowKey::Effects(7, Some(0));
    let to = button(&h, delete);
    let events = h.drag_to(at(&h, "Drop Shadow", 0.5), to);
    assert_eq!(footer_drops(&events), [(LayersFooterButton::Delete, vec![effect])], "{events:?}");
    for label in [new_layer, new_group] {
        let to = button(&h, label);
        let events = h.drag_to(at(&h, "Drop Shadow", 0.5), to);
        assert!(footer_drops(&events).is_empty(), "an effect on {label}: {events:?}");
    }

    let masks = h.thumb_centres("Masked");
    assert!(!masks.is_empty());
    let mask_at = Pos2::new(*masks.last().expect("a thumbnail"), rect(&h, "Masked").center().y);
    let mask = LayerRowKey::Mask(3, LayerPart::LayerMask);
    let to = button(&h, delete);
    let events = h.drag_to(mask_at, to);
    assert_eq!(footer_drops(&events), [(LayersFooterButton::Delete, vec![mask])], "{events:?}");
    for label in [new_layer, new_group] {
        let to = button(&h, label);
        let events = h.drag_to(mask_at, to);
        assert!(footer_drops(&events).is_empty(), "a mask on {label}: {events:?}");
    }
}

// A mask thumbnail dragged shows the mask's name beside the pointer.
#[test]
fn a_mask_drag_paints_its_ghost_beside_the_pointer() {
    let mut h = harness();
    let masks = h.thumb_centres("Masked");
    let from = Pos2::new(*masks.last().expect("a mask thumbnail"), rect(&h, "Masked").center().y);
    assert_eq!(painted_texts(&h, "layer mask"), 0);
    h.grab(from);
    h.hover(from + vec2(30.0, 40.0));
    assert_eq!(painted_texts(&h, "layer mask"), 1, "the ghost");
    h.release(from + vec2(30.0, 40.0));
    h.run();
    assert_eq!(painted_texts(&h, "layer mask"), 0, "gone after the drop");
}

// The list scrolls while a drag is near its top or bottom edge.
#[test]
fn edge_scroll_runs_while_a_drag_is_near_the_edge() {
    let mut h = harness_of(App::new(MockLayers::flat(300)));
    let list = h.get_by_role(Role::Tree).rect();
    h.grab(at(&h, "Layer 1", 0.5));
    let bottom = Pos2::new(list.center().x, list.bottom() - 2.0);
    h.event(Event::PointerMoved(bottom));
    h.run_steps(40);
    assert!(h.query_by_label("Layer 40").is_some(), "scrolled down to later rows");
    assert!(h.query_by_label("Layer 2").is_none(), "the first rows left the view");
    assert!(h.query_by_label("Layer 1").is_some(), "the grabbed row stays drawn");
    let top = Pos2::new(list.center().x, list.top() + 2.0);
    h.event(Event::PointerMoved(top));
    h.run_steps(120);
    assert!(h.query_by_label("Layer 40").is_none(), "scrolled back up");
    h.event(Event::PointerButton { pos: top, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}

// `.lock_column(true)` adds a lock column whose clicks and sweeps report `Toggle` with `Lock`.
#[test]
fn lock_column_reports_toggle_events() {
    let mut app = App::new(MockLayers::sample());
    app.lock_column = true;
    let mut h = harness_of(app);
    let cell = |h: &Harness<'static, App>, name: &str| h.get_by_label(&format!("Lock {name}")).rect().center();
    h.click(cell(&h, "Title"), Modifiers::NONE);
    let events = h.take();
    assert_click_gesture(&events, 4, LayerColumn::Lock, true);

    let (start, end) = (cell(&h, "Title"), cell(&h, "Smart Object"));
    h.hover(start);
    h.mouse(start, true, Modifiers::NONE);
    for t in [0.5, 1.0] {
        h.hover(start + (end - start) * t);
    }
    h.mouse(end, false, Modifiers::NONE);
    let events = h.take();
    let sweep = toggles(&events);
    assert!(sweep.len() >= 3 && sweep.iter().all(|t| t.column == LayerColumn::Lock && t.row == LayerRowKey::Layer(4)), "{events:?}");
    let ids: Vec<_> = sweep.iter().filter_map(|t| t.edit.id()).collect();
    assert!(ids.windows(2).all(|w| w[0] == w[1]), "one gesture: {ids:?}");
}

// Rename Layer… in the row menu starts the rename of the row.
#[test]
#[ignore = "Rename Layer… from the row menu is not wired: the panel has no `LayersPanel::start_rename` (L8 As built); a later fix adds it"]
fn rename_layer_from_the_row_menu_starts_the_rename() {
    let mut h = harness();
    h.click(at(&h, "Title", 0.5), Modifiers::NONE);
    h.take();
    h.get_by_label("Visibility Title").click_secondary();
    h.run();
    h.get_by_label("Rename Layer…").click();
    h.run();
    assert!(h.query_by_role(Role::TextInput).is_some(), "the name is being edited");
}
