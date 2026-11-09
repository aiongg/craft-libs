//! The mock editor: its document logic, and the window's keys and menus.

use craft_ui::runtime::access::Role;
use craft_ui::runtime::env::RootEnv;
use craft_ui::shell::ScreenMode;
use craft_ui::testing::input::key_press;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, Frames, Target, pointer, trial};
use craft_ui_gallery::mock::{EditorMock, MockState};
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
        let seen = vec![Seen { mode: ScreenMode::Standard, body: Rect::NOTHING }; count];
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
                    seen[at] = Seen { mode: out.screen_mode, body: out.regions.body };
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

/// The document canvas: the largest canvas of the window.
fn canvas(fixture: &Fixture) -> Target {
    let area = |target: &Target| target.rect.width() * target.rect.height();
    fixture.by_role(Role::CANVAS).into_iter().max_by(|a, b| area(a).total_cmp(&area(b))).expect("a canvas")
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
