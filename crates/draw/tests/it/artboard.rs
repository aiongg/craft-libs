//! The Artboard tool: the overlay's parts and the tool on one canvas, the bar and the Properties section.

use craft_behavior::kurbo::{Point, Rect, Size, Vec2};
use craft_behavior::{CanvasView, EditEvent};
use craft_draw::ui::add_beside::AddSide;
use craft_draw::ui::artboard::{
    ArtboardBackground, ArtboardBar, ArtboardEvent, ArtboardItem, ArtboardOverlay, ArtboardPreset, ArtboardProperties, ArtboardRules, ArtboardTool,
};
use craft_ui::bars::{Bar, BarKind};
use craft_ui::canvas::{Canvas, ToolHost, ToolPath};
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2};
use std::cell::RefCell;
use std::rc::Rc;

const PATH: ToolPath = ToolPath::Command("the Width and Height fields of the options bar and Add New Artboard");

struct App {
    items: Vec<ArtboardItem>,
    selected: Vec<usize>,
    rules: ArtboardRules,
    events: Vec<ArtboardEvent>,
    view: CanvasView,
}

struct Harness {
    fixture: Fixture,
    app: Rc<RefCell<App>>,
}

fn boards() -> Vec<ArtboardItem> {
    let mut locked = ArtboardItem::new("Locked", Rect::new(400.0, 0.0, 500.0, 100.0));
    locked.locked = true;
    vec![ArtboardItem::new("One", Rect::new(0.0, 0.0, 180.0, 240.0)), ArtboardItem::new("Two", Rect::new(240.0, 0.0, 420.0, 240.0)), locked]
}

fn harness(rules: ArtboardRules, selected: &[usize]) -> Harness {
    let app = Rc::new(RefCell::new(App {
        items: boards(),
        selected: selected.to_vec(),
        rules,
        events: Vec::new(),
        view: CanvasView::new(egui::Rect::NOTHING, Point::ZERO, 1.0),
    }));
    let tool = Rc::new(RefCell::new(ArtboardTool::new()));
    let (shown_app, shown_tool) = (Rc::clone(&app), tool);
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, Point::new(250.0, 120.0), 1.0);
        let mut app = shown_app.borrow_mut();
        app.view = view;
        let mut tool = shown_tool.borrow_mut();
        tool.items(&app.items).selected(&app.selected).rules(app.rules);
        let (items, selected, rules) = (app.items.clone(), app.selected.clone(), app.rules);
        let shown = Canvas::new("Document", PATH)
            .view(view)
            .show(ui, |ui, body| ArtboardOverlay::new("Artboards", &items, &selected).rules(rules).show_at(ui, body.rect));
        ToolHost::new().run(ui, &shown.output, &mut *tool);
        let mut events = tool.take_events();
        if let Some(output) = shown.inner {
            events.extend(output.events);
        }
        for event in &events {
            match event {
                ArtboardEvent::Select { indices, .. } => app.selected.clone_from(indices),
                ArtboardEvent::Resize {
                    index,
                    event: EditEvent::Update { value, .. } | EditEvent::Commit { after: value, .. } | EditEvent::Set { after: value, .. },
                } => {
                    app.items[*index].rect = *value;
                }
                _ => {}
            }
        }
        app.events.extend(events);
    });
    fixture.settle();
    Harness { fixture, app }
}

fn frame(h: &mut Harness, events: Vec<egui::Event>) {
    h.fixture.frame(events);
}

impl Harness {
    fn screen(&self, x: f64, y: f64) -> Pos2 {
        self.app.borrow().view.to_screen(Point::new(x, y))
    }

    fn events(&self) -> Vec<ArtboardEvent> {
        self.app.borrow().events.clone()
    }

    fn hold(&mut self, from: Pos2, to: Pos2) {
        frame(self, vec![pointer::moved(from)]);
        frame(self, vec![pointer::press(from)]);
        frame(self, vec![pointer::moved(from.lerp(to, 0.5))]);
        frame(self, vec![pointer::moved(to)]);
    }

    fn drag(&mut self, from: Pos2, to: Pos2, mods: Modifiers) {
        self.fixture.hold(mods);
        self.hold(from, to);
        frame(self, vec![pointer::release(to)]);
        self.fixture.hold(Modifiers::NONE);
        frame(self, Vec::new());
    }
}

fn by(x: f32, y: f32) -> egui::Vec2 {
    egui::vec2(x, y)
}

fn moved_by(events: &[ArtboardEvent]) -> Option<(Vec<usize>, Vec2, bool)> {
    events.iter().find_map(|event| match event {
        ArtboardEvent::Move { indices, event: EditEvent::Commit { after, .. } | EditEvent::Set { after, .. }, copy, .. } => {
            Some((indices.clone(), *after, *copy))
        }
        _ => None,
    })
}

fn resized(events: &[ArtboardEvent]) -> Option<Rect> {
    events.iter().find_map(|event| match event {
        ArtboardEvent::Resize { event: EditEvent::Commit { after, .. } | EditEvent::Set { after, .. }, .. } => Some(*after),
        _ => None,
    })
}

fn drawn(events: &[ArtboardEvent]) -> Vec<Rect> {
    events
        .iter()
        .filter_map(|event| match event {
            ArtboardEvent::Draw(EditEvent::Commit { after, .. }) => Some(*after),
            _ => None,
        })
        .collect()
}

#[trial]
fn a_drag_on_the_pasteboard_draws_a_board_and_a_short_one_draws_nothing() {
    let mut h = harness(ArtboardRules::PHOTOSHOP, &[0]);
    let from = h.screen(10.0, 300.0);
    h.drag(from, from + by(100.0, 60.0), Modifiers::NONE);
    assert_eq!(drawn(&h.events()), [Rect::new(10.0, 300.0, 110.0, 360.0)], "{:?}", h.events());
    let count = h.events().len();
    h.drag(from, from + by(2.0, 1.0), Modifiers::NONE);
    assert!(drawn(&h.events()[count..]).is_empty(), "a drag under the click slop draws nothing");
}

#[trial]
fn a_press_in_a_board_selects_it_and_a_drag_moves_it_in_whole_pixels() {
    let mut h = harness(ArtboardRules::PHOTOSHOP, &[0]);
    let from = h.screen(300.0, 100.0);
    h.drag(from, from + by(20.4, -9.6), Modifiers::NONE);
    assert!(h.events().iter().any(|event| matches!(event, ArtboardEvent::Select { indices, .. } if indices == &[1])), "{:?}", h.events());
    assert_eq!(moved_by(&h.events()), Some((vec![1], Vec2::new(20.0, -10.0), false)));
}

#[trial]
fn a_locked_board_selects_and_never_moves() {
    let mut h = harness(ArtboardRules::PHOTOSHOP, &[0]);
    let from = h.screen(450.0, 50.0);
    h.drag(from, from + by(30.0, 0.0), Modifiers::NONE);
    assert!(h.events().iter().any(|event| matches!(event, ArtboardEvent::Select { indices, .. } if indices == &[2])));
    assert!(moved_by(&h.events()).is_none() && resized(&h.events()).is_none());
}

#[trial]
fn handles_resize_the_active_board_with_shift_keeping_proportions() {
    let mut h = harness(ArtboardRules::PHOTOSHOP, &[0]);
    let corner = h.screen(180.0, 240.0);
    h.drag(corner, corner + by(40.0, 10.0), Modifiers::NONE);
    assert_eq!(resized(&h.events()), Some(Rect::new(0.0, 0.0, 220.0, 250.0)));
    let mut h = harness(ArtboardRules::PHOTOSHOP, &[0]);
    h.drag(corner, corner + by(60.0, 0.0), Modifiers::SHIFT);
    let rect = resized(&h.events()).expect("a resize");
    assert!((rect.width() / rect.height() - 0.75).abs() < 0.01, "Shift keeps the 3 by 4 proportions: {rect:?}");
}

#[trial]
fn the_second_rule_set_toggles_copies_and_marquees() {
    let mut h = harness(ArtboardRules::ILLUSTRATOR, &[0]);
    let inside = h.screen(300.0, 100.0);
    h.fixture.hold(Modifiers::SHIFT);
    frame(&mut h, vec![pointer::moved(inside)]);
    frame(&mut h, vec![pointer::press(inside)]);
    frame(&mut h, vec![pointer::release(inside)]);
    h.fixture.hold(Modifiers::NONE);
    assert!(
        h.events().iter().any(|event| matches!(event, ArtboardEvent::Select { indices, .. } if indices == &[1, 0])),
        "Shift-click toggles: {:?}",
        h.events()
    );

    let mut h = harness(ArtboardRules::ILLUSTRATOR, &[0]);
    let from = h.screen(20.0, 60.0);
    h.drag(from, from + by(40.0, 0.0), Modifiers::ALT);
    assert_eq!(moved_by(&h.events()), Some((vec![0], Vec2::new(40.0, 0.0), true)), "Alt-drag moves a copy");

    let mut h = harness(ArtboardRules::ILLUSTRATOR, &[]);
    let from = h.screen(200.0, 280.0);
    h.drag(from, h.screen(300.0, 100.0), Modifiers::SHIFT);
    assert!(
        h.events().iter().any(|event| matches!(event, ArtboardEvent::Select { indices, .. } if indices == &[1])),
        "a marquee selects what it touches: {:?}",
        h.events()
    );
}

#[trial]
fn keys_on_a_focused_board_move_resize_delete_and_open() {
    let mut h = harness(ArtboardRules::PHOTOSHOP, &[0]);
    h.fixture.click_at(h.screen(60.0, 100.0));
    h.app.borrow_mut().events.clear();
    h.fixture.key(Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(moved_by(&h.events()), Some((vec![0], Vec2::new(10.0, 0.0), false)));
    h.fixture.key(Key::ArrowDown, Modifiers::ALT);
    assert_eq!(resized(&h.events()), Some(Rect::new(0.0, 0.0, 180.0, 241.0)), "Alt and the arrows resize from the end edges");
    h.fixture.key(Key::Delete, Modifiers::NONE);
    assert!(h.events().iter().any(|event| matches!(event, ArtboardEvent::Delete { indices, .. } if indices == &[0])));
    h.fixture.key(Key::Enter, Modifiers::NONE);
    assert!(h.events().iter().any(|event| matches!(event, ArtboardEvent::OpenOptions { index: 0, .. })));
}

#[trial]
fn escape_cancels_a_move_and_the_second_rule_set_leaves_on_escape() {
    let mut h = harness(ArtboardRules::PHOTOSHOP, &[0]);
    let from = h.screen(60.0, 100.0);
    h.hold(from, from + by(30.0, 0.0));
    h.fixture.key(Key::Escape, Modifiers::NONE);
    assert!(h.events().iter().any(|event| matches!(event, ArtboardEvent::Move { event: EditEvent::Cancel { .. }, .. })), "{:?}", h.events());

    let mut h = harness(ArtboardRules::ILLUSTRATOR, &[0]);
    h.fixture.key(Key::Escape, Modifiers::NONE);
    assert!(h.events().contains(&ArtboardEvent::Leave));
}

#[trial]
fn the_plus_buttons_answer_becomes_add_beside_and_alt_copies() {
    let mut tool = ArtboardTool::new();
    tool.items(&boards()).selected(&[1]);
    tool.add_beside(AddSide::Right, false).add_beside(AddSide::Bottom, true);
    let events = tool.take_events();
    let sides: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ArtboardEvent::AddBeside { index, side, copy, .. } => Some((*index, *side, *copy)),
            _ => None,
        })
        .collect();
    assert_eq!(sides, [(1, AddSide::Right, false), (1, AddSide::Bottom, true)]);
}

fn presets() -> Vec<ArtboardPreset> {
    vec![ArtboardPreset::new("Square", Size::new(100.0, 100.0))]
}

#[trial]
fn make_portrait_shows_only_while_the_board_is_wider_than_tall() {
    let show = |rect: Rect| {
        let board = ArtboardItem::new("One", rect);
        let presets = presets();
        let mut fixture = Fixture::new(move |ui| {
            Bar::new("Options", BarKind::Options).show(ui, |ui, _| ArtboardBar::new(Some(&board), &presets).show(ui));
        });
        fixture.settle();
        fixture.texts().iter().map(|text| text.text.clone()).collect::<Vec<_>>()
    };
    let wide = show(Rect::new(0.0, 0.0, 200.0, 100.0));
    assert!(wide.contains(&"Make Portrait".to_owned()) && !wide.contains(&"Make Landscape".to_owned()), "{wide:?}");
    let tall = show(Rect::new(0.0, 0.0, 100.0, 200.0));
    assert!(tall.contains(&"Make Landscape".to_owned()) && !tall.contains(&"Make Portrait".to_owned()), "{tall:?}");
    let square = show(Rect::new(0.0, 0.0, 100.0, 100.0));
    assert!(!square.contains(&"Make Portrait".to_owned()) && !square.contains(&"Make Landscape".to_owned()), "{square:?}");
}

#[trial]
fn properties_background_other_asks_for_a_color() {
    let board = ArtboardItem {
        background: ArtboardBackground::Custom(craft_theme::Srgba::rgb(0.9, 0.4, 0.2)),
        ..ArtboardItem::new("One", Rect::new(0.0, 0.0, 100.0, 100.0))
    };
    let asked = Rc::new(RefCell::new(None));
    let seen = Rc::clone(&asked);
    let presets = presets();
    let mut fixture = Fixture::new(move |ui| {
        if let Some(request) = ArtboardProperties::new(&board, &presets).show(ui).pick_color {
            *seen.borrow_mut() = Some(request);
        }
    });
    fixture.settle();
    let swatch = fixture.by_label("Artboard Background Color").expect("the custom colour swatch");
    fixture.click(&swatch);
    let request = asked.borrow().clone().expect("a click on the swatch asks for a colour");
    assert_eq!(request.key, "artboard.background");
}
