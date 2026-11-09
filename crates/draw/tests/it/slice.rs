//! The Slice tools: the overlay's parts and the tool on one canvas, the commands, the dialogs.

use craft_behavior::geometry::Constraint;
use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{CanvasView, EditEvent};
use craft_draw::ui::slice::{
    DivideBy, DivideSlices, DivideSlicesDialog, SliceCommand, SliceEvent, SliceItem, SliceKind, SliceMode, SliceOptions, SliceOptionsDialog,
    SliceOrigin, SliceOverlay, SliceRefusal, SliceRules, SliceTool, SliceToolSettings,
};
use craft_ui::canvas::{Canvas, ToolHost};
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::dialog::{DialogHost, StackDialog as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2};
use std::cell::RefCell;
use std::rc::Rc;

/// What the app keeps of the slices and what the passes reported.
struct App {
    mode: SliceMode,
    items: Vec<SliceItem>,
    selected: Option<usize>,
    locked: bool,
    hide_auto: bool,
    settings: SliceToolSettings,
    rules: SliceRules,
    events: Vec<SliceEvent>,
    id: egui::Id,
    view: CanvasView,
}

struct Harness {
    fixture: Fixture,
    app: Rc<RefCell<App>>,
}

fn user(number: usize, rect: Rect) -> SliceItem {
    SliceItem::new(number, Some(number as u32), rect, SliceOrigin::User)
}

/// A user slice, a layer slice, an auto slice and a small user slice.
fn items() -> Vec<SliceItem> {
    vec![
        user(1, Rect::new(0.0, 0.0, 120.0, 80.0)),
        SliceItem::new(2, Some(2), Rect::new(120.0, 80.0, 240.0, 160.0), SliceOrigin::Layer),
        SliceItem::new(3, None, Rect::new(120.0, 0.0, 240.0, 80.0), SliceOrigin::Auto),
        user(4, Rect::new(10.0, 100.0, 22.0, 108.0)),
    ]
}

fn harness(mode: SliceMode, setup: impl FnOnce(&mut App)) -> Harness {
    let mut state = App {
        mode,
        items: items(),
        selected: None,
        locked: false,
        hide_auto: false,
        settings: SliceToolSettings::default(),
        rules: SliceRules::PHOTOSHOP,
        events: Vec::new(),
        id: egui::Id::NULL,
        view: CanvasView::new(egui::Rect::NOTHING, Point::ZERO, 1.0),
    };
    setup(&mut state);
    let app = Rc::new(RefCell::new(state));
    let tool = Rc::new(RefCell::new(SliceTool::new()));
    let shown_app = Rc::clone(&app);
    let shown_tool = tool;
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, Point::new(120.0, 80.0), 1.0);
        let mut app = shown_app.borrow_mut();
        app.view = view;
        let mut tool = shown_tool.borrow_mut();
        let mut settings = app.settings;
        settings.hide_auto = app.hide_auto;
        tool.mode(app.mode).items(&app.items).selected(app.selected).locked(app.locked).settings(settings).rules(app.rules);
        let (items, selected, rules) = (app.items.clone(), app.selected, app.rules);
        let (pick, locked, hide_auto) = (app.mode == SliceMode::Select, app.locked, app.hide_auto);
        let shown = Canvas::new("Document").view(view).show(ui, |ui, body| {
            let mut overlay = SliceOverlay::new("Slices", &items).pick(pick).locked(locked).hide_auto(hide_auto).rules(rules);
            if let Some(number) = selected {
                overlay = overlay.selected_slice(number);
            }
            overlay.show_at(ui, body.rect)
        });
        ToolHost::new().run(ui, &shown.output, &mut *tool);
        let mut events = tool.take_events();
        if let Some(output) = shown.inner {
            app.id = output.response.id;
            events.extend(output.events);
        }
        for event in &events {
            match event {
                SliceEvent::Select { number, add: false, .. } => app.selected = *number,
                SliceEvent::Move { number, event, .. } | SliceEvent::Resize { number, event, .. } => {
                    let rect = match event {
                        EditEvent::Update { value, .. } => Some(*value),
                        EditEvent::Commit { after, .. } | EditEvent::Set { after, .. } => Some(*after),
                        _ => None,
                    };
                    if let Some(rect) = rect
                        && let Some(item) = app.items.iter_mut().find(|item| item.number == *number)
                    {
                        item.rect = rect;
                    }
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

    fn events(&self) -> Vec<SliceEvent> {
        self.app.borrow().events.clone()
    }

    fn clear(&self) {
        self.app.borrow_mut().events.clear();
    }

    fn rect(&self, number: usize) -> Rect {
        self.app.borrow().items.iter().find(|item| item.number == number).map(|item| item.rect).unwrap_or(Rect::ZERO)
    }

    /// Presses at `from`, moves to `to` in two steps and releases, with `mods` held.
    fn drag(&mut self, from: Pos2, to: Pos2, mods: Modifiers) {
        self.fixture.hold(mods);
        self.hold(from, to);
        frame(self, vec![pointer::release(to)]);
        self.fixture.hold(Modifiers::NONE);
        frame(self, Vec::new());
    }

    /// Presses at `from` and moves to `to`, the button held.
    fn hold(&mut self, from: Pos2, to: Pos2) {
        frame(self, vec![pointer::moved(from)]);
        frame(self, vec![pointer::press(from)]);
        frame(self, vec![pointer::moved(from.lerp(to, 0.5))]);
        frame(self, vec![pointer::moved(to)]);
    }

    fn click(&mut self, at: Pos2) {
        self.fixture.click_at(at);
        frame(self, Vec::new());
    }
}

fn drawn(events: &[SliceEvent]) -> Vec<Rect> {
    events
        .iter()
        .filter_map(|event| match event {
            SliceEvent::Draw(EditEvent::Commit { after, .. }) => Some(*after),
            _ => None,
        })
        .collect()
}

fn committed(events: &[SliceEvent], number: usize, resize: bool) -> Option<Rect> {
    events.iter().find_map(|event| match event {
        SliceEvent::Move { number: n, event: EditEvent::Commit { after, .. }, .. } if !resize && *n == number => Some(*after),
        SliceEvent::Resize { number: n, event: EditEvent::Commit { after, .. }, .. } if resize && *n == number => Some(*after),
        _ => None,
    })
}

#[trial]
fn slice_draw_needs_a_drag_over_two_pixels_except_fixed_size() {
    let mut h = harness(SliceMode::Slice, |_| {});
    let from = h.screen(30.0, 120.0);
    h.drag(from, from + egui::vec2(1.0, 1.0), Modifiers::NONE);
    assert!(drawn(&h.events()).is_empty(), "a 1 pixel drag draws nothing: {:?}", h.events());
    assert!(h.events().iter().any(|event| matches!(event, SliceEvent::Select { .. })), "it is a click, which selects");

    h.clear();
    h.drag(from, from + egui::vec2(40.0, 25.0), Modifiers::NONE);
    assert_eq!(drawn(&h.events()), [Rect::new(30.0, 120.0, 70.0, 145.0)]);

    let mut h = harness(SliceMode::Slice, |app| app.settings = SliceToolSettings::new(Constraint::Size(32.0, 16.0)));
    let from = h.screen(30.5, 120.5);
    h.click(from);
    assert_eq!(drawn(&h.events()), [Rect::new(30.0, 120.0, 62.0, 136.0)], "Fixed Size is made by a click");
}

#[trial]
fn slice_select_moves_the_whole_slice_when_the_press_hits_a_covered_piece() {
    let mut h = harness(SliceMode::Select, |app| app.items[0] = user(1, Rect::new(0.0, 0.0, 120.0, 80.0)));
    // Slice 4 does not overlap; slice 1 lies under nothing, so grab it by a point near its edge.
    let from = h.screen(100.0, 60.0);
    h.drag(from, from + egui::vec2(20.0, 10.0), Modifiers::NONE);
    assert_eq!(committed(&h.events(), 1, false), Some(Rect::new(20.0, 10.0, 140.0, 90.0)), "{:?}", h.events());
    assert!(h.events().iter().any(|event| matches!(event, SliceEvent::Select { number: Some(1), .. })));
}

// A drag of a handle resizes the edge it hangs off; a click on a handle resizes nothing.
#[trial]
fn slice_handle_gestures_a_drag_resizes_the_edge_and_a_click_changes_nothing() {
    {
        let mut h = harness(SliceMode::Select, |app| app.selected = Some(1));
        let right = h.screen(120.0, 40.0);
        h.drag(right, right + egui::vec2(20.0, 30.0), Modifiers::NONE);
        assert_eq!(committed(&h.events(), 1, true), Some(Rect::new(0.0, 0.0, 140.0, 80.0)), "the middle handle moves one edge: {:?}", h.events());
        let corner = h.screen(0.0, 0.0);
        h.clear();
        h.drag(corner, corner + egui::vec2(-10.0, -5.0), Modifiers::NONE);
        assert_eq!(committed(&h.events(), 1, true), Some(Rect::new(-10.0, -5.0, 140.0, 80.0)));
    }
    {
        let mut h = harness(SliceMode::Select, |app| app.selected = Some(1));
        let right = h.screen(120.0, 40.0);
        h.click(right);
        assert!(committed(&h.events(), 1, true).is_none() && committed(&h.events(), 1, false).is_none(), "{:?}", h.events());
        assert_eq!(h.rect(1), Rect::new(0.0, 0.0, 120.0, 80.0));
    }
}

#[trial]
fn auto_slices_do_not_move_and_hide_when_asked() {
    let mut h = harness(SliceMode::Select, |_| {});
    let from = h.screen(180.0, 40.0);
    h.drag(from, from + egui::vec2(20.0, 0.0), Modifiers::NONE);
    assert!(h.events().iter().any(|event| matches!(event, SliceEvent::Select { number: Some(3), .. })), "{:?}", h.events());
    assert!(h.events().iter().all(|event| !matches!(event, SliceEvent::Move { .. })), "an auto slice only selects");

    let mut h = harness(SliceMode::Select, |app| app.hide_auto = true);
    h.click(h.screen(180.0, 40.0));
    assert!(
        h.events().iter().any(|event| matches!(event, SliceEvent::Select { number: None, .. })),
        "a hidden auto slice is nothing: {:?}",
        h.events()
    );
}

#[trial]
fn slice_delete_key_deletes_a_stored_slice_and_leaves_an_auto_one() {
    let mut h = harness(SliceMode::Select, |_| {});
    h.click(h.screen(60.0, 40.0));
    h.clear();
    h.fixture.key(Key::Delete, Modifiers::NONE);
    assert!(h.events().contains(&SliceEvent::Command(SliceCommand::Delete)), "{:?}", h.events());

    h.click(h.screen(180.0, 40.0));
    h.clear();
    h.fixture.key(Key::Delete, Modifiers::NONE);
    assert!(!h.events().contains(&SliceEvent::Command(SliceCommand::Delete)), "an auto slice is not deleted");
}

#[trial]
fn locked_slices_refuse_every_gesture() {
    let mut h = harness(SliceMode::Select, |app| {
        app.locked = true;
        app.selected = Some(1);
    });
    let from = h.screen(60.0, 40.0);
    h.drag(from, from + egui::vec2(30.0, 0.0), Modifiers::NONE);
    let events = h.events();
    assert!(events.contains(&SliceEvent::Refused(SliceRefusal::Locked)), "{events:?}");
    assert!(events.iter().any(|event| matches!(event, SliceEvent::Select { number: None, .. })), "a locked press clears the selection");
    assert!(events.iter().all(|event| !matches!(event, SliceEvent::Move { .. } | SliceEvent::Resize { .. } | SliceEvent::Draw(_))));

    let mut h = harness(SliceMode::Slice, |app| app.locked = true);
    let from = h.screen(30.0, 120.0);
    h.drag(from, from + egui::vec2(40.0, 25.0), Modifiers::NONE);
    assert!(drawn(&h.events()).is_empty() && h.events().contains(&SliceEvent::Refused(SliceRefusal::Locked)));
}

#[trial]
fn slice_badges_hide_on_slices_too_small_to_hold_them() {
    let h = harness(SliceMode::Slice, |_| {});
    let texts = h.fixture.texts();
    assert!(texts.iter().any(|text| text.text == "01"), "the large slice has its badge");
    assert!(!texts.iter().any(|text| text.text == "04"), "the 12 by 8 pixel slice has none");
}

#[trial]
fn slice_keyboard_picks_moves_and_opens_options() {
    let mut h = harness(SliceMode::Select, |_| {});
    h.click(h.screen(60.0, 40.0));
    h.clear();
    h.fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(committed_or_set(&h.events(), 1), Some(Rect::new(1.0, 0.0, 121.0, 80.0)), "{:?}", h.events());
    h.clear();
    h.fixture.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(committed_or_set(&h.events(), 1), Some(Rect::new(1.0, 10.0, 121.0, 90.0)));
    h.clear();
    h.fixture.key(Key::Enter, Modifiers::NONE);
    assert!(h.events().contains(&SliceEvent::Command(SliceCommand::Options)));
    h.clear();
    h.fixture.key(Key::PageDown, Modifiers::NONE);
    assert!(h.events().iter().any(|event| matches!(event, SliceEvent::Select { number: Some(_), .. })), "the focus arriving at a slice selects it");
}

fn committed_or_set(events: &[SliceEvent], number: usize) -> Option<Rect> {
    events.iter().find_map(|event| match event {
        SliceEvent::Move { number: n, event: EditEvent::Set { after, .. }, .. } if *n == number => Some(*after),
        _ => None,
    })
}

#[trial]
fn slice_cancels_on_escape() {
    let mut h = harness(SliceMode::Select, |_| {});
    let from = h.screen(60.0, 40.0);
    h.hold(from, from + egui::vec2(30.0, 0.0));
    h.fixture.key(Key::Escape, Modifiers::NONE);
    assert!(h.events().iter().any(|event| matches!(event, SliceEvent::Move { event: EditEvent::Cancel { .. }, .. })), "{:?}", h.events());
    let count = h.events().len();
    frame(&mut h, vec![pointer::release(from + egui::vec2(40.0, 0.0))]);
    assert_eq!(h.events().len(), count, "the release after Escape is ignored");

    let mut h = harness(SliceMode::Slice, |_| {});
    let from = h.screen(30.0, 120.0);
    h.hold(from, from + egui::vec2(40.0, 25.0));
    h.fixture.key(Key::Escape, Modifiers::NONE);
    assert!(h.events().iter().any(|event| matches!(event, SliceEvent::Draw(EditEvent::Cancel { .. }))), "{:?}", h.events());
}

#[trial]
fn slice_commands_follow_the_selection_the_lock_and_the_document() {
    use SliceOrigin::{Auto, Layer, User};
    assert!(SliceCommand::Promote.enabled(Some(Auto), false, true) && SliceCommand::Promote.enabled(Some(Layer), false, true));
    assert!(!SliceCommand::Promote.enabled(Some(User), false, true));
    assert!(SliceCommand::Delete.enabled(Some(Layer), false, true), "a layer slice is deleted like a user slice");
    assert!(!SliceCommand::Delete.enabled(Some(Auto), false, true));
    assert!(SliceCommand::Divide.enabled(Some(Auto), false, true) && SliceCommand::Options.enabled(Some(User), false, true));
    assert!(!SliceCommand::FromGuides.enabled(None, false, false) && SliceCommand::FromGuides.enabled(None, false, true));
    assert!(
        [SliceCommand::Promote, SliceCommand::Divide, SliceCommand::Delete, SliceCommand::Options, SliceCommand::FromGuides]
            .into_iter()
            .all(|command| !command.enabled(Some(Layer), true, true))
    );
}

#[trial]
fn divide_slices_answer_counts_and_sizes() {
    let ctx = egui::Context::default();
    let mut dialog = DivideSlicesDialog::new(DivideSlices::default());
    assert_eq!(dialog.confirm(&ctx), Some(DivideSlices::new(Some(DivideBy::Count(2)), None)));
    let mut dialog = DivideSlicesDialog::new(DivideSlices::new(Some(DivideBy::Size(64)), Some(DivideBy::Count(3))));
    assert_eq!(dialog.confirm(&ctx), Some(DivideSlices::new(Some(DivideBy::Size(64)), Some(DivideBy::Count(3)))));
    let mut dialog = DivideSlicesDialog::new(DivideSlices::new(None, None));
    assert_eq!(dialog.confirm(&ctx), None, "OK is off while neither axis is on");
}

#[trial]
fn slice_options_fields_follow_the_slice_type() {
    let host = Rc::new(RefCell::new(DialogHost::new()));
    let shown = Rc::clone(&host);
    let mut options = SliceOptions::default();
    options.kind = SliceKind::NoImage;
    shown.borrow_mut().open(SliceOptionsDialog::new(options));
    let mut fixture = Fixture::new(move |ui| host.borrow_mut().run(ui));
    fixture.settle();
    let texts = fixture.texts();
    assert!(texts.iter().any(|text| text.text == "Text"), "No Image has the Text field");
    assert!(!texts.iter().any(|text| text.text == "Alt Tag"), "and no Alt Tag");
    let mut options = SliceOptions::default();
    options.kind = SliceKind::Image;
    shown.borrow_mut().open(SliceOptionsDialog::new(options));
    fixture.settle();
    assert!(fixture.texts().iter().any(|text| text.text == "Alt Tag"), "Image has Alt Tag");
}
