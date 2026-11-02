//! Tests of the brush card parts and layout (task BR-BP1).

use craft_theme::Mode;
use craft_ui::theme::{Base, install, tokens_in_mode};
use egui::{Rect, pos2, vec2};

use super::*;

fn style() -> BrushCardStyle {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    BrushPresetCards::<u32>::style(&tokens_in_mode(&ctx, Mode::Dark))
}

fn parts(name: bool, stroke: bool, tip: bool) -> BrushCardParts {
    BrushCardParts { name, stroke, tip }
}

fn layout(p: BrushCardParts, scale: f32, number: f32) -> (Rect, BrushCardLayout) {
    let style = style();
    let size = BrushCardLayout::cell_size(p, scale, 400.0, &style);
    let cell = Rect::from_min_size(pos2(10.0, 20.0), size);
    (cell, BrushCardLayout::of(cell, p, scale, Some(30.0), number, &style))
}

fn rects(l: &BrushCardLayout) -> Vec<Rect> {
    [l.tip_cell, l.number, l.stroke, l.name].into_iter().flatten().collect()
}

// All seven part combinations: every part rect lies inside the card, none overlap, and the name
// spans the bottom row unless only the tip and the name show (then it sits in the stroke's cell).
#[test]
fn card_layout_places_tip_stroke_and_name() {
    for bits in 1u8..8 {
        let p = parts(bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
        let (cell, l) = layout(p, 1.0, 13.0);
        assert!(cell.contains_rect(l.card), "{p:?}: the card is inside its cell");
        assert_eq!((l.tip_cell.is_some(), l.stroke.is_some(), l.name.is_some()), (p.tip, p.stroke, p.name), "{p:?}");
        // The tip thumbnail and the number sit in the tip cell and are checked against it.
        let parts_only: Vec<Rect> = [l.tip_cell, l.stroke, l.name].into_iter().flatten().collect();
        for (i, a) in parts_only.iter().enumerate() {
            assert!(l.card.expand(0.01).contains_rect(*a), "{p:?}: {a:?} inside {:?}", l.card);
            for b in &parts_only[i + 1..] {
                assert!(!a.shrink(0.01).intersects(*b), "{p:?}: {a:?} overlaps {b:?}");
            }
        }
        if let (Some(tip_cell), Some(tip)) = (l.tip_cell, l.tip) {
            assert!(tip_cell.expand(0.01).contains_rect(tip), "{p:?}: the tip is inside its cell");
        }
        if let (Some(tip), Some(number)) = (l.tip, l.number) {
            assert!(tip.bottom() <= number.top() + 0.01, "{p:?}: the tip stays above the number");
        }
        if let (Some(name), Some(stroke)) = (l.name, l.stroke) {
            assert!(name.top() >= stroke.bottom() - 0.01, "{p:?}: the name is below the stroke");
        }
        if p.tip_and_name() {
            let tip_cell = l.tip_cell.unwrap_or(Rect::NOTHING);
            let name = l.name.unwrap_or(Rect::NOTHING);
            assert!(name.left() >= tip_cell.right(), "the name sits in the stroke's cell");
        }
        assert!(rects(&l).iter().all(|r| r.is_finite()));
    }
}

// At scale 1 a tip-only cell is 60 × 70 and a tip-and-name cell 150 × 70; a full card is 300 wide.
#[test]
fn tip_only_and_tip_and_name_cards_use_their_fixed_cells() {
    let style = style();
    assert_eq!(BrushCardLayout::cell_size(parts(false, false, true), 1.0, 500.0, &style), vec2(60.0, 70.0));
    assert_eq!(BrushCardLayout::cell_size(parts(true, false, true), 1.0, 500.0, &style), vec2(150.0, 70.0));
    let full = BrushCardLayout::cell_size(BrushCardParts::ALL, 1.0, 500.0, &style);
    assert_eq!((full.x, full.y), (300.0, 70.0));
    assert_eq!(BrushCardLayout::cell_size(BrushCardParts::ALL, 1.0, 200.0, &style).x, 200.0, "a narrow room caps the width");
}

// Heights and padding stop shrinking at scale 0.6 while widths keep shrinking; at 0.7 and below
// the number band is gone and the tip takes its cell.
#[test]
fn heights_floor_at_0_6_and_the_number_drops_at_0_7() {
    let style = style();
    let at = |s| BrushCardLayout::cell_size(BrushCardParts::ALL, s, 500.0, &style);
    assert_eq!(at(0.6).y, at(0.3).y, "the height stops at 0.6");
    assert!(at(0.3).x < at(0.6).x, "the width keeps shrinking");
    assert!(at(0.6).y < at(1.0).y);
    let (_, compact) = layout(BrushCardParts::ALL, 0.7, 13.0);
    assert!(compact.number.is_none() && compact.tip.is_some());
    let (_, roomy) = layout(BrushCardParts::ALL, 0.8, 13.0);
    assert!(roomy.number.is_some());
    let (Some(cell), Some(tip)) = (compact.tip_cell, compact.tip) else { panic!("tip shows") };
    assert!((tip.width() - (cell.width().min(cell.height()) - 2.0)).abs() < 0.01, "the compact tip fills its cell less 2");
}

// `set` refuses to clear the last part; `normalised` turns all three on when none is.
#[test]
fn at_least_one_part_stays_on() {
    let only_tip = parts(false, false, true);
    assert_eq!(only_tip.set(BrushCardPart::Tip, false), only_tip);
    assert_eq!(only_tip.set(BrushCardPart::Name, true), parts(true, false, true));
    assert_eq!(BrushCardParts::ALL.set(BrushCardPart::Stroke, false), parts(true, false, true));
    assert_eq!(parts(false, false, false).normalised(), BrushCardParts::ALL);
    assert_eq!(only_tip.normalised(), only_tip);
}

// `tip_thumb_scale`: 0.3 under 12 px, 1 from 40 px, size / 40 between; NaN gives 1.
#[test]
fn tip_thumbnails_scale_with_the_preset_size() {
    assert_eq!(tip_thumb_scale(5.0), 0.3);
    assert_eq!(tip_thumb_scale(40.0), 1.0);
    assert_eq!(tip_thumb_scale(300.0), 1.0);
    assert!((tip_thumb_scale(20.0) - 0.5).abs() < 1e-6);
    assert_eq!(tip_thumb_scale(f64::NAN), 1.0);
}

// A card row's accessible name is "{name}, {size} px" with the size rounded; its label stays the name.
#[test]
fn card_rows_are_named_with_their_size() {
    let labels = BrushCardLabels::default();
    let row = BrushPresetCards::<u32>::row(7, "Soft Round", 1, 29.6, &labels, &craft_ui::localized::LocaleNumbers::from_tag("en"));
    assert_eq!(row.label, "Soft Round");
    assert_eq!(row.accessible_name.as_deref(), Some("Soft Round, 30 px"));
    assert_eq!(row.depth, 1);
}

// The tip is at the leading end of a card and the stroke after it, in both directions.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn cards_mirror_in_right_to_left() {
    use craft_ui::direction::{self, Direction};
    use craft_ui::localized::LocaleNumbers;
    use egui_kittest::Harness;

    let centres = |dir: Direction| {
        let seen = std::rc::Rc::new(std::cell::Cell::new((None, None)));
        let sink = seen.clone();
        let (tip_sink, stroke_sink) = (seen.clone(), seen.clone());
        let mut state = BrushCardsState::<u32>::new();
        let rows = vec![BrushPresetCards::row(1, "Soft", 0, 30.0, &BrushCardLabels::default(), &LocaleNumbers::from_tag("en"))];
        let mut harness = Harness::builder()
            .with_size([400.0, 200.0])
            .with_context_setup(move |ctx: &egui::Context| {
                install(ctx, &Base::suite_default());
                direction::apply(ctx, dir);
            })
            .build_ui(move |ui| {
                let _ = &sink;
                let _ = BrushPresetCards::new("cards", &mut state, &rows)
                    .diameter_of(|_| Some(30.0))
                    .tip(|_, rect, _| tip_sink.set((Some(rect.center().x), stroke_sink.get().1)))
                    .stroke(|_, rect, _| stroke_sink.set((tip_sink.get().0, Some(rect.center().x))))
                    .show(ui);
            });
        harness.run();
        let (tip, stroke) = seen.get();
        (tip.expect("the tip was painted"), stroke.expect("the stroke was painted"))
    };
    let (tip, stroke) = centres(Direction::Ltr);
    assert!(tip < stroke, "left to right: the tip leads {tip} {stroke}");
    let (tip, stroke) = centres(Direction::Rtl);
    assert!(tip > stroke, "right to left: the tip leads {tip} {stroke}");
}

// The accessible name writes the size in the given number format.
#[test]
fn the_row_names_the_size_in_the_given_numbers() {
    use craft_ui::localized::LocaleNumbers;
    let labels = BrushCardLabels::default();
    let row = BrushPresetCards::row(1u32, "Soft", 0, 30.4, &labels, &craft_ui::localized::LocaleNumbers::from_tag("en"));
    assert_eq!(row.accessible_name.as_deref(), Some("Soft, 30 px"));
}
