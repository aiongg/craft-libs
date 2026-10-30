//! Popover tests: the opening press, the close rules, children, focus, placement, resizing and
//! AccessKit.

use egui::accesskit::{HasPopup, Role};
use egui::{CursorIcon, Event, Id, Key, LayerId, Modifiers, Order, PointerButton, Pos2, Rect, Ui, Vec2, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::controls::{Select, Slider};
use crate::direction::Direction;
use crate::testing::a11y;
use crate::theme::{Base, install};

const SCREEN: Vec2 = vec2(480.0, 640.0);
const RESIZABLE: PopoverSize = PopoverSize::Resizable { default: vec2(200.0, 150.0), min: vec2(120.0, 100.0) };

struct Host {
    open: bool,
    size: PopoverSize,
    enter: bool,
    /// Space above and left of the chip.
    top: f32,
    left: f32,
    point: Option<Pos2>,
    extra: bool,
    foreign: bool,
    mode: usize,
    amount: f64,
    chip: Id,
    chip_rect: Rect,
    popover: Id,
    rect: Rect,
    content: Vec2,
    closed: Vec<CloseReason>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            open: false,
            size: PopoverSize::Fit,
            enter: false,
            top: 0.0,
            left: 0.0,
            point: None,
            extra: false,
            foreign: false,
            mode: 0,
            amount: 40.0,
            chip: Id::NULL,
            chip_rect: Rect::NOTHING,
            popover: Id::NULL,
            rect: Rect::NOTHING,
            content: Vec2::ZERO,
            closed: Vec::new(),
        }
    }
}

fn app(ui: &mut Ui, host: &mut Host) {
    ui.add_space(host.top);
    let chip = ui
        .horizontal(|ui| {
            ui.add_space(host.left);
            ui.button("Chip")
        })
        .inner;
    host.chip = chip.id;
    host.chip_rect = chip.rect;
    if chip.clicked() {
        host.open = !host.open;
    }
    if host.foreign {
        egui::Area::new(Id::unique("foreign")).order(Order::Foreground).fixed_pos(pos2(300.0, 300.0)).show(ui.ctx(), |ui| {
            ui.label("Foreign");
        });
    }
    let anchor = host.point.map_or_else(|| PopoverAnchor::Control(chip.clone()), PopoverAnchor::Point);
    let mut popover = Popover::new("p", anchor).size(host.size).close(PopoverClose::default().enter(host.enter)).label("Picker");
    if host.point.is_none() {
        popover = popover.toggle(chip.id);
    }
    let (extra, mode, amount) = (host.extra, &mut host.mode, &mut host.amount);
    let mut id = Id::NULL;
    let shown = popover.show(ui, &mut host.open, |ui, pop| {
        id = pop.id();
        let _ = ui.button("Inner");
        let _ = Slider::new("amount", "Amount", *amount, 0.0..=100.0).show(ui);
        let _ = Select::new("mode", "Mode", mode).option(0, "A").option(1, "B").show(ui);
        if extra {
            for i in 0..8 {
                ui.label(format!("Row {i}"));
            }
        }
    });
    if let Some(shown) = shown {
        host.popover = id;
        host.rect = shown.rect;
        host.content = shown.size;
        host.closed.extend(shown.closed_by);
    }
}

fn harness(dir: Direction) -> HarnessBuilder<Host> {
    Harness::<Host>::builder().with_size(SCREEN).with_context_setup(move |ctx| {
        install(ctx, &Base::suite_default());
        crate::direction::apply(ctx, dir);
    })
}

fn host(host: Host) -> Harness<'static, Host> {
    let mut h = harness(Direction::Ltr).build_ui_state(app, host);
    h.run();
    h
}

fn press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn click<S>(h: &mut Harness<'_, S>, pos: Pos2) {
    h.hover_at(pos);
    h.event(press(pos, true));
    h.run();
    h.event(press(pos, false));
    h.run();
}

fn opened(mut h: Harness<'static, Host>) -> Harness<'static, Host> {
    let chip = h.state().chip_rect.center();
    click(&mut h, chip);
    h.run();
    assert!(h.state().open, "the chip opens the popover");
    h
}

fn options<S>(h: &Harness<'_, S>) -> usize {
    h.query_all_by_role(Role::ListBoxOption).count()
}

fn open_select<S>(h: &mut Harness<'_, S>) {
    let at = h.get_by_role(Role::ComboBox).rect().center();
    click(h, at);
    assert!(options(h) > 0, "the select's list is open");
}

#[test]
fn the_press_that_opens_it_is_not_outside() {
    let mut h = host(Host::default());
    // The host opens it in the frame of a press far away, as a right-click picker does.
    h.state_mut().open = true;
    h.event(press(pos(470.0, 630.0), true));
    h.run();
    h.event(press(pos(470.0, 630.0), false));
    h.run();
    assert!(h.state().open);
    assert!(h.state().closed.is_empty(), "{:?}", h.state().closed);
}

fn pos(x: f32, y: f32) -> Pos2 {
    pos2(x, y)
}

#[test]
fn a_press_outside_closes_it() {
    let mut h = opened(host(Host::default()));
    click(&mut h, pos(470.0, 630.0));
    assert!(!h.state().open);
    assert_eq!(h.state().closed, [CloseReason::Outside]);
    assert!(!any_open(&h.ctx));
}

#[test]
fn a_press_on_the_toggle_control_toggles_instead_of_closing() {
    let mut h = opened(host(Host::default()));
    let chip = h.state().chip_rect.center();
    click(&mut h, chip);
    assert!(!h.state().open, "the click on the chip closes it");
    assert!(h.state().closed.is_empty(), "not as an outside press: {:?}", h.state().closed);
}

#[test]
fn a_press_on_a_child_or_a_foreign_foreground_popup_is_not_outside() {
    let mut h = opened(host(Host { foreign: true, ..Host::default() }));
    open_select(&mut h);
    let b = h.get_by_role_and_label(Role::ListBoxOption, "B").rect().center();
    click(&mut h, b);
    assert_eq!(h.state().mode, 1);
    assert!(h.state().open, "{:?}", h.state().closed);
    let foreign = h.get_by_label("Foreign").rect().center();
    click(&mut h, foreign);
    assert!(h.state().open, "{:?}", h.state().closed);
}

#[test]
fn esc_closes_the_child_first_then_the_popover_and_returns_the_focus() {
    let mut h = opened(host(Host::default()));
    open_select(&mut h);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(options(&h), 0, "the first Esc closes the list");
    assert!(h.state().open);
    h.key_press(Key::Escape);
    h.run();
    assert!(!h.state().open);
    assert_eq!(h.state().closed, [CloseReason::Escape]);
    let chip = h.state().chip;
    assert!(h.ctx.memory(|m| m.has_focus(chip)), "focus is back on the anchor");
}

#[test]
fn enter_closes_only_when_the_host_asks() {
    let mut h = opened(host(Host::default()));
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().open);
    h.state_mut().enter = true;
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().closed, [CloseReason::Enter]);
}

#[test]
fn a_press_on_a_slider_closes_an_open_child_and_the_popover_stays() {
    let mut h = opened(host(Host::default()));
    open_select(&mut h);
    let slider = h.get_by_role(Role::Slider).rect().center();
    h.hover_at(slider);
    h.event(press(slider, true));
    h.run();
    assert_eq!(options(&h), 0, "the press dismissed the list");
    assert!(h.state().open);
}

#[test]
fn tab_past_the_last_control_closes_it() {
    let mut h = opened(host(Host::default()));
    for _ in 0..8 {
        if !h.state().open {
            break;
        }
        h.key_press(Key::Tab);
        h.run();
    }
    assert_eq!(h.state().closed, [CloseReason::FocusLeft]);
}

#[test]
fn the_focus_moves_into_the_popover_on_open() {
    let h = opened(host(Host::default()));
    let focused = h.ctx.memory(|m| m.focused()).and_then(|f| h.ctx.read_response(f)).map(|r| r.layer_id.id);
    assert_eq!(focused, Some(h.state().popover));
}

#[test]
fn a_press_on_the_popover_leaves_an_open_child_above_it() {
    let mut h = opened(host(Host::default()));
    open_select(&mut h);
    let rect = h.state().rect;
    let blank = rect.left_top() + vec2(2.0, 2.0);
    h.hover_at(blank);
    h.event(press(blank, true));
    h.run();
    h.event(press(blank, false));
    h.run();
    let parent = LayerId::new(Order::Foreground, h.state().popover);
    let family = popup_list::family(&h.ctx, h.state().popover);
    let child = LayerId::new(Order::Foreground, family[1]);
    let at = |layer: LayerId| h.ctx.memory(|m| m.layer_ids().position(|l| l == layer));
    assert!(at(child) > at(parent), "the child is above its parent: {:?} {:?}", at(child), at(parent));
}

#[test]
fn it_opens_below_or_above_and_keeps_its_side_while_the_contents_grow() {
    let h = opened(host(Host::default()));
    assert!(h.state().rect.top() >= h.state().chip_rect.bottom() - 1.0, "{:?} under {:?}", h.state().rect, h.state().chip_rect);

    let mut h = opened(host(Host { top: 560.0, ..Host::default() }));
    assert!(h.state().rect.bottom() <= h.state().chip_rect.top() + 1.0, "{:?} over {:?}", h.state().rect, h.state().chip_rect);
    h.state_mut().extra = true;
    h.run();
    h.run();
    assert!(h.state().rect.bottom() <= h.state().chip_rect.top() + 1.0, "still above: {:?}", h.state().rect);
}

#[test]
fn a_point_anchor_keeps_it_inside_the_window() {
    let mut h = host(Host { point: Some(pos(470.0, 630.0)), ..Host::default() });
    h.state_mut().open = true;
    h.run();
    h.run();
    let rect = h.state().rect;
    assert!(Rect::from_min_size(Pos2::ZERO, SCREEN).expand(0.5).contains_rect(rect), "{rect:?}");
}

#[test]
fn it_starts_at_the_leading_edge_of_the_control() {
    let ltr = opened(host(Host { left: 300.0, ..Host::default() }));
    assert!((ltr.state().rect.left() - ltr.state().chip_rect.left()).abs() < 1.5, "{:?}", ltr.state().rect);

    let mut rtl = harness(Direction::Rtl).build_ui_state(app, Host { left: 300.0, ..Host::default() });
    rtl.run();
    let rtl = opened(rtl);
    assert!((rtl.state().rect.right() - rtl.state().chip_rect.right()).abs() < 1.5, "{:?}", rtl.state().rect);
}

fn grip(h: &Harness<'_, Host>, rtl: bool) -> Pos2 {
    let rect = h.state().rect;
    if rtl { rect.left_bottom() + vec2(8.0, -8.0) } else { rect.right_bottom() - vec2(8.0, 8.0) }
}

#[test]
fn the_grip_resizes_within_the_limits_and_overshoot_is_not_banked() {
    let mut h = opened(host(Host { size: RESIZABLE, ..Host::default() }));
    assert_eq!(h.state().content, vec2(200.0, 150.0));
    let at = grip(&h, false);
    h.hover_at(at);
    h.run();
    assert_eq!(h.ctx.output(|o| o.cursor_icon), CursorIcon::ResizeNwSe);
    h.event(press(at, true));
    h.run();
    h.hover_at(at + vec2(30.0, 20.0));
    h.run();
    h.run();
    assert!((h.state().content - vec2(230.0, 170.0)).length() < 1.5, "{:?}", h.state().content);
    h.hover_at(at + vec2(3000.0, 3000.0));
    h.run();
    h.run();
    assert_eq!(h.state().content, SCREEN - Vec2::splat(80.0), "clamped to the window less the margin");
    h.hover_at(at + vec2(50.0, 0.0));
    h.run();
    h.run();
    assert!((h.state().content - vec2(250.0, 150.0)).length() < 1.5, "the overshoot was not banked: {:?}", h.state().content);
    h.hover_at(at - vec2(3000.0, 3000.0));
    h.run();
    h.run();
    assert_eq!(h.state().content, vec2(120.0, 100.0), "clamped to the minimum");
}

#[test]
fn the_size_is_remembered_and_a_corrupt_one_falls_back_to_the_default() {
    let mut h = opened(host(Host { size: RESIZABLE, ..Host::default() }));
    let at = grip(&h, false);
    h.hover_at(at);
    h.event(press(at, true));
    h.run();
    h.hover_at(at + vec2(40.0, 0.0));
    h.run();
    h.event(press(at + vec2(40.0, 0.0), false));
    h.run();
    let grown = h.state().content;
    h.state_mut().open = false;
    h.run();
    h.state_mut().open = true;
    h.run();
    h.run();
    assert_eq!(h.state().content, grown, "read back at the next open");

    let key = size::key(h.state().popover);
    h.state_mut().open = false;
    h.run();
    h.ctx.data_mut(|d| d.insert_persisted(key, [f32::NAN, 10.0]));
    h.state_mut().open = true;
    h.run();
    h.run();
    assert_eq!(h.state().content, vec2(200.0, 150.0));
}

#[test]
fn the_grip_has_no_accessibility_node_and_mirrors() {
    let mut h = opened(host(Host { size: RESIZABLE, ..Host::default() }));
    a11y::assert_labelled(&h);
    let mut rtl = harness(Direction::Rtl).build_ui_state(app, Host { size: RESIZABLE, left: 300.0, ..Host::default() });
    rtl.run();
    let mut rtl = opened(rtl);
    let at = grip(&rtl, true);
    rtl.hover_at(at);
    rtl.run();
    assert_eq!(rtl.ctx.output(|o| o.cursor_icon), CursorIcon::ResizeNeSw);
    let before = rtl.state().content;
    rtl.event(press(at, true));
    rtl.run();
    rtl.hover_at(at - vec2(20.0, 0.0));
    rtl.run();
    rtl.run();
    assert!((rtl.state().content.x - before.x - 20.0).abs() < 1.5, "dragging toward the leading edge grows it");
    h.run();
}

#[test]
fn it_is_registered_while_open_only() {
    let mut h = host(Host::default());
    assert!(!any_open(&h.ctx));
    h.state_mut().open = true;
    h.run();
    assert!(any_open(&h.ctx));
    h.state_mut().open = false;
    h.run();
    assert!(!any_open(&h.ctx));
}

#[test]
fn the_anchor_has_a_popup_and_the_popover_is_a_named_dialog() {
    let mut h = host(Host::default());
    let chip = h.get_by_label("Chip").accesskit_node().data().clone();
    assert_eq!(chip.is_expanded(), Some(false));
    h.state_mut().open = true;
    h.run();
    h.run();
    let chip = h.get_by_label("Chip").accesskit_node().data().clone();
    assert_eq!(chip.has_popup(), Some(HasPopup::Dialog));
    assert_eq!(chip.is_expanded(), Some(true));
    let _dialog = h.get_by_role_and_label(Role::Dialog, "Picker");
}

#[test]
fn the_stories_are_labelled_in_both_directions() {
    for story in crate::story::registry().into_iter().filter(|s| s.id.starts_with("patterns/popover/")) {
        for rtl in [false, true] {
            let mut args = story.defaults();
            for arg in args.iter_mut().filter(|a| a.name == "rtl") {
                arg.value = crate::story::ArgValue::Bool(rtl);
            }
            let mut h = Harness::<crate::story::Args>::builder()
                .with_size(SCREEN)
                .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
                .build_ui_state(move |ui, args| (story.show)(ui, args), args);
            h.run();
            let chip = h.query_all_by_role(Role::Button).next().map(|n| n.rect().center());
            if let Some(chip) = chip {
                click(&mut h, chip);
            }
            a11y::assert_labelled(&h);
        }
    }
}
