//! The stroke editor, its button and the line sample: the rows' rules, what a pick reports and the sample's geometry.

use craft_behavior::Mixed;
use craft_draw::ui::stroke::{
    ChoicePainter, LineCap, LineJoin, LineStyle, StrokeButton, StrokeChoice, StrokeChoiceRow, StrokeEditor, StrokeMixed, StrokePreview,
    StrokeSections, StrokeStyle,
};
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::primitives::PaintShape;
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Color32, Key, Modifiers, Pos2, Rect, Shape};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A sample rect for the line geometry.
const SAMPLE: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(200.0, 40.0));

/// The dash offset a trial types into its field.
const TYPED_OFFSET: f32 = 3.0;
/// One step of an 8-bit alpha, the rounding of a faded fill.
const ALPHA_STEP: f32 = 1.0;

fn dashed() -> StrokeStyle {
    StrokeStyle::default().with_line_style(LineStyle::Dashed)
}

type Configure = for<'a> fn(StrokeEditor<'a>) -> StrokeEditor<'a>;

/// An editor over a shared style, set up by `configure`.
fn editor(style: StrokeStyle, configure: Configure) -> (Fixture, Rc<RefCell<StrokeStyle>>) {
    let value = Rc::new(RefCell::new(style));
    let held = Rc::clone(&value);
    let mut fixture = Fixture::new(move |ui| {
        let mut style = held.borrow_mut();
        configure(StrokeEditor::new("Stroke", Bind::bound(&mut *style))).show(ui);
    });
    fixture.settle();
    (fixture, value)
}

fn plain(editor: StrokeEditor<'_>) -> StrokeEditor<'_> {
    editor
}

/// Whether the control named `name` is disabled in the access tree.
fn disabled(fixture: &Fixture, name: &str) -> bool {
    let target = fixture.by_label(name).unwrap_or_else(|| panic!("`{name}` is shown"));
    fixture.tree().is_disabled_key(target.node)
}

/// Picking a preset row applies that preset's pattern, caps and corners to the style.
#[trial]
fn stroke_editor_preset_rows_report_the_preset_style() {
    let (mut fixture, value) = editor(StrokeStyle::default(), plain);
    let dotted = fixture.by_label("Dotted").expect("the Dotted preset is shown");
    fixture.click(&dotted);
    fixture.idle(3);
    assert_eq!(value.borrow().line_style(), LineStyle::Dotted);
    assert_eq!(value.borrow().cap, LineCap::Round, "the dotted preset has round caps");
}

/// Alignment needs a closed path; caps need dashes on a closed path.
#[trial]
fn stroke_editor_disables_alignment_on_open_paths_and_caps_on_solid_closed_paths() {
    let (open, _) = editor(StrokeStyle::default(), |editor| editor.closed(false));
    assert!(disabled(&open, "Alignment"), "an open path is always centred");
    assert!(!disabled(&open, "Caps"), "an open path has caps");
    let (solid, _) = editor(StrokeStyle::default(), plain);
    assert!(!disabled(&solid, "Alignment"));
    assert!(disabled(&solid, "Caps"), "a solid closed path has no ends");
    let (dashes, _) = editor(dashed(), plain);
    assert!(!disabled(&dashes, "Caps"), "the dashes have ends");
}

/// The miter limit applies to miter corners only.
#[trial]
fn stroke_editor_miter_limit_only_for_miter_corners() {
    let (miter, _) = editor(StrokeStyle::default(), plain);
    assert!(!disabled(&miter, "Miter limit"));
    let round = StrokeStyle { join: LineJoin::Round, ..StrokeStyle::default() };
    let (round, _) = editor(round, plain);
    assert!(disabled(&round, "Miter limit"));
    let (corners, _) = editor(StrokeStyle::default(), |editor| editor.corners(false));
    assert!(disabled(&corners, "Miter limit"), "a path without corners has no miter");
}

/// Add pair appends `[4, 2]`, Remove pair drops the last two entries, and a typed dash offset is stored.
#[trial]
fn stroke_editor_dash_pairs_add_remove_and_offset() {
    let (mut fixture, value) = editor(dashed(), plain);
    let add = fixture.by_label("Add pair").expect("Add pair is shown");
    fixture.click(&add);
    fixture.idle(3);
    let pair = dashed().dashes;
    assert_eq!(value.borrow().dashes, [pair.clone(), pair.clone()].concat());
    let remove = fixture.by_label("Remove pair").expect("Remove pair is shown");
    fixture.click(&remove);
    fixture.idle(3);
    assert_eq!(value.borrow().dashes, pair);
    let offset = fixture.by_label("Dash offset").expect("the Dash offset field is shown");
    fixture.click(&offset);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(&TYPED_OFFSET.to_string());
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(value.borrow().dash_offset, TYPED_OFFSET);
}

/// A pattern that fails validation is shown with the error, and a click on a control that would keep it invalid reports
/// nothing.
#[trial]
fn stroke_editor_shows_the_error_for_an_invalid_pattern_and_reports_nothing() {
    let invalid = StrokeStyle { dashes: vec![0.0, 0.0], ..StrokeStyle::default() };
    assert!(invalid.validate().is_err());
    let reported = Rc::new(Cell::new(false));
    let seen = Rc::clone(&reported);
    let held = Rc::new(RefCell::new(invalid.clone()));
    let style = Rc::clone(&held);
    let mut fixture = Fixture::new(move |ui| {
        let mut style = style.borrow_mut();
        let out = StrokeEditor::new("Stroke", Bind::bound(&mut *style)).show(ui);
        seen.set(seen.get() || out.event.is_some());
    });
    fixture.settle();
    fixture.idle(3);
    assert!(!reported.get(), "an invalid style the host holds is not an edit");
    assert_eq!(*held.borrow(), invalid);
    assert!(fixture.texts().iter().any(|text| format!("{text:?}").contains("positive pattern total")), "the error is shown");
}

/// A mixed cap lights no segment, and the first pick sets the field.
#[trial]
fn stroke_editor_mixed_values_light_no_button() {
    let mut mixed = StrokeMixed::default();
    mixed.cap = true;
    let value = Rc::new(RefCell::new(dashed()));
    let held = Rc::clone(&value);
    let mut fixture = Fixture::new(move |ui| {
        let mut style = held.borrow_mut();
        StrokeEditor::new("Stroke", Bind::bound(&mut *style)).sections(StrokeSections::PANEL).mixed(&mixed).show(ui);
    });
    fixture.settle();
    let round = fixture.by_label("Round").expect("the cap segments are shown");
    fixture.click(&round);
    fixture.idle(3);
    assert_eq!(value.borrow().cap, LineCap::Round);
}

/// Picking a choice of the app's row reports the row's id and the choice's id.
#[trial]
fn stroke_editor_choice_row_reports_the_pick() {
    let picked = Rc::new(RefCell::new(None));
    let seen = Rc::clone(&picked);
    let mut fixture = Fixture::new(move |ui| {
        let choices = [StrokeChoice::new("open", "Open"), StrokeChoice::new("filled", "Filled")];
        let rows = [StrokeChoiceRow::new("arrowhead", "Arrowhead", &choices).selected(Mixed::Same("open")).paint(ChoicePainter::new(|_, _, _| {}))];
        let out = StrokeEditor::new("Stroke", Bind::given(StrokeStyle::default())).rows(&rows).show(ui);
        if out.choice.is_some() {
            *seen.borrow_mut() = out.choice;
        }
    });
    fixture.settle();
    let select = fixture.by_label("Arrowhead").expect("the app row is shown");
    fixture.click(&select);
    fixture.idle(3);
    let filled = fixture.by_label("Filled").expect("the choices are listed");
    fixture.click(&filled);
    fixture.idle(3);
    assert_eq!(*picked.borrow(), Some(("arrowhead".to_owned(), "filled".to_owned())));
}

/// The popover closes when the target changes.
#[trial]
fn stroke_button_closes_on_target_change() {
    let open = Rc::new(Cell::new(false));
    let target = Rc::new(Cell::new(1_u64));
    let (shared, layer) = (Rc::clone(&open), Rc::clone(&target));
    let mut fixture = Fixture::new(move |ui| {
        let mut style = StrokeStyle::default();
        let mut now = shared.get();
        StrokeButton::new("Stroke Options", Bind::bound(&mut style), Bind::bound(&mut now)).target(layer.get()).show(ui);
        shared.set(now);
    });
    fixture.settle();
    let button = fixture.by_role(Role::BUTTON).first().cloned().expect("the button is shown");
    fixture.click(&button);
    fixture.idle(3);
    assert!(open.get(), "a click opens the popover");
    target.set(2);
    fixture.idle(4);
    assert!(!open.get(), "another target closes it");
}

/// The fields that differ across the styles are marked.
#[trial]
fn stroke_mixed_of_marks_the_fields_that_differ() {
    let first = StrokeStyle::default();
    let other = StrokeStyle { cap: LineCap::Round, opacity: 0.5, ..first.clone() };
    let (style, mixed) = StrokeMixed::of([&first, &other]).expect("two styles");
    assert_eq!(style, first);
    assert!(mixed.cap && mixed.opacity);
    assert!(!mixed.align && !mixed.join && !mixed.dashes && !mixed.miter_limit && !mixed.dash_offset);
    assert!(StrokeMixed::of(std::iter::empty()).is_none());
}

/// A dashed sample has more pieces with round or square caps than with butt caps, and a round corner has more points
/// than a bevel.
#[trial]
fn stroke_preview_dashes_follow_cap_and_join() {
    let count = |cap| {
        let style = StrokeStyle { cap, ..dashed() };
        StrokePreview::new(&style, 4.0).shapes(SAMPLE).len()
    };
    assert!(count(LineCap::Round) > count(LineCap::Butt), "round caps add a fan per dash end");
    assert!(count(LineCap::Square) > count(LineCap::Butt), "square caps add a quad per dash end");
    let corners = |join| {
        let style = StrokeStyle { join, ..StrokeStyle::default() };
        let shapes = StrokePreview::new(&style, 4.0).closed(true).shapes(SAMPLE);
        shapes.iter().map(|shape| if let PaintShape::Polygon(points) = shape { points.len() } else { 0 }).sum::<usize>()
    };
    assert!(corners(LineJoin::Round) > corners(LineJoin::Bevel), "a round corner is a fan, a bevel a triangle");
}

/// An invalid style draws nothing.
#[trial]
fn stroke_preview_skips_invalid_styles() {
    let invalid = StrokeStyle { width: -1.0, ..StrokeStyle::default() };
    assert!(StrokePreview::new(&invalid, 4.0).shapes(SAMPLE).is_empty());
    let half = StrokeStyle { opacity: 0.5, ..StrokeStyle::default() };
    let preview = StrokePreview::new(&half, 4.0);
    assert!(!preview.shapes(SAMPLE).is_empty());
}

/// The alphas of the filled polygons a stroke button showing `style` paints: the sample's dashes, and any glyph of the
/// face drawn as a polygon.
fn sample_alphas(style: &StrokeStyle) -> Vec<u8> {
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        let (mut style, mut open) = (style.clone(), false);
        StrokeButton::new("Stroke Options", Bind::bound(&mut style), Bind::bound(&mut open)).show(ui);
    };
    frames.frame_shapes(&mut show);
    frames
        .frame_shapes(&mut show)
        .iter()
        .filter_map(|shape| match shape {
            Shape::Path(path) if path.closed && path.fill != Color32::TRANSPARENT => Some(path.fill.a()),
            _ => None,
        })
        .collect()
}

/// The sample a stroke button shows is painted at the style's opacity: each dash of a half-opaque style has half the
/// alpha of the same dash at full opacity.
#[trial]
fn the_stroke_sample_is_painted_at_the_styles_opacity() {
    let half = StrokeStyle { opacity: 0.5, ..StrokeStyle::default() };
    let (full, faded) = (sample_alphas(&StrokeStyle::default()), sample_alphas(&half));
    assert_eq!(full.len(), faded.len(), "the same sample at both opacities: {full:?} {faded:?}");
    let dashes: Vec<(u8, u8)> = full.iter().copied().zip(faded.iter().copied()).filter(|(full, faded)| full != faded).collect();
    assert!(!dashes.is_empty(), "the opacity changes the sample: {full:?}");
    assert!(
        dashes.iter().all(|(full, faded)| (f32::from(*full) * half.opacity - f32::from(*faded)).abs() <= ALPHA_STEP),
        "each dash at half the alpha: {dashes:?}"
    );
}
