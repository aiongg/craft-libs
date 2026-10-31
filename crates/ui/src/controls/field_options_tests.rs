//! The options the dialogs need on the text and number fields: the number field's preset chevron
//! and the text field's width, trailing-arrow room and own id.

use egui::accesskit::{Node, NodeId, Role};
use egui::text_edit::TextEditState;
use egui::{Context, Event, Id, Pos2, RawInput, Rect, vec2};

use super::{NumberField, TextField};
use crate::edit::EditEvent;
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

struct Rig {
    ctx: Context,
    time: f64,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        Self { ctx, time: 0.0, nodes: Vec::new() }
    }

    fn frame(&mut self, events: Vec<Event>, add: impl FnMut(&mut egui::Ui)) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, add);
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.nodes = update.nodes;
        }
        output.drop_without_applying_deltas();
    }

    fn node(&self, role: Role, label: &str) -> Option<&(NodeId, Node)> {
        self.nodes.iter().find(|(_, n)| n.role() == role && n.label() == Some(label))
    }

    fn bounds(&self, role: Role, label: &str) -> Rect {
        let b = self.node(role, label).unwrap_or_else(|| panic!("no {role:?} {label}")).1.bounds().expect("bounds");
        Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32))
    }
}

fn click(pos: Pos2) -> Vec<Event> {
    let button = |pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    vec![Event::PointerMoved(pos), button(true), button(false)]
}

// The chevron sits right after the box in a vertical parent; a click opens the menu; picking a
// preset commits exactly one edit with its value.
#[test]
fn the_preset_chevron_sits_beside_the_box_and_a_pick_commits_one_edit() {
    let mut rig = Rig::new();
    let numbers = LocaleNumbers::from_tag("en");
    let presets: Vec<(String, f64)> = [12.0, 24.0, 36.0].into_iter().map(|v| (format!("{v}"), v)).collect();
    let mut value = 12.0;
    let mut events: Vec<EditEvent<f64>> = Vec::new();
    let run = |rig: &mut Rig, input: Vec<Event>, value: &mut f64, events: &mut Vec<EditEvent<f64>>| {
        rig.frame(input, |ui| {
            let edit = NumberField::new("size", "Size", *value, &numbers).range(1.0..=1000.0).presets(&presets).show(ui);
            if let Some(event) = edit.event {
                if let Some(v) = event.value() {
                    *value = *v;
                }
                events.push(event);
            }
        });
    };
    run(&mut rig, vec![], &mut value, &mut events);
    run(&mut rig, vec![], &mut value, &mut events);
    let field = rig.bounds(Role::SpinButton, "Size");
    let chevron = rig.bounds(Role::Button, "Size");
    assert!((chevron.left() - field.right()).abs() < 1.0, "{field:?} {chevron:?}");
    assert!((chevron.center().y - field.center().y).abs() < 1.0, "{field:?} {chevron:?}");

    run(&mut rig, click(chevron.center()), &mut value, &mut events);
    run(&mut rig, vec![], &mut value, &mut events);
    let item = rig.bounds(Role::MenuItem, "24");
    assert!(events.is_empty(), "opening the menu edits nothing: {events:?}");
    run(&mut rig, click(item.center()), &mut value, &mut events);
    run(&mut rig, vec![], &mut value, &mut events);
    run(&mut rig, vec![], &mut value, &mut events);
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].value(), Some(&24.0));
    assert!(!matches!(events[0], EditEvent::Begin { .. } | EditEvent::Update { .. }), "{:?}", events[0]);
}

/// Where the caret of a focused field at the end of a long text is, as the x of the last
/// vertical line the field painted, and the field's rect.
fn caret_of(arrow: bool, own: Id) -> (Rect, f32) {
    let rig = Rig::new();
    let long = "x".repeat(200);
    let mut rect = Rect::NOTHING;
    let mut caret = f32::NAN;
    for frame in 0..6 {
        let events = match frame {
            3 => vec![Event::Key { key: egui::Key::End, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }],
            _ => vec![],
        };
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))),
            time: Some(f64::from(frame)),
            events,
            ..Default::default()
        };
        let ctx = rig.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            if frame == 1 {
                ui.memory_mut(|m| m.request_focus(own));
            }
            rect = TextField::new("name", "Name", &long).width(180.0).trailing_arrow(arrow).id(own).show(ui).response.rect;
        });
        let mut lines = output.shapes.iter().filter_map(|c| match &c.shape {
            egui::Shape::LineSegment { points, .. } if points[0].x == points[1].x && points[0].y != points[1].y => Some(points[0].x),
            _ => None,
        });
        if let Some(x) = lines.next_back() {
            caret = x;
        }
        output.drop_without_applying_deltas();
    }
    (rect, caret)
}

// `width` makes the box that wide, `trailing_arrow` keeps room clear of the text, and `id` is the
// `TextEdit`'s own id, so a caller loads its state by it.
#[test]
fn text_field_width_trailing_arrow_and_id_apply() {
    let own = Id::unique("caller-id");
    let (plain, plain_caret) = caret_of(false, own);
    let (arrowed, arrow_caret) = caret_of(true, own);
    assert_eq!(plain.width(), 180.0);
    assert_eq!(arrowed.width(), 180.0);
    assert!(plain_caret.is_finite() && arrow_caret.is_finite(), "no caret painted: {plain_caret} {arrow_caret}");
    assert!(plain_caret <= plain.right() && arrow_caret < plain_caret - 8.0, "{plain_caret} {arrow_caret}");

    let mut rig = Rig::new();
    let typed = String::from("abc");
    for frame in 0..4 {
        rig.frame(vec![], |ui| {
            if frame == 1 {
                ui.memory_mut(|m| m.request_focus(own));
            }
            let _ = TextField::new("name", "Name", &typed).id(own).show(ui);
        });
    }
    let state = TextEditState::load(&rig.ctx, own).expect("the TextEdit's state lives under the caller's id");
    assert!(state.cursor.char_range().is_some());
}
