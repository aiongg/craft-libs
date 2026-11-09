//! The choice family: `RadioGroup` and `Radio` are one Tab stop where arrows move focus and a click or Space selects;
//! `Rating` and `Segmented` change their value from clicks and keys.

use crate::contract::kit::{PlainStyle, Probe, Seen};
use craft_behavior::{IconId, PartKey, Sense};
use craft_ui::choice::{Checkbox, Radio, RadioGroup, Rating, Segmented};
use craft_ui::contract::{
    Bind, Common, Component, Cx, Entry, MeasureCx, PaintUi, PartsCx, Props as _, Role, Show as _, ShowContainer as _, Validity,
};
use craft_ui::runtime::env::{Message, Word, WordRows, WordsOverride, with_env};
use craft_ui::runtime::overlay::{OpenFilter, any_open};
use craft_ui::testing::focus::{egui_focused, is_focused, key_release};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{click, moved};
use craft_ui::testing::{Fixture, Frames, Target, trial};
use egui::{Color32, Id, Key, Modifiers, Pos2, Rect, Response, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What the last frame showed.
#[derive(Default)]
struct Shown {
    radios: Vec<(Rect, Id)>,
    before: Option<Id>,
    group: Option<Id>,
}

fn frame(ui: &mut Ui, value: &mut u8, shown: &mut Shown) {
    let seen = RefCell::new(Seen::default());
    shown.before = Some(Probe::new(&seen).label("Before").id_salt("before").show(ui).id);
    let out = RadioGroup::new("Size", Bind::bound(value)).id_salt("group").show(ui, |ui, _| {
        // The body runs in a measuring run and then the real run; the last run's radios are the shown ones.
        shown.radios.clear();
        ui.horizontal(|ui| {
            for v in 1..=3u8 {
                let response = Radio::new(v).label(format!("Option {v}")).id_salt(("radio", v)).show(ui);
                shown.radios.push((response.rect, response.id));
            }
        });
    });
    shown.group = Some(out.output.response.id);
}

fn tap(frames: &mut Frames, key: Key, value: &mut u8, shown: &mut Shown) {
    frames.frame(vec![key_press(key, Modifiers::NONE)], |ui| frame(ui, value, shown));
    frames.frame(vec![key_release(key, Modifiers::NONE)], |ui| frame(ui, value, shown));
}

/// Skeleton `frames_pointer::radio_group_click_selects_one_tab_stop_arrows_move`.
#[trial]
fn radio_group_click_selects_one_tab_stop_arrows_move() {
    let mut frames = Frames::new();
    let (mut value, mut shown) = (1u8, Shown::default());
    frames.idle(3, |ui| frame(ui, &mut value, &mut shown));
    assert_eq!(shown.radios.len(), 3);
    let rects: Vec<Rect> = shown.radios.iter().map(|(rect, _)| *rect).collect();
    assert!(rects[0].max.x <= rects[1].min.x && rects[1].max.x <= rects[2].min.x, "radios shown in a horizontal layout sit side by side");

    let second = rects[1].center();
    click(&mut frames, second, &mut |ui| frame(ui, &mut value, &mut shown));
    assert_eq!(value, 2, "a click on the second radio selects it");

    // The group is one Tab stop: Tab reaches the button, then the group's root, then wraps. No radio holds egui focus.
    let (group, before) = (shown.group, shown.before);
    let mut order = Vec::new();
    for _ in 0..3 {
        tap(&mut frames, Key::Tab, &mut value, &mut shown);
        order.push(egui_focused(&frames.ctx));
    }
    assert_eq!(order.iter().filter(|focused| **focused == group).count(), 1, "one stop in three Tab presses: {order:?}");
    assert!(order.iter().all(|focused| *focused == group || *focused == before || focused.is_none()), "no radio takes egui focus: {order:?}");

    while egui_focused(&frames.ctx) != group {
        tap(&mut frames, Key::Tab, &mut value, &mut shown);
    }
    let focused_member = |frames: &Frames, shown: &Shown| shown.radios.iter().position(|(_, id)| is_focused(&frames.ctx, *id));
    assert_eq!(focused_member(&frames, &shown), Some(1), "Tab arrives at the selected member");
    tap(&mut frames, Key::ArrowRight, &mut value, &mut shown);
    assert_eq!(value, 2, "ArrowRight moves focus, not the selection");
    assert_eq!(
        (focused_member(&frames, &shown), egui_focused(&frames.ctx)),
        (Some(2), group),
        "focus moved to the third member; egui focus stays on the group"
    );
    tap(&mut frames, Key::Space, &mut value, &mut shown);
    assert_eq!(value, 3, "Space selects the focused radio");
    tap(&mut frames, Key::ArrowLeft, &mut value, &mut shown);
    tap(&mut frames, Key::ArrowLeft, &mut value, &mut shown);
    tap(&mut frames, Key::Enter, &mut value, &mut shown);
    assert_eq!(value, 3, "two ArrowLeft presses move focus to the first radio; Enter is the dialog default's and selects nothing");
    tap(&mut frames, Key::Space, &mut value, &mut shown);
    assert_eq!(value, 1, "Space selects the focused first radio");
}

/// Members shown in nested layouts (a non-arena container, a horizontal `Ui`) with an ordinary control between them
/// join the group's arena: a click on each selects it, the control between them is not picked, and AccessKit reports
/// the group with each member a radio button at its place in the set.
#[trial]
fn members_of_a_group_form_in_a_nested_layout_join_its_arena() {
    use crate::contract::kit::Panel;
    use craft_ui::contract::Role;
    let mut frames = Frames::new();
    let mut value = 1u8;
    let rects = RefCell::new(Vec::<(Rect, Id)>::new());
    let mut group = None;
    let mut show = |ui: &mut Ui, value: &mut u8| {
        let out = RadioGroup::new("Pages", Bind::bound(value)).id_salt("group").show(ui, |ui, _| {
            // The body runs in a measuring run and then the real run; the last run's members are the shown ones.
            rects.borrow_mut().clear();
            Panel::new().id_salt("outer").show(ui, |ui, _| {
                ui.horizontal(|ui| {
                    let first = Radio::new(1u8).label("All").id_salt("all").show(ui);
                    let seen = RefCell::new(Seen::default());
                    Probe::new(&seen).label("Between").sense(Sense::HOVER).id_salt("between").show(ui);
                    let second = Radio::new(2u8).label("Range").id_salt("range").show(ui);
                    rects.borrow_mut().extend([(first.rect, first.id), (second.rect, second.id)]);
                });
            });
        });
        group = Some(out.output.response.id);
    };
    frames.idle(3, |ui| show(ui, &mut value));
    let second = rects.borrow()[1].0;
    click(&mut frames, second.center(), &mut |ui| show(ui, &mut value));
    assert_eq!(value, 2, "a radio inside a nested container selects through the group's arena");
    let tree = frames.tree();
    let group = group.expect("the group showed");
    assert_eq!(tree.role(group), Some(Role::RADIO_GROUP));
    assert_eq!(tree.name(group), Some("Pages"));
    let ids: Vec<Id> = rects.borrow().iter().map(|(_, id)| *id).collect();
    for (at, id) in ids.iter().enumerate() {
        assert_eq!(tree.role(*id), Some(Role::RADIO_BUTTON));
        let mut up = craft_ui::testing::access::NodeKey::of(*id);
        let mut inside = false;
        while let Some(parent) = tree.parent_of_key(up) {
            inside |= parent == craft_ui::testing::access::NodeKey::of(group);
            up = parent;
        }
        assert!(inside, "members are descendants of the group node");
        assert_eq!(tree.position(*id), Some((at, 2)), "each member reports its place in the set");
    }
}

fn rating_rig(start: u8) -> (Fixture, Rc<Cell<u8>>) {
    let value = Rc::new(Cell::new(start));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut stars = shown.get();
        Rating::new(Bind::bound(&mut stars), 5).label("Rating").show(ui);
        shown.set(stars);
    });
    fixture.settle();
    (fixture, value)
}

fn press(fixture: &mut Fixture, key: Key) {
    fixture.key(key, Modifiers::NONE);
    fixture.idle(2);
}

/// A click on a star sets the rating to it; a click on the current star clears it.
#[trial]
fn a_click_on_the_current_star_clears_the_rating() {
    let (mut fixture, value) = rating_rig(0);
    let star = fixture.by_label("3 stars").expect("the third star is a node");
    fixture.click(&star);
    assert_eq!(value.get(), 3);
    fixture.click(&star);
    assert_eq!(value.get(), 0);
}

/// Tab arrives at the current star; the arrows and Home and End move focus between the stars and leave the rating;
/// Space sets the focused star's rating and a digit sets that many stars.
#[trial]
fn rating_arrows_move_focus_and_space_and_digits_set() {
    let (mut fixture, value) = rating_rig(1);
    press(&mut fixture, Key::Tab);
    press(&mut fixture, Key::ArrowRight);
    press(&mut fixture, Key::ArrowRight);
    assert_eq!(value.get(), 1, "the arrows move focus, not the rating");
    press(&mut fixture, Key::Space);
    assert_eq!(value.get(), 3, "Space sets the focused star");
    press(&mut fixture, Key::Num5);
    assert_eq!(value.get(), 5);
    press(&mut fixture, Key::Home);
    press(&mut fixture, Key::Space);
    assert_eq!(value.get(), 1, "Home focuses the first star");
    press(&mut fixture, Key::End);
    press(&mut fixture, Key::Space);
    assert_eq!(value.get(), 5, "End focuses the last star");
}

/// Tab arrives at the selected segment, the arrows move focus past a disabled segment, Space picks the focused one,
/// and a click on a disabled segment picks nothing.
#[trial]
fn segmented_arrows_move_focus_past_disabled_segments_and_space_picks() {
    let value = Rc::new(Cell::new(3u8));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let segments = [Entry::new(1u8, "Draft"), Entry::new(2, "Review").enabled(false), Entry::new(3, "Final")];
        let mut pick = shown.get();
        Segmented::new("Stage", Bind::bound(&mut pick), &segments).show(ui);
        shown.set(pick);
    });
    fixture.settle();
    let review = fixture.by_label("Review").expect("every segment is a node");
    fixture.click(&review);
    assert_eq!(value.get(), 3, "a disabled segment is not picked");
    press(&mut fixture, Key::Tab);
    press(&mut fixture, Key::ArrowLeft);
    assert_eq!(value.get(), 3, "the arrows move focus, not the pick");
    press(&mut fixture, Key::Space);
    assert_eq!(value.get(), 1, "Tab arrived at the selected segment; one ArrowLeft passed the disabled one to Draft");
}

/// Rows labelled through the body's label step carry an icon and a count, and a click on a row's radio still
/// selects through the group's registrations.
#[trial]
fn radio_rows_labelled_through_the_body_select_on_click() {
    use craft_behavior::IconId;
    use craft_ui::contract::SideLabel;
    let mut frames = Frames::new();
    let mut value = 1u8;
    let radios = RefCell::new(Vec::<(Rect, Id)>::new());
    let show = |ui: &mut Ui, value: &mut u8| {
        RadioGroup::new("Show", Bind::bound(value)).id_salt("group").show(ui, |ui, cx| {
            // The body runs in a measuring run and then the real run; the last run's radios are the shown ones.
            radios.borrow_mut().clear();
            for (v, text, count) in [(1u8, "Layers", "12"), (2, "Pages", "4")] {
                let side = SideLabel::new().icon(IconId::new("list")).detail(count);
                let response = cx.labelled(ui, v, text.into(), side, Radio::new(v));
                radios.borrow_mut().push((response.rect, response.id));
            }
        });
    };
    frames.idle(3, |ui| show(ui, &mut value));
    let rects: Vec<Rect> = radios.borrow().iter().map(|(rect, _)| *rect).collect();
    assert_eq!(rects.len(), 2);
    assert!(rects[0].max.y <= rects[1].min.y, "the rows stack in the body's flow: {rects:?}");
    click(&mut frames, rects[1].center(), &mut |ui| show(ui, &mut value));
    assert_eq!(value, 2, "a click on the second row's radio selects it");
}

/// The colour of the first outlined circle a frame painted: a radio's ring.
fn ring_colour(show: impl FnMut(&mut Ui)) -> Color32 {
    let mut show = show;
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    let shapes = frames.frame_shapes(&mut show);
    shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Circle(circle) if circle.stroke.width > 0.0 => Some(circle.stroke.color),
            _ => None,
        })
        .expect("a radio paints its ring")
}

fn refuse_one(value: &u8) -> Validity<u8> {
    if *value == 1 { Validity::Refuse(Message::new("No")) } else { Validity::Ok }
}

/// A disabled radio's ring takes the disabled colour, and the ring of a radio in a group whose value is refused takes
/// the same bad tone a refused check box's edge does.
#[trial]
fn a_radio_ring_follows_disabled_and_invalid() {
    let ring = |enabled: bool, invalid: bool| {
        let mut value = if invalid { 1u8 } else { 2u8 };
        ring_colour(move |ui| {
            let bind = if invalid { Bind::bound(&mut value).check(refuse_one) } else { Bind::bound(&mut value) };
            RadioGroup::new("Size", bind).show(ui, |ui, _| {
                Radio::new(3u8).label("Large").enabled(enabled).show(ui);
            });
        })
    };
    let rest = ring(true, false);
    assert_ne!(ring(false, false), rest, "a disabled radio's ring differs from an enabled one's");
    let invalid = ring(true, true);
    assert_ne!(invalid, rest, "an invalid group's ring differs from a valid one's");
    let mut refused = true;
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        Checkbox::new(Bind::bound(&mut refused).check(|on: &bool| if *on { Validity::Refuse(Message::new("No")) } else { Validity::Ok }))
            .label("Lock")
            .show(ui);
    };
    frames.idle(3, &mut show);
    let bad = frames
        .frame_shapes(&mut show)
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.stroke.width > 0.0 => Some(rect.stroke.color),
            _ => None,
        })
        .expect("a check box paints its edge");
    assert_eq!(invalid, bad, "the ring takes the bad tone");
}

const DRAFT: Word = Word::keyed("trial.draft", "Draft");
const DRAFT_DE: &str = "Entwurf zur Ueberpruefung";

/// The node of the first segment, shown in English or under a language whose word for it is longer.
fn draft_segment(translated: bool) -> Option<Target> {
    let mut fixture = Fixture::new(move |ui| {
        let entries = [Entry::new(1u8, DRAFT), Entry::new(2, "Final")];
        let mut pick = 1u8;
        let mut show = |ui: &mut Ui| {
            Segmented::new("Stage", Bind::bound(&mut pick), &entries).show(ui);
        };
        if translated {
            let rows = WordRows::new().row(DRAFT, DRAFT_DE);
            with_env(ui, |overrides| overrides.words = Some(WordsOverride::Overlay(rows)), show);
        } else {
            show(ui);
        }
    });
    fixture.settle();
    fixture.by_label(if translated { DRAFT_DE } else { "Draft" })
}

/// A segment is measured from its translated text and its node is named in that language.
#[trial]
fn a_segment_is_measured_and_named_from_its_translated_label() {
    let english = draft_segment(false).expect("the English segment is a node");
    let german = draft_segment(true).expect("the segment's node is named in the scope's language");
    assert!(german.rect.width() > english.rect.width(), "the longer word makes a wider segment: {english:?} {german:?}");
}

/// A disabled icon-only segment still shows its label as a tooltip.
#[trial]
fn a_disabled_icon_segment_keeps_its_tooltip() {
    use craft_ui::contract::tooltip::DELAY;
    let mut fixture = Fixture::new(|ui| {
        let views = [Entry::new(1u8, "List").icon(IconId::new("list")), Entry::new(2, "Detail").icon(IconId::new("grid")).enabled(false)];
        let mut pick = 1u8;
        Segmented::new("View", Bind::bound(&mut pick), &views).show(ui);
    });
    fixture.settle();
    let detail = fixture.by_label("Detail").expect("a disabled segment is a node");
    fixture.frame(vec![moved(detail.center())]);
    fixture.advance(DELAY);
    fixture.idle(3);
    assert!(any_open(fixture.ctx(), OpenFilter::Any), "the disabled segment's tooltip shows");
}

const TALL: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(210.0, 130.0) };

/// In a rating placed taller than its control, the stars are painted in their parts: a click on the third star's
/// painted area sets 3.
#[trial]
fn a_click_on_a_painted_star_in_a_tall_rating_sets_it() {
    let mut frames = Frames::new();
    let mut stars = 0u8;
    let mut show = |ui: &mut Ui| {
        Rating::new(Bind::bound(&mut stars), 5).label("Rating").show_at(ui, TALL);
    };
    frames.idle(3, &mut show);
    let mut centres = star_centres(&frames.frame_shapes(&mut show), TALL);
    assert_eq!(centres.len(), 5, "five stars are painted: {centres:?}");
    centres.sort_by(|a, b| a.x.total_cmp(&b.x));
    click(&mut frames, centres[2], &mut show);
    assert_eq!(stars, 3, "the click on the third star's painted area picks it");
}

/// The middle of each painted shape inside `within`, one per distinct column.
fn star_centres(shapes: &[egui::Shape], within: Rect) -> Vec<Pos2> {
    let mut centres: Vec<Pos2> = shapes
        .iter()
        .map(egui::Shape::visual_bounding_rect)
        .filter(|rect| rect.is_positive() && within.contains_rect(*rect))
        .map(|rect| rect.center())
        .collect();
    centres.sort_by(|a, b| a.x.total_cmp(&b.x));
    centres.dedup_by(|a, b| (a.x - b.x).abs() < 1.0);
    centres
}

/// A rating takes the digits up to its own maximum: with three stars, 3 is claimed and 4 reaches egui.
#[trial]
fn a_rating_claims_only_the_digits_up_to_its_maximum() {
    let stars = Rc::new(Cell::new(1u8));
    let reached = Rc::new(Cell::new([false; 2]));
    let (shown, seen) = (stars.clone(), reached.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut value = shown.get();
        Rating::new(Bind::bound(&mut value), 3).label("Rating").show(ui);
        shown.set(value);
        let mut now = seen.get();
        now[0] |= craft_ui::testing::input::key_reached(ui.ctx(), Key::Num3);
        now[1] |= craft_ui::testing::input::key_reached(ui.ctx(), Key::Num4);
        seen.set(now);
    });
    fixture.settle();
    press(&mut fixture, Key::Tab);
    press(&mut fixture, Key::Num3);
    press(&mut fixture, Key::Num4);
    assert_eq!(stars.get(), 3, "3 sets three stars");
    assert_eq!(reached.get(), [false, true], "3 is claimed and 4 is not");
}

const THUMB: PartKey = PartKey::new("thumb");

/// A composite that shows a rating as the face of one part, as a browse thumbnail shows its rating.
#[derive(Component)]
struct Thumbnail {
    #[common]
    common: Common,
}

impl Component for Thumbnail {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &craft_ui::style::StyleCx<'_>, _: ThumbnailStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(200.0, 30.0)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(THUMB, rect).sense(Sense::HOVER);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
        ui.face(THUMB, Rating::new(Bind::given(2u8), 5));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A rating shown as the face of a part paints its stars inside the part.
#[trial]
fn a_rating_shown_as_a_face_paints_its_stars_in_the_part() {
    let mut frames = Frames::new();
    let mut host = Rect::NOTHING;
    let mut shapes = Vec::new();
    for _ in 0..3 {
        shapes = frames.frame_shapes(|ui| host = Thumbnail { common: Common::new() }.show(ui).rect);
    }
    assert_eq!(star_centres(&shapes, host).len(), 5, "five stars are painted inside the part: {shapes:?}");
}
