//! Reset on a tile: it appears once the story's args changed or its preview was used, and it
//! gives a fresh copy of the story.
use craft_ui::inspect::AccessTree;
use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::Gallery;

struct Fixture {
    ctx: Context,
    gallery: Gallery,
    time: f64,
}

impl Fixture {
    fn new() -> Self {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let mut f = Self { ctx, gallery: Gallery::new(None), time: 0.0 };
        f.frames(3);
        f.select_overview_story("composites/curve-editor/small");
        f
    }

    fn select_overview_story(&mut self, id: &'static str) {
        let story = *self.gallery.stories.iter().find(|s| s.id == id).expect("the story");
        self.gallery.picks.overview_story.insert(story.component, story.id);
        self.gallery.picks.select_component(&self.gallery.stories, story.component);
        self.frames(3);
    }

    fn frame_with(&mut self, events: Vec<Event>) -> Option<AccessTree> {
        self.time += 1.0;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 1600.0))),
            events,
            time: Some(self.time),
            ..RawInput::default()
        };
        let output = self.ctx.run_ui(raw, |ui| self.gallery.show(ui));
        let tree = output.platform_output.accesskit_update.as_ref().and_then(AccessTree::from_update);
        output.drop_without_applying_deltas();
        tree
    }

    fn frames(&mut self, count: usize) -> AccessTree {
        for _ in 1..count {
            self.frame_with(Vec::new());
        }
        self.frame_with(Vec::new()).expect("an AccessKit tree")
    }

    fn bounds_of(&mut self, name: &str, role: Role) -> Option<Rect> {
        let tree = self.frames(2);
        let id = tree
            .walk(tree.root())
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| tree.name(*id).as_deref() == Some(name) && tree.node(*id).is_some_and(|n| n.role() == role))?;
        tree.bounds(id)
    }

    fn reset_shown(&mut self) -> bool {
        self.bounds_of("Reset CurveEditor", Role::Button).is_some()
    }

    fn button(pos: Pos2, pressed: bool) -> Event {
        Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
    }

    fn click(&mut self, pos: Pos2) {
        self.frame_with(vec![Event::PointerMoved(pos)]);
        self.frame_with(Vec::new());
        self.frame_with(vec![Self::button(pos, true), Self::button(pos, false)]);
        self.frame_with(Vec::new());
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame_with(vec![Event::PointerMoved(from)]);
        self.frame_with(Vec::new());
        self.frame_with(vec![Self::button(from, true)]);
        self.frame_with(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
        self.frame_with(vec![Event::PointerMoved(to)]);
        self.frame_with(vec![Self::button(to, false)]);
        self.frame_with(Vec::new());
    }

    fn graph(&mut self) -> Rect {
        self.bounds_of("Tone curve", Role::Group).expect("the curve graph")
    }
}

#[test]
fn dragging_in_the_story_shows_reset_and_reset_restores_a_fresh_story() {
    let mut f = Fixture::new();
    assert!(!f.reset_shown());
    let initial = f.bounds_of("Point 2 of 3", Role::GraphicsObject).expect("the middle point").center();
    let from = initial;
    f.drag(from, from + vec2(20.0, -30.0));
    assert!(f.reset_shown(), "Reset after editing the curve");
    let moved = f.bounds_of("Point 2 of 3", Role::GraphicsObject).expect("the middle point").center();
    assert_ne!(moved, from, "the drag moved the point");
    let button = f.bounds_of("Reset CurveEditor", Role::Button).expect("Reset");
    f.click(button.center());
    assert!(!f.reset_shown(), "Reset hides after it was used");
    let after = f.bounds_of("Point 2 of 3", Role::GraphicsObject).expect("the middle point").center();
    assert_eq!(after, initial, "the curve is the story's initial one");
}

#[test]
fn selecting_a_tile_by_its_empty_area_does_not_show_reset() {
    let mut f = Fixture::new();
    let graph = f.graph();
    // Below the graph, inside the tile.
    let spot = Pos2::new(graph.center().x, graph.bottom() + 2.0);
    f.click(spot);
    assert!(!f.reset_shown());
}

impl Fixture {
    fn key(&mut self, key: egui::Key) {
        let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
        self.frame_with(vec![event(true), event(false)]);
        self.frames(2);
    }

    fn width(&mut self) -> Option<f64> {
        let tree = self.frames(2);
        let id = tree
            .walk(tree.root())
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| tree.name(*id).as_deref() == Some("Width") && tree.node(*id).is_some_and(|n| n.role() == Role::Slider))?;
        tree.node(id)?.numeric_value()
    }
}

/// A story's own state lives under the tile's ids, so Reset gives the story back as it first
/// showed: here the stroke width set in a popover is the first one again. The component's own tab
/// keeps the full-window dialog story's modal layer from covering the tile.
#[test]
fn reset_restores_the_value_a_story_kept() {
    let mut f = Fixture::new();
    f.gallery.open_component("Popover");
    f.frames(3);
    let chip = f.bounds_of("Stroke", Role::Button).expect("the Stroke button");
    f.click(chip.center());
    let first = f.width().expect("the popover is open");
    f.key(egui::Key::ArrowRight);
    let changed = f.width().expect("the popover stays open");
    assert_ne!(changed, first, "the key moved the slider");
    let reset = f.bounds_of("Reset Stroke editor", Role::Button).expect("Reset after using the story");
    f.click(reset.center());
    f.click(chip.center());
    assert_eq!(f.width(), Some(first), "the story starts again");
}
