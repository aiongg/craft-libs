//! The pure painters: focus rings, scroll bars, handles and the face id.

use craft_behavior::PartKey;
use craft_theme::{Contrast, ControlSize, Flags, Mode, State, StateMix, Variant};
use craft_ui::primitives::{BarAxis, FocusKind, FocusRing, Handle, HandleShape, PaintShape, ScrollBar};
use craft_ui::style::{FocusLook, StateSet};
use craft_ui::testing::style::{face_id, tokens, with_cx};
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Shape, Stroke, pos2, vec2};

const EPS: f32 = 1e-3;
const END: f32 = 800.0;
const MIDDLE: f32 = 400.0;

#[trial]
fn a_ring_shows_only_on_the_focused_component_with_focus_visible() {
    assert!(FocusRing::visible(Flags::FOCUSED | Flags::FOCUS_VISIBLE));
    assert!(!FocusRing::visible(Flags::FOCUSED), "focus by mouse shows no ring");
    assert!(!FocusRing::visible(Flags::FOCUS_VISIBLE | Flags::SELECTED));
}

#[trial]
fn a_halo_adds_a_wider_translucent_stroke_under_the_ring() {
    let look = FocusLook::new(Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 255)), 1.0);
    let shape = PaintShape::Circle(pos2(20.0, 20.0), 6.0);
    let ring = FocusRing::shapes(&look, FocusKind::Ring, &shape, 1.0);
    let halo = FocusRing::shapes(&look, FocusKind::Halo, &shape, 1.0);
    assert_eq!(ring.len(), 1);
    assert_eq!(halo.len(), 2);
    let width = |s: &Shape| match s {
        Shape::Circle(c) => c.stroke.width,
        _ => 0.0,
    };
    assert!(width(&halo[0]) > width(&halo[1]), "the halo lies under a narrower ring");
    assert_eq!(width(&halo[1]), width(&ring[0]), "the ring inside a halo is the plain ring");
}

#[trial]
fn the_scroll_thumb_tracks_the_offset_and_never_leaves_the_track() {
    let frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let look = with_cx(&frames.ctx, &tokens, Variant::SECONDARY, ControlSize::Md, |cx| cx.scroll_bar());
    let track = Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 200.0));
    let at = |offset| ScrollBar::thumb(&look, BarAxis::Vertical, track, 1000.0, 200.0, offset);
    assert_eq!(at(0.0).min.y, track.min.y);
    assert!((at(END).max.y - track.max.y).abs() < EPS, "the end of the content puts the thumb at the track's end");
    assert!(at(MIDDLE).min.y > at(0.0).min.y && at(MIDDLE).max.y < at(END).max.y + EPS);
    let tiny = ScrollBar::thumb(&look, BarAxis::Vertical, track, 1_000_000.0, 200.0, 0.0);
    assert!(tiny.height() >= look.min_thumb.min(track.height()), "the thumb keeps a grabbable length");
    let shapes = ScrollBar::shapes(&look, track, at(0.0), &StateSet::default());
    assert_eq!(shapes.len(), 2, "track, then thumb");
}

#[trial]
fn a_hovered_handle_fills_differently_from_a_resting_one() {
    let frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let look = with_cx(&frames.ctx, &tokens, Variant::SECONDARY, ControlSize::Md, |cx| cx.handle());
    let fill = |state: &StateSet| match &Handle::shapes(&look, HandleShape::Circle, pos2(5.0, 5.0), state)[0] {
        Shape::Circle(c) => c.fill,
        _ => egui::Color32::TRANSPARENT,
    };
    let rest = StateSet::default();
    let hover = StateSet::new(State::Hover, Flags::empty(), StateMix::new(1.0, 0.0, 0.0));
    assert_ne!(fill(&rest), fill(&hover));
}

#[trial]
fn a_face_has_its_own_id_per_part_owned_by_the_composite() {
    const CHECK: PartKey = PartKey::new("check");
    const CLOSE: PartKey = PartKey::new("close");
    let chip = "chip";
    assert_eq!(face_id(chip, CLOSE.into()), face_id(chip, CLOSE.into()));
    assert_ne!(face_id(chip, CLOSE.into()), face_id(chip, CHECK.into()), "parts hover and press apart");
    assert_ne!(face_id(chip, CLOSE.at(1)), face_id(chip, CLOSE.at(2)), "repeated parts too");
    assert_ne!(face_id(chip, CLOSE.into()), face_id("other", CLOSE.into()), "two composites do not share faces");
}
