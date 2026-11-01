//! `ComparePair` and `survey_columns` (task B3).

use craft_theme::Mode;
use craft_ui::theme::tokens_in_mode;
use egui::{Rect, pos2, vec2};

use super::CompareStyle;
use super::arrange::{ComparePair, survey_columns};

// 8 tiles in 1600 × 600 give 4 columns; 12 in 1200 × 800 tie at 3 and 4 columns and take the
// first; 1 tile gives 1.
#[test]
fn survey_columns_maximise_the_tile() {
    let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0));
    assert_eq!(survey_columns(12, area), 3);
    assert_eq!(survey_columns(8, Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 600.0))), 4);
    assert_eq!(survey_columns(1, area), 1);
}

// Panes are equal and inset; in RTL the Select pane is the right one.
#[test]
fn compare_panes_are_equal_and_mirror() {
    let style = CompareStyle::of(&tokens_in_mode(&egui::Context::default(), Mode::Light));
    let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 600.0));
    let [select, candidate] = ComparePair::new().rects(area, &style);
    let expected = (1000.0 - 2.0 * style.margin - style.gap) / 2.0;
    assert_eq!(select.width(), expected);
    assert_eq!(candidate.width(), expected);
    assert_eq!(select.left(), style.margin);
    assert_eq!(candidate.right(), 1000.0 - style.margin);
    assert_eq!(candidate.left() - select.right(), style.gap);
    let divider = ComparePair::new().divider(area, &style);
    assert!(divider.left() > select.right() && divider.right() < candidate.left());
    let [select_rtl, candidate_rtl] = ComparePair::new().rtl(true).rects(area, &style);
    assert_eq!((select_rtl, candidate_rtl), (candidate, select));
}
