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

// A preset list shows a check mark on the row holding the field's value, a separator between
// groups and a non-numeric row; any pick reports its index, a number also commits as one edit
// and a choice leaves the value alone.
#[test]
fn a_preset_list_checks_the_current_row_and_reports_the_pick() {
    use super::Preset;
    let mut rig = Rig::new();
    let numbers = LocaleNumbers::from_tag("en");
    let rows = [Preset::choice("Auto", false), Preset::value("8", 8.0), Preset::value("12", 12.0), Preset::Separator, Preset::value("24", 24.0)];
    let value = 12.0;
    let mut events: Vec<EditEvent<f64>> = Vec::new();
    let mut picked = None;
    let run = |rig: &mut Rig, input: Vec<Event>, events: &mut Vec<EditEvent<f64>>, picked: &mut Option<usize>| {
        let mut pick = None;
        rig.frame(input, |ui| {
            let edit = NumberField::new("size", "Size", value, &numbers).range(1.0..=1000.0).preset_list(&rows, &mut pick).show(ui);
            events.extend(edit.event);
        });
        *picked = picked.or(pick);
    };
    run(&mut rig, vec![], &mut events, &mut picked);
    run(&mut rig, vec![], &mut events, &mut picked);
    let chevron = rig.bounds(Role::Button, "Size");
    run(&mut rig, click(chevron.center()), &mut events, &mut picked);
    run(&mut rig, vec![], &mut events, &mut picked);
    let toggled = |rig: &Rig, label: &str| rig.node(Role::MenuItemCheckBox, label).map(|(_, n)| n.toggled());
    assert_eq!(toggled(&rig, "12"), Some(Some(egui::accesskit::Toggled::True)));
    assert_eq!(toggled(&rig, "8"), Some(Some(egui::accesskit::Toggled::False)));
    assert_eq!(toggled(&rig, "Auto"), Some(Some(egui::accesskit::Toggled::False)));

    let auto = rig.bounds(Role::MenuItemCheckBox, "Auto");
    run(&mut rig, click(auto.center()), &mut events, &mut picked);
    run(&mut rig, vec![], &mut events, &mut picked);
    assert_eq!((picked, events.len()), (Some(0), 0), "a choice is only reported");

    picked = None;
    run(&mut rig, click(chevron.center()), &mut events, &mut picked);
    run(&mut rig, vec![], &mut events, &mut picked);
    let item = rig.bounds(Role::MenuItemCheckBox, "24");
    run(&mut rig, click(item.center()), &mut events, &mut picked);
    run(&mut rig, vec![], &mut events, &mut picked);
    run(&mut rig, vec![], &mut events, &mut picked);
    assert_eq!(picked, Some(4), "the index counts the separator");
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].value(), Some(&24.0));
}

// Typing a choice's label (any case) and pressing Enter picks it, with the value unchanged; a
// checked choice shows its label where a mixed field shows the blank.
#[test]
fn a_typed_choice_label_picks_the_choice() {
    use super::Preset;
    let mut rig = Rig::new();
    let numbers = LocaleNumbers::from_tag("en");
    let rows = [Preset::choice("Metrics", true), Preset::choice("Optical", false), Preset::Separator, Preset::value("10", 10.0)];
    let mut picked = None;
    let mut events: Vec<EditEvent<f64>> = Vec::new();
    let run = |rig: &mut Rig, input: Vec<Event>, events: &mut Vec<EditEvent<f64>>, picked: &mut Option<usize>| {
        let mut pick = None;
        rig.frame(input, |ui| {
            let edit = NumberField::new("kern", "Kerning", 0.0, &numbers).mixed(true).preset_list(&rows, &mut pick).show(ui);
            events.extend(edit.event);
        });
        *picked = picked.or(pick);
    };
    run(&mut rig, vec![], &mut events, &mut picked);
    run(&mut rig, vec![], &mut events, &mut picked);
    let node = rig.node(Role::SpinButton, "Kerning").expect("the field");
    let _ = node;
    let field = rig.bounds(Role::SpinButton, "Kerning");
    run(&mut rig, click(field.center()), &mut events, &mut picked);
    run(&mut rig, vec![Event::Text("optical".into())], &mut events, &mut picked);
    let enter = |pressed| Event::Key { key: egui::Key::Enter, physical_key: None, pressed, repeat: false, modifiers: egui::Modifiers::NONE };
    run(&mut rig, vec![enter(true), enter(false)], &mut events, &mut picked);
    run(&mut rig, vec![], &mut events, &mut picked);
    assert_eq!(picked, Some(1), "Optical, whatever the case: {events:?}");
    assert!(events.iter().all(|e| e.value().is_none_or(|v| *v == 0.0)), "the value stays: {events:?}");
}
