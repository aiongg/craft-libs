//! The mock editor: its document logic, and the window's keys and menus.

use craft_ui::runtime::access::Role;
use craft_ui::runtime::env::RootEnv;
use craft_ui::runtime::env::UnitKind;
use craft_ui::shell::ScreenMode;
use craft_ui::testing::input::key_press;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, Frames, Target, pointer, trial};
use craft_ui_gallery::mock::{EditorMock, MockState, Tool};
use egui::{Event, Key, Modifiers, Pos2, Rect, Vec2, vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// Frames that let the layout, the focus and the animations of the window settle.
const SETTLED: usize = 4;

/// Frames that let a dialog open.
const DIALOG_OPEN: usize = 30;

/// What a mock window showed in the latest pass.
#[derive(Clone, Copy)]
struct Seen {
    mode: ScreenMode,
    slots: usize,
    body: Rect,
}

/// Mock windows side by side in one viewport, as the cells of a gallery page are.
struct Cells {
    frames: Frames,
    states: Vec<MockState>,
    seen: Vec<Seen>,
}

impl Cells {
    fn new(count: usize) -> Self {
        let mut frames = Frames::new();
        let mut root = RootEnv::default();
        root.tab_toggles_chrome = true;
        frames.root(root);
        let seen = vec![Seen { mode: ScreenMode::Standard, slots: 0, body: Rect::NOTHING }; count];
        let mut cells = Cells { frames, states: (0..count).map(|_| MockState::default()).collect(), seen };
        cells.settle();
        cells
    }

    fn frame(&mut self, events: Vec<Event>) {
        let Cells { frames, states, seen } = self;
        frames.frame(events, |ui| {
            let room = ui.max_rect();
            let width = room.width() / states.len() as f32;
            for (at, state) in states.iter_mut().enumerate() {
                let rect = Rect::from_min_size(room.min + vec2(width * at as f32, 0.0), vec2(width, room.height()));
                in_region(ui, rect, |ui| {
                    let out = EditorMock::new(format!("Editor {at}"), state).show(ui);
                    seen[at] = Seen { mode: out.screen_mode, slots: out.regions.slots.len(), body: out.regions.body };
                });
            }
        });
    }

    fn settle(&mut self) {
        for _ in 0..SETTLED {
            self.frame(Vec::new());
        }
    }

    fn click_body(&mut self, cell: usize) {
        let at = self.seen[cell].body.center();
        self.frame(vec![pointer::moved(at)]);
        self.frame(vec![pointer::press(at)]);
        self.frame(vec![pointer::release(at)]);
        self.settle();
    }

    fn press(&mut self, key: Key) {
        self.frame(vec![key_press(key, Modifiers::NONE)]);
        self.settle();
    }

    fn modes(&self) -> Vec<ScreenMode> {
        self.seen.iter().map(|seen| seen.mode).collect()
    }
}

/// A mock window whose state the test reads.
fn window(state: MockState) -> (Fixture, Rc<RefCell<MockState>>) {
    let state = Rc::new(RefCell::new(state));
    let shown = state.clone();
    let mut fixture = Fixture::new(move |ui| {
        EditorMock::new("Editor", &mut shown.borrow_mut()).show(ui);
    });
    fixture.room(Vec2::new(1000.0, 700.0)).settle();
    (fixture, state)
}

/// The menu title or row named `name`.
fn menu_row(fixture: &Fixture, name: &str) -> Target {
    let tree = fixture.tree();
    fixture
        .by_role(Role::MENU_ITEM)
        .into_iter()
        .find(|target| tree.name_of(target.node) == Some(name))
        .unwrap_or_else(|| panic!("a menu row named {name}"))
}

/// Opens Layer > Delete > Layer.
fn choose_delete_layer(fixture: &mut Fixture) {
    let layer = menu_row(fixture, "Layer");
    fixture.click(&layer).settle();
    let delete = menu_row(fixture, "Delete");
    fixture.click(&delete).settle();
    let row = menu_row(fixture, "Layer…");
    fixture.frame(vec![pointer::moved(row.center())]);
    fixture.click(&row);
}

#[trial]
fn f_cycles_the_screen_mode_of_the_cell_that_holds_focus_and_no_other() {
    let mut cells = Cells::new(2);
    cells.click_body(0);
    assert_eq!(cells.modes(), [ScreenMode::Standard, ScreenMode::Standard]);
    cells.press(Key::F);
    assert_eq!(cells.modes(), [ScreenMode::FullScreenWithMenuBar, ScreenMode::Standard]);
    cells.press(Key::F);
    assert_eq!(cells.modes(), [ScreenMode::FullScreen, ScreenMode::Standard]);
    cells.press(Key::F);
    assert_eq!(cells.modes(), [ScreenMode::Standard, ScreenMode::Standard]);
}

#[trial]
fn tab_hides_the_bars_and_rails_and_tab_shows_them() {
    let mut cells = Cells::new(1);
    cells.click_body(0);
    assert!(cells.seen[0].slots > 0, "the bars and rails show");
    cells.press(Key::Tab);
    assert_eq!(cells.seen[0].slots, 0, "Tab hides them");
    cells.press(Key::Tab);
    assert!(cells.seen[0].slots > 0, "Tab shows them again");
}

#[trial]
fn deleting_a_layer_from_the_menu_asks_and_proceed_deletes_the_current_layer() {
    let (mut fixture, state) = window(MockState::default());
    choose_delete_layer(&mut fixture);
    fixture.idle(DIALOG_OPEN);
    assert_eq!(state.borrow().document.layers.len(), 3, "the question comes first");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    let document = &state.borrow().document;
    assert_eq!(document.layers.len(), 2);
    assert!(document.layers.iter().all(|layer| layer.name != "Mountains"), "the current layer went");
    assert_eq!(document.current_layer().map(|layer| layer.name.as_str()), Some("Sky"), "the layer below is current");
}

#[trial]
fn the_delete_layer_row_asks_nothing_with_one_layer_left() {
    let mut only = MockState::default();
    only.document.layers.truncate(1);
    let (mut fixture, state) = window(only);
    choose_delete_layer(&mut fixture);
    fixture.idle(DIALOG_OPEN);
    let tree = fixture.tree();
    let asked = fixture.by_role(Role::BUTTON).into_iter().any(|target| tree.name_of(target.node).is_some_and(|name| name == "Delete"));
    assert!(!asked, "no question was asked: the prompt's Delete button is absent (the Layer menu's Delete row may stay open)");
    assert_eq!(state.borrow().document.layers.len(), 1);
}

#[trial]
fn the_tool_keys_pick_move_then_brush_and_shift_b_cycles_to_pencil() {
    let (mut fixture, state) = window(MockState::default());
    fixture.key(Key::M, Modifiers::NONE).settle();
    assert_eq!(state.borrow().tool, Tool::Marquee);
    fixture.key(Key::V, Modifiers::NONE).settle();
    assert_eq!(state.borrow().tool, Tool::Move);
    fixture.key(Key::B, Modifiers::NONE).settle();
    assert_eq!(state.borrow().tool, Tool::Brush);
    fixture.key(Key::B, Modifiers::SHIFT).settle();
    assert_eq!(state.borrow().tool, Tool::Pencil);
}

/// The document canvas: the largest canvas of the window.
fn canvas(fixture: &Fixture) -> Target {
    let area = |target: &Target| target.rect.width() * target.rect.height();
    fixture.by_role(Role::CANVAS).into_iter().max_by(|a, b| area(a).total_cmp(&area(b))).expect("a canvas")
}

/// A window whose document is zoomed in, so a pan has room to move the view.
fn zoomed_window() -> (Fixture, Rc<RefCell<MockState>>) {
    let (mut fixture, state) = window(MockState::default());
    state.borrow_mut().document.view.zoom *= 4.0;
    fixture.settle();
    (fixture, state)
}

fn center_of(state: &Rc<RefCell<MockState>>) -> (f64, f64) {
    let center = state.borrow().document.view.center;
    (center.x, center.y)
}

/// The shapes as large as the canvas's document: the pasteboard's page and one picture per visible layer.
fn page_sized_shapes(fixture: &Fixture) -> usize {
    let page = canvas(fixture).rect;
    fixture.shape_bounds().iter().filter(|rect| rect.width() > page.width() / 3.0 && rect.height() > page.height() / 3.0).count()
}

#[trial]
fn hiding_a_layer_with_its_eye_changes_what_the_canvas_paints() {
    let (mut fixture, state) = window(MockState::default());
    let shown = page_sized_shapes(&fixture);
    let tree = fixture.tree();
    let eye = fixture
        .by_role(Role::BUTTON)
        .into_iter()
        .find(|target| tree.name_of(target.node).is_some_and(|name| name.starts_with("Visibility") && name.contains("Mountains")))
        .expect("the eye of the top layer");
    fixture.click(&eye).settle();
    assert!(!state.borrow().document.layers[0].visible);
    assert_eq!(page_sized_shapes(&fixture), shown - 1, "the hidden layer paints nothing");
}

#[trial]
fn a_hand_drag_pans_the_view() {
    let (mut fixture, state) = zoomed_window();
    fixture.key(Key::H, Modifiers::NONE).settle();
    let before = center_of(&state);
    let target = canvas(&fixture);
    fixture.drag(&target, vec2(-80.0, 0.0)).settle();
    assert!(center_of(&state).0 > before.0, "dragging left moves the view right");
}

#[trial]
fn holding_space_over_the_canvas_pans_with_another_tool_current() {
    let (mut fixture, state) = zoomed_window();
    assert_eq!(state.borrow().tool, Tool::Move);
    let before = center_of(&state);
    let target = canvas(&fixture);
    fixture.frame(vec![pointer::moved(target.center())]);
    fixture.frame(vec![key_press(Key::Space, Modifiers::NONE)]);
    fixture.drag(&target, vec2(-80.0, 0.0));
    fixture.frame(vec![Event::Key { key: Key::Space, physical_key: Some(Key::Space), pressed: false, repeat: false, modifiers: Modifiers::NONE }]);
    fixture.settle();
    assert!(center_of(&state).0 > before.0, "Space made the drag a pan");
    assert_eq!(state.borrow().tool, Tool::Move, "the tool is the same afterwards");
}

#[trial]
fn the_marquee_drag_selects_a_rectangle_and_escape_clears_it() {
    let (mut fixture, state) = window(MockState::default());
    fixture.key(Key::M, Modifiers::NONE).settle();
    let target = canvas(&fixture);
    fixture.drag(&target, vec2(60.0, 40.0)).settle();
    assert!(state.borrow().document.marquee.is_some(), "the drag made a selection");
    fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(state.borrow().document.marquee.is_none(), "Escape cleared it");
}

#[trial]
fn hiding_the_rulers_gives_the_canvas_their_room() {
    let (mut fixture, state) = window(MockState::default());
    let with = state.borrow().document.view.rect;
    state.borrow_mut().document.rulers = false;
    fixture.settle();
    let without = state.borrow().document.view.rect;
    assert!(without.min.x < with.min.x && without.min.y < with.min.y, "the view reaches the canvas edge without the rulers");
}

/// A point on the middle of the top ruler of the window's canvas.
fn top_ruler(fixture: &Fixture, state: &Rc<RefCell<MockState>>) -> Pos2 {
    let view = state.borrow().document.view.rect;
    Pos2::new(view.center().x, (canvas(fixture).rect.min.y + view.min.y) / 2.0)
}

#[trial]
fn a_guide_dragged_from_the_ruler_lands_in_the_document() {
    let (mut fixture, state) = window(MockState::default());
    assert!(state.borrow().document.guides.is_empty());
    let from = top_ruler(&fixture, &state);
    let to = from + vec2(0.0, 120.0);
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![pointer::moved(from + (to - from) * (step as f32 / 4.0))]);
    }
    fixture.frame(vec![pointer::release(to)]).settle();
    assert_eq!(state.borrow().document.guides.len(), 1, "the drag out of the ruler placed a guide");
}

#[trial]
fn picking_a_unit_in_the_ruler_menu_changes_the_rulers_unit() {
    let (mut fixture, state) = window(MockState::default());
    assert_eq!(state.borrow().document.units, UnitKind::Pixels);
    let at = top_ruler(&fixture, &state);
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]).settle();
    let tree = fixture.tree();
    let inches = [Role::MENU_ITEM, Role::MENU_ITEM_CHECK_BOX, Role::MENU_ITEM_RADIO]
        .into_iter()
        .flat_map(|role| fixture.by_role(role))
        .find(|row| tree.name_of(row.node) == Some("in"))
        .expect("the ruler menu lists inches");
    fixture.click(&inches).settle();
    assert_eq!(state.borrow().document.units, UnitKind::Inches);
}

#[trial]
fn closing_the_document_tab_closes_the_document_and_opens_it_again_as_it_was() {
    let (mut fixture, state) = window(MockState::default());
    let top = state.borrow().document.layers[0].id;
    state.borrow_mut().document.set_visible(top, false);
    fixture.settle();
    assert!(state.borrow().document.dirty);
    let tree = fixture.tree();
    let close = fixture
        .by_role(Role::BUTTON)
        .into_iter()
        .find(|target| tree.name_of(target.node).is_some_and(|name| name.starts_with("Close")))
        .expect("the tab's close button");
    fixture.click(&close).settle();
    let document = &state.borrow().document;
    assert!(!document.dirty && document.layers.iter().all(|layer| layer.visible), "the document reopened as it was");
}

#[trial]
fn a_marquee_drag_shows_its_width_and_height_beside_the_pointer() {
    let (mut fixture, _) = window(MockState::default());
    fixture.key(Key::M, Modifiers::NONE).settle();
    let from = canvas(&fixture).center();
    let shows = |fixture: &Fixture, label: &str| fixture.texts().iter().any(|painted| painted.text == label);
    assert!(!shows(&fixture, "W"), "no readout before the drag");
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    for step in 1..=3 {
        fixture.frame(vec![pointer::moved(from + vec2(20.0, 14.0) * step as f32)]);
    }
    assert!(shows(&fixture, "W") && shows(&fixture, "H"), "the readout shows the marquee's width and height while it drags");
    fixture.frame(vec![pointer::release(from + vec2(60.0, 42.0))]).settle();
    assert!(!shows(&fixture, "W"), "the readout ends with the drag");
}
