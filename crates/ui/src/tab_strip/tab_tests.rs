//! Tab tests: layout of the parts per platform and direction, states, the × as a part of the
//! tab, AccessKit and the story.

use egui::accesskit::Role;
use egui::{Color32, Context, Event, Modifiers, PointerButton, Pos2, Rect, Sense, Shape, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::story::Args;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

fn harness<State>(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 200.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

#[allow(clippy::expect_used)]
fn style() -> TabStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(Tab::<u8>::style(&Tokens::of(ui)))).drop_without_applying_deltas();
    style.expect("style")
}

fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

/// Every shape a frame of `add` paints.
fn shapes(add: impl FnMut(&mut Ui)) -> Vec<Shape> {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut add = add;
    let output = ctx.run_ui(egui::RawInput::default(), |ui| add(ui));
    let mut out = Vec::new();
    output.shapes.iter().for_each(|s| flatten(&s.shape, &mut out));
    output.drop_without_applying_deltas();
    out
}

fn rect() -> Rect {
    Rect::from_min_size(pos2(10.0, 0.0), vec2(200.0, 26.0))
}

fn metrics() -> TabMetrics {
    TabMetrics { label: 60.0, detail: 30.0, mark: 6.0 }
}

const BOTH: TabParts = TabParts { icon: true, close: true };

#[test]
fn the_close_goes_to_the_reading_start_on_macos_and_the_reading_end_elsewhere() {
    let s = style();
    let near = |a: f32, b: f32| (a - b).abs() < 0.01;
    let ltr_mac = s.layout(rect(), &metrics(), BOTH, false, true);
    let ltr_other = s.layout(rect(), &metrics(), BOTH, false, false);
    assert!(near(ltr_mac.close.expect("×").left(), rect().left() + s.pad_x));
    assert!(near(ltr_other.close.expect("×").right(), rect().right() - s.pad_x));
    let rtl_mac = s.layout(rect(), &metrics(), BOTH, true, true);
    let rtl_other = s.layout(rect(), &metrics(), BOTH, true, false);
    assert!(near(rtl_mac.close.expect("×").right(), rect().right() - s.pad_x));
    assert!(near(rtl_other.close.expect("×").left(), rect().left() + s.pad_x));
}

#[test]
fn parts_follow_reading_order_and_mirror() {
    let s = style();
    let ltr = s.layout(rect(), &metrics(), BOTH, false, false);
    let (icon, label, detail, mark) = (ltr.icon.expect("icon"), ltr.label, ltr.detail.expect("detail"), ltr.mark.expect("mark"));
    assert!(icon.right() < label.left() && label.right() < detail.left() && detail.right() <= mark.left() + 0.01);
    assert!((mark.left() - detail.right()).abs() < 0.01, "the mark follows the detail with no gap");
    assert!(mark.right() < ltr.close.expect("×").left());
    let rtl = s.layout(rect(), &metrics(), BOTH, true, false);
    assert!(rtl.icon.expect("icon").left() > rtl.label.right());
    assert!(rtl.label.left() > rtl.detail.expect("detail").right());
    assert!((rtl.label.width() - ltr.label.width()).abs() < 0.01);
}

#[test]
fn the_label_is_cut_before_the_detail_and_a_natural_width_cuts_nothing() {
    let s = style();
    let natural = s.natural_width(&metrics(), BOTH);
    let fits = s.layout(Rect::from_min_size(rect().min, vec2(natural, 26.0)), &metrics(), BOTH, false, false);
    assert!(!fits.label_cut && !fits.detail_cut);
    assert_eq!(fits.label.width(), 60.0);
    let narrower = s.layout(Rect::from_min_size(rect().min, vec2(natural - 20.0, 26.0)), &metrics(), BOTH, false, false);
    assert!(narrower.label_cut && !narrower.detail_cut);
    assert!((narrower.label.width() - 40.0).abs() < 0.01 && narrower.detail.expect("detail").width() == 30.0);
    let tiny = s.layout(Rect::from_min_size(rect().min, vec2(60.0, 26.0)), &metrics(), BOTH, false, false);
    assert!(tiny.label_cut && tiny.detail_cut);
    assert_eq!(tiny.label.width(), 0.0);
}

#[test]
fn a_tab_without_icon_detail_mark_or_close_is_the_label_between_the_paddings() {
    let s = style();
    let m = TabMetrics { label: 50.0, detail: 0.0, mark: 0.0 };
    assert_eq!(s.natural_width(&m, TabParts::default()), 50.0 + 2.0 * s.pad_x);
    let l = s.layout(rect(), &m, TabParts::default(), false, false);
    assert!(l.icon.is_none() && l.close.is_none() && l.detail.is_none() && l.mark.is_none());
    assert_eq!(l.label.left(), rect().left() + s.pad_x);
}

struct Clicks {
    tab: u32,
    close: u32,
}

fn view<'a>(strip: egui::Id, text: &'a TabStripText, active: bool) -> TabView<'a> {
    TabView {
        strip,
        active,
        focused: false,
        large: false,
        divider: false,
        drop_highlight: false,
        sense: Sense::CLICK,
        position: 2,
        text,
        controls: None,
    }
}

fn one_tab(ui: &mut Ui, clicks: &mut Clicks, tab: &Tab<u8>, active: bool) {
    let text = TabStripText::default();
    let strip = ui.make_persistent_id("strip");
    let (_, at) = ui.allocate_space(vec2(220.0, 26.0));
    let out = tab.show_at(ui, at, &view(strip, &text, active));
    clicks.tab += u32::from(out.tab.clicked());
    clicks.close += u32::from(out.close.is_some_and(|c| c.clicked()));
}

fn press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

#[test]
fn a_press_on_the_close_closes_and_does_not_activate_the_tab() {
    let tab = Tab::new(1u8, "Photo").closable(true);
    let mut harness = harness(Direction::Ltr).build_ui_state(|ui, clicks: &mut Clicks| one_tab(ui, clicks, &tab, false), Clicks { tab: 0, close: 0 });
    harness.run();
    let close = harness.get_by_label("Close Photo").rect().center();
    harness.event(Event::PointerMoved(close));
    harness.run();
    harness.event(press(close, true));
    harness.run();
    harness.event(press(close, false));
    harness.run();
    assert_eq!((harness.state().tab, harness.state().close), (0, 1));
    let body = harness.get_by_role(Role::Tab).rect().left_center() + vec2(4.0, 0.0);
    harness.event(Event::PointerMoved(body));
    harness.run();
    harness.event(press(body, true));
    harness.run();
    harness.event(press(body, false));
    harness.run();
    assert_eq!((harness.state().tab, harness.state().close), (1, 1));
}

#[test]
fn a_tab_is_a_labelled_tab_with_a_close_button_inside_and_no_tab_stop() {
    let tab = Tab::new(1u8, "Photo").detail("RGB/8").modified(true).closable(true);
    let mut harness = harness(Direction::Ltr).build_ui_state(|ui, clicks: &mut Clicks| one_tab(ui, clicks, &tab, true), Clicks { tab: 0, close: 0 });
    harness.run();
    a11y::assert_labelled(&harness);
    let node = harness.get_by_role(Role::Tab);
    let access = node.accesskit_node();
    assert_eq!(access.label().as_deref(), Some("Photo, RGB/8, modified"));
    assert_eq!((access.is_selected(), access.position_in_set(), access.size_of_set()), (Some(true), Some(2), None));
    let close = harness.get_by_label("Close Photo");
    assert_eq!(close.accesskit_node().role(), Role::Button);
    let ancestors: Vec<_> = std::iter::successors(close.accesskit_node().parent(), |n| n.parent()).map(|n| n.id()).collect();
    assert!(ancestors.contains(&access.id()), "the × is inside its tab");
    assert!(a11y::tab_order(&mut harness).is_empty(), "a strip, not its tabs, is the Tab stop");
}

#[test]
fn a_tab_without_close_has_no_button() {
    let tab = Tab::new(1u8, "Panel");
    let harness = harness(Direction::Ltr).build_ui_state(|ui, clicks: &mut Clicks| one_tab(ui, clicks, &tab, false), Clicks { tab: 0, close: 0 });
    assert!(harness.query_by_role(Role::Button).is_none());
}

fn fills(shapes: &[Shape]) -> Vec<Color32> {
    shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { (r.fill.a() > 0).then_some(r.fill) } else { None }).collect()
}

fn label_colour(shapes: &[Shape], text: &str) -> Option<Color32> {
    shapes.iter().find_map(|s| match s {
        Shape::Text(t) if t.galley.job.text == text => t.galley.job.sections.first().map(|s| s.format.color),
        _ => None,
    })
}

#[test]
fn the_active_tab_takes_the_active_fill_and_the_text_colour() {
    let s = style();
    let tab = Tab::new(1u8, "Photo");
    let paint = |active| {
        shapes(|ui| {
            let mut clicks = Clicks { tab: 0, close: 0 };
            one_tab(ui, &mut clicks, &tab, active);
        })
    };
    let (on, off) = (paint(true), paint(false));
    assert_eq!(fills(&on), [s.active]);
    assert!(fills(&off).is_empty(), "an inactive tab at rest draws no fill: the strip shows");
    assert_eq!(label_colour(&on, "Photo"), Some(s.text));
    assert_eq!(label_colour(&off, "Photo"), Some(s.inactive_text));
}

/// A tab under the pointer takes the active tab's fill, and its label stays the inactive colour
/// (PhotoCraft's tabs do not light up).
#[test]
fn a_tab_under_the_pointer_takes_the_active_fill_and_keeps_its_label_colour() {
    let s = style();
    let tab = Tab::new(1u8, "Photo");
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut painted = Vec::new();
    for frame in 0..3 {
        let mut input = egui::RawInput::default();
        if frame > 0 {
            input.events.push(Event::PointerMoved(rect().center()));
        }
        let output = ctx.run_ui(input, |ui| {
            let mut clicks = Clicks { tab: 0, close: 0 };
            one_tab(ui, &mut clicks, &tab, false);
        });
        painted.clear();
        output.shapes.iter().for_each(|s| flatten(&s.shape, &mut painted));
        output.drop_without_applying_deltas();
    }
    assert_eq!(fills(&painted), [s.active]);
    assert_eq!(label_colour(&painted, "Photo"), Some(s.inactive_text));
}

#[test]
fn the_mark_detail_and_progress_are_drawn() {
    let s = style();
    let tab = Tab::new(1u8, "Photo").detail("RGB/8").modified(true).progress(Some(0.5));
    let painted = shapes(|ui| {
        let mut clicks = Clicks { tab: 0, close: 0 };
        one_tab(ui, &mut clicks, &tab, false);
    });
    assert_eq!(label_colour(&painted, "*"), Some(s.inactive_text));
    assert_eq!(label_colour(&painted, "RGB/8"), Some(s.detail));
    let fills = fills(&painted);
    assert_eq!(fills.iter().filter(|c| **c == s.track).count(), 1);
    assert_eq!(fills.iter().filter(|c| **c == s.fill).count(), 1);
}

#[test]
fn the_story_shows_in_both_directions_and_both_sizes() {
    let story = crate::story::registry().into_iter().find(|s| s.id == "composites/tab/states").expect("story");
    for dir in [Direction::Ltr, Direction::Rtl] {
        for large in [false, true] {
            let mut args = Args::new().with_bool("large", large).with_bool("dividers", true).with_number("width", 180.0, 40.0, 320.0);
            let harness = harness(dir).build_ui(|ui| (story.show)(ui, &mut args));
            a11y::assert_labelled(&harness);
        }
    }
}
