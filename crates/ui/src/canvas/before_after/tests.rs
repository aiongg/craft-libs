//! Tests of the before and after view (task K14).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use egui::accesskit::Role;
use egui::{Event, Key, KeyboardShortcut, Modifiers, PointerButton, Rect, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{BeforeAfter, BeforeAfterMode, BeforeAfterPane, BeforeAfterSide, BeforeAfterStyle};
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::testing::HarnessExt as _;
use crate::theme::{Base, Tokens, TokensOf as _, install};

struct Host {
    mode: BeforeAfterMode,
    fraction: f32,
    rtl: bool,
    captions: bool,
    anywhere: bool,
    hold: Option<Key>,
    field: bool,
    text: String,
    events: Vec<EditEvent<f32>>,
    shown: BeforeAfterMode,
    before: Option<BeforeAfterPane>,
    after: Option<BeforeAfterPane>,
    panes: [Rect; 2],
    image: Rect,
    style: Option<BeforeAfterStyle>,
    strip: f32,
}

impl Host {
    fn new(mode: BeforeAfterMode) -> Self {
        Self {
            mode,
            fraction: 0.5,
            rtl: false,
            captions: false,
            anywhere: true,
            hold: None,
            field: false,
            text: String::new(),
            events: Vec::new(),
            shown: mode,
            before: None,
            after: None,
            panes: [Rect::NOTHING; 2],
            image: Rect::NOTHING,
            style: None,
            strip: 0.0,
        }
    }
}

fn area() -> Rect {
    Rect::from_min_size(pos2(20.0, 60.0), vec2(400.0, 200.0))
}

fn view(ui: &mut egui::Ui, host: &mut Host) {
    direction::apply(ui.ctx(), if host.rtl { Direction::Rtl } else { Direction::Ltr });
    if host.field {
        let name = ui.label("Note");
        ui.add(egui::TextEdit::singleline(&mut host.text)).labelled_by(name.id);
    }
    let tokens = Tokens::of(ui);
    host.style = Some(BeforeAfter::style(&tokens));
    host.strip = tokens.typography.get(craft_theme::TypeRole::Label).line_height + host.style.as_ref().map_or(0.0, |s| s.caption_gap);
    let mut compare = BeforeAfter::new("compare", host.mode).fraction(host.fraction).captions(host.captions).drag_anywhere(host.anywhere);
    if let Some(key) = host.hold {
        compare = compare.original_while(key);
    }
    host.panes = compare.panes(ui, area());
    host.image = host.panes[1].shrink(10.0);
    let out = compare.show(ui, area(), host.image, |_, _| {});
    (host.shown, host.before, host.after) = (out.mode, out.before, out.after);
    if let Some(event) = out.fraction {
        if let Some(&value) = event.value() {
            host.fraction = value;
        }
        host.events.push(event);
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut h =
        Harness::builder().with_size([480.0, 300.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(view, host);
    h.run_steps(3);
    h
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn press(h: &mut Harness<'_, Host>, key: Key, modifiers: Modifiers) {
    h.press(KeyboardShortcut::new(modifiers, key));
}

fn focus_knob(h: &mut Harness<'_, Host>) {
    h.get_by_role_and_label(Role::Slider, "Compare position").focus();
    h.run_steps(2);
}

// SideBySide: two halves `gap_side` apart, before on the left; in RTL before on the right.
#[test]
fn side_by_side_panes_are_gapped_halves_and_mirror_in_rtl() {
    let mut h = harness(Host::new(BeforeAfterMode::SideBySide));
    let (gap, a) = (h.state().style.as_ref().map_or(0.0, |s| s.gap_side), area());
    let [before, after] = h.state().panes;
    assert!(near(before.width(), (a.width() - gap) / 2.0) && near(after.width(), before.width()), "{before:?} {after:?}");
    assert!(near(after.min.x - before.max.x, gap) && before.min.x < after.min.x, "{before:?} {after:?}");
    h.state_mut().rtl = true;
    h.run_steps(3);
    let [before, after] = h.state().panes;
    assert!(before.min.x > after.min.x && near(before.min.x - after.max.x, gap), "{before:?} {after:?}");
    assert_eq!(h.state().before.map(|p| p.side), Some(BeforeAfterSide::Before));
}

// Stacked: two halves `gap_stacked` apart, before above; captions take a strip off each pane.
#[test]
fn stacked_panes_are_gapped_halves() {
    let mut h = harness(Host::new(BeforeAfterMode::Stacked));
    let (gap, a) = (h.state().style.as_ref().map_or(0.0, |s| s.gap_stacked), area());
    let [before, after] = h.state().panes;
    assert!(near(before.height(), (a.height() - gap) / 2.0) && near(after.height(), before.height()), "{before:?} {after:?}");
    assert!(near(after.min.y - before.max.y, gap), "{before:?} {after:?}");
    h.state_mut().captions = true;
    h.run_steps(3);
    let strip = h.state().strip;
    let [captioned, _] = h.state().panes;
    assert!(strip > 0.0 && near(captioned.height(), before.height() - strip), "{captioned:?} {strip}");
}

// Split at 0.25: before's clip ends a quarter across the shown picture from the reading start,
// after's begins there; SplitStacked the same down.
#[test]
fn split_clips_before_on_the_reading_start_side_of_the_divider() {
    let mut host = Host::new(BeforeAfterMode::Split);
    host.fraction = 0.25;
    let mut h = harness(host);
    let (image, Some(before), Some(after)) = (h.state().image, h.state().before, h.state().after) else { panic!("both sides shown") };
    let at = image.min.x + 0.25 * image.width();
    assert!(near(before.clip.max.x, at) && near(after.clip.min.x, at), "{before:?} {after:?}");
    h.state_mut().rtl = true;
    h.run_steps(3);
    let (Some(before), Some(after)) = (h.state().before, h.state().after) else { panic!("both sides shown") };
    let at = image.max.x - 0.25 * image.width();
    assert!(near(before.clip.min.x, at) && near(after.clip.max.x, at), "{before:?} {after:?}");
    h.state_mut().rtl = false;
    h.state_mut().mode = BeforeAfterMode::SplitStacked;
    h.run_steps(3);
    let (Some(before), Some(after)) = (h.state().before, h.state().after) else { panic!("both sides shown") };
    let at = image.min.y + 0.25 * image.height();
    assert!(near(before.clip.max.y, at) && near(after.clip.min.y, at), "{before:?} {after:?}");
}

// A press and drag on the picture report Begin, Update and Commit of the fraction; Esc cancels;
// with `drag_anywhere(false)` a press away from the line reports nothing.
#[test]
fn divider_drag_sets_the_fraction_and_reports_edit_events() {
    let mut h = harness(Host::new(BeforeAfterMode::Split));
    let image = h.state().image;
    let (from, to) = (pos2(image.min.x + 0.2 * image.width(), image.min.y + 10.0), pos2(image.min.x + 0.8 * image.width(), image.min.y + 10.0));
    h.drag(from, to, Modifiers::NONE);
    let events = &h.state().events;
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(events.iter().any(|e| matches!(e, EditEvent::Update { .. })), "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { .. })), "{events:?}");
    assert!(near(h.state().fraction, 0.8), "{}", h.state().fraction);

    // Esc during a drag puts the start value back.
    h.state_mut().events.clear();
    let start = h.state().fraction;
    h.event(Event::PointerMoved(from));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerMoved(to));
    h.run_steps(2);
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { .. })), "{:?}", h.state().events);
    assert!(!near(start, 0.2));

    let mut host = Host::new(BeforeAfterMode::Split);
    host.anywhere = false;
    let mut h = harness(host);
    h.drag(from, to, Modifiers::NONE);
    assert!(h.state().events.is_empty(), "{:?}", h.state().events);
}

// Knob focused: → 1 %, ⇧→ 10 %, Home 0, End 1, one Set each; in RTL ← moves toward the end.
#[test]
fn divider_keys_move_one_percent_and_shift_ten_and_home_end() {
    let mut host = Host::new(BeforeAfterMode::Split);
    host.fraction = 0.25;
    let mut h = harness(host);
    focus_knob(&mut h);
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert!(near(h.state().fraction, 0.26), "{}", h.state().fraction);
    press(&mut h, Key::ArrowRight, Modifiers::SHIFT);
    assert!(near(h.state().fraction, 0.36), "{}", h.state().fraction);
    press(&mut h, Key::ArrowLeft, Modifiers::NONE);
    assert!(near(h.state().fraction, 0.35), "{}", h.state().fraction);
    press(&mut h, Key::Home, Modifiers::NONE);
    assert!(near(h.state().fraction, 0.0), "{}", h.state().fraction);
    press(&mut h, Key::End, Modifiers::NONE);
    assert!(near(h.state().fraction, 1.0), "{}", h.state().fraction);
    assert_eq!(h.state().events.len(), 5);
    assert!(h.state().events.iter().all(|e| matches!(e, EditEvent::Set { .. })), "{:?}", h.state().events);

    h.state_mut().rtl = true;
    h.state_mut().fraction = 0.5;
    h.run_steps(3);
    focus_knob(&mut h);
    press(&mut h, Key::ArrowLeft, Modifiers::NONE);
    assert!(near(h.state().fraction, 0.51), "{}", h.state().fraction);

    h.state_mut().rtl = false;
    h.state_mut().mode = BeforeAfterMode::SplitStacked;
    h.state_mut().fraction = 0.5;
    h.run_steps(3);
    focus_knob(&mut h);
    press(&mut h, Key::ArrowDown, Modifiers::NONE);
    assert!(near(h.state().fraction, 0.51), "{}", h.state().fraction);
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert!(near(h.state().fraction, 0.51), "a horizontal divider ignores ←/→: {}", h.state().fraction);
}

// While the hold key is down the output mode is Original with only the before pane; released, the
// host's mode is back; not while a text field has focus.
#[test]
fn hold_to_show_original_shows_only_before_while_the_key_is_down() {
    let key = |pressed| Event::Key { key: Key::Backslash, physical_key: Some(Key::Backslash), pressed, repeat: false, modifiers: Modifiers::NONE };
    let mut host = Host::new(BeforeAfterMode::Split);
    host.hold = Some(Key::Backslash);
    host.field = true;
    let mut h = harness(host);
    h.event(key(true));
    h.run_steps(2);
    assert_eq!(h.state().shown, BeforeAfterMode::Original);
    assert!(h.state().before.is_some() && h.state().after.is_none());
    h.event(key(false));
    h.run_steps(2);
    assert_eq!(h.state().shown, BeforeAfterMode::Split);
    assert!(h.state().after.is_some());

    h.get_by_role(Role::TextInput).click();
    h.run_steps(2);
    h.event(key(true));
    h.run_steps(2);
    assert_eq!(h.state().shown, BeforeAfterMode::Split);
}

// The knob is a Slider named "Compare position" with value "25%"; the panes are Images named
// "Before" and "After".
#[test]
fn before_after_accesskit_slider_has_name_and_percent_value() {
    let mut host = Host::new(BeforeAfterMode::Split);
    host.fraction = 0.25;
    let h = harness(host);
    let knob = h.get_by_role_and_label(Role::Slider, "Compare position");
    assert_eq!(knob.accesskit_node().data().value(), Some("25%"));
    h.get_by_role_and_label(Role::Image, "Before");
    h.get_by_role_and_label(Role::Image, "After");
}
