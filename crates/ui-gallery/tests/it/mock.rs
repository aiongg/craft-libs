//! The mock editor: its document logic, and the window's keys and menus.

use craft_ui::runtime::access::Role;
use craft_ui::runtime::env::RootEnv;
use craft_ui::shell::ScreenMode;
use craft_ui::testing::input::key_press;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, Frames, Target, pointer, trial};
use craft_ui_gallery::mock::{Document, EditorMock, MockState, Tool};
use egui::{Event, Key, Modifiers, Rect, Vec2, vec2};
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
        frames.root(RootEnv { tab_toggles_chrome: true, ..RootEnv::default() });
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
                    ui.push_id(at, |ui| {
                        let out = EditorMock::new("Editor", state).show(ui);
                        seen[at] = Seen { mode: out.screen_mode, slots: out.regions.slots.len(), body: out.regions.body };
                    });
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
fn deleting_the_current_layer_keeps_a_current_layer_and_marks_the_document_changed() {
    let mut document = Document::default();
    let before = document.layers.len();
    let removed = document.delete_current();
    assert_eq!(removed.map(|layer| layer.name), Some("Mountains".to_owned()));
    assert_eq!(document.layers.len(), before - 1);
    assert!(document.current_layer().is_some());
    assert!(document.dirty);
}

#[trial]
fn the_last_layer_cannot_be_deleted() {
    let mut document = Document::default();
    while document.delete_current().is_some() {}
    assert_eq!(document.layers.len(), 1);
    assert!(document.delete_current().is_none());
}

#[trial]
fn hiding_a_layer_marks_the_document_changed_and_setting_the_same_state_does_not() {
    let mut document = Document::default();
    document.set_visible(0, true);
    assert!(!document.dirty);
    document.set_visible(0, false);
    assert!(document.dirty);
    assert!(!document.layers[0].visible);
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
}

#[trial]
fn the_delete_layer_row_does_nothing_with_one_layer_left() {
    let mut only = MockState::default();
    only.document.layers.truncate(1);
    only.document.current = 0;
    let (mut fixture, state) = window(only);
    choose_delete_layer(&mut fixture);
    fixture.idle(DIALOG_OPEN);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(state.borrow().document.layers.len(), 1, "no question was asked, so Enter deleted nothing");
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
