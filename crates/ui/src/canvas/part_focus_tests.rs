use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Sense, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{Part, PartFocus};
use crate::canvas::Claim;
use crate::canvas::claim::PrimaryPress;
use crate::theme::{Base, install};

/// A host with three parts on a 400 × 300 canvas; a press on a part is the overlay's.
struct Host {
    picked: Option<u64>,
    selected: u64,
    keyboard: bool,
    focused: bool,
}

const KEYS: [u64; 3] = [10, 20, 30];

fn centre(key: u64) -> Pos2 {
    pos2(40.0 + key as f32 * 5.0, 140.0)
}

fn harness() -> Harness<'static, Host> {
    let host = Host { picked: None, selected: 20, keyboard: false, focused: false };
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            ui.add_space(40.0);
            let (_, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let parts: Vec<Part> = KEYS
                .iter()
                .map(|&key| Part {
                    key,
                    name: format!("Point {key}"),
                    value: Some(format!("{key} px")),
                    rect: Rect::from_center_size(centre(key), vec2(12.0, 12.0)),
                    selected: key == host.selected,
                })
                .collect();
            let id = ui.make_persistent_id("parts");
            if let Some(press) = PrimaryPress::current(ui.ctx())
                && parts.iter().any(|p| p.rect.contains(press.origin))
            {
                Claim::take(ui.ctx(), canvas.id, id);
            }
            let r = PartFocus::new(id, "Points").parts(&parts).picked(host.picked).value("3 points").show(ui, &canvas);
            if let Some(pick) = r.pick {
                host.picked = pick;
            }
            (host.keyboard, host.focused) = (r.keyboard, r.focused);
        },
        host,
    )
}

fn click(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run();
}

fn key(h: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) -> Option<u64> {
    h.key_press_modifiers(modifiers, key);
    h.run();
    h.state().picked
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

#[test]
fn parts_are_one_tab_stop_picked_with_the_page_keys() {
    let mut h = harness();
    h.run();
    h.key_press(Key::Tab);
    h.run();
    assert!(h.state().focused && h.state().keyboard);
    assert_eq!(h.state().picked, Some(10), "gaining focus picks the first part");
    let group = h.get_by_label("Points");
    let stops = descendants(group.accesskit_node()).into_iter().filter(|n| n.data().supports_action(egui::accesskit::Action::Focus)).count();
    assert_eq!(stops, 0, "one Tab stop, not one per part");
    let none = Modifiers::NONE;
    assert_eq!(key(&mut h, none, Key::PageDown), Some(20));
    assert_eq!(key(&mut h, none, Key::PageDown), Some(30));
    assert_eq!(key(&mut h, none, Key::PageDown), Some(30), "the last part stays");
    assert_eq!(key(&mut h, none, Key::PageUp), Some(20));
    assert_eq!(key(&mut h, none, Key::Home), Some(10));
    assert_eq!(key(&mut h, none, Key::End), Some(30));
    assert_eq!(key(&mut h, Modifiers::SHIFT, Key::PageUp), Some(30), "⇧ Page keys are the overlay's");
    // The arrows and Esc stay with the group.
    key(&mut h, none, Key::ArrowLeft);
    key(&mut h, none, Key::Escape);
    assert!(h.state().focused, "arrows and Esc keep the focus");
}

#[test]
fn a_press_on_a_part_gives_focus_without_the_ring() {
    let mut h = harness();
    h.run();
    click(&mut h, pos2(300.0, 300.0));
    assert!(!h.state().focused, "a press off the parts leaves the focus alone");
    click(&mut h, centre(20));
    assert!(h.state().focused, "the release of a press on a part focuses the group");
    assert!(!h.state().keyboard, "no ring after the pointer");
    assert_eq!(key(&mut h, Modifiers::NONE, Key::PageDown), Some(10));
    assert!(h.state().keyboard, "the ring after keyboard input");
}

#[test]
fn parts_are_graphics_objects_in_a_group() {
    let mut h = harness();
    h.state_mut().picked = Some(30);
    h.run();
    let group = h.get_by_label("Points");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    assert_eq!(group.accesskit_node().value().as_deref(), Some("3 points"));
    let objects: Vec<_> = descendants(group.accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected()))
        .collect();
    let want: Vec<_> = KEYS.iter().map(|&k| (format!("Point {k}"), format!("{k} px"), Some(k == 20))).collect();
    assert_eq!(objects, want);
    assert_eq!(group.accesskit_node().active_descendant().and_then(|n| n.label()).as_deref(), Some("Point 30"));
    // A picked part that is gone is cleared.
    h.state_mut().picked = Some(99);
    h.run();
    assert_eq!(h.state().picked, None);
}
