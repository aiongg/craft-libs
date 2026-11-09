//! Page placement, fits and spreads.

use std::ops::Range;

use kurbo::{Point, Size};

use super::*;

/// Spacing for the tests: the numbers the assertions are written against.
const SPACING: PageSpacing = PageSpacing { gap: 14.0, margin: 28.0, side: 10.0, white_space_pad: 6.0, seam: 1.0 };

const LETTER: Size = Size::new(612.0, 792.0);

fn viewport() -> Size {
    Size::new(1000.0, 800.0)
}

#[test]
fn single_pages_centre_and_stack_with_gaps() {
    let narrow = Size::new(400.0, 500.0);
    let laid = place_pages(&[LETTER, narrow, LETTER], 1.0, viewport(), &LayoutOptions::ACROBAT, &SPACING);
    assert_eq!(laid.rects[0].y0, SPACING.margin, "the first page starts below the margin");
    assert!(laid.rects.windows(2).all(|pair| pair[1].y0 - pair[0].y1 == SPACING.gap), "pages stack with equal gaps");
    assert!(laid.rects.iter().all(|r| (r.center().x - viewport().width / 2.0).abs() < 1e-9), "every page, narrower ones too, is centred");

    let joined = LayoutOptions { page_gaps: false, ..LayoutOptions::ACROBAT };
    let laid = place_pages(&[Size::new(612.4, 792.4); 2], 1.0, viewport(), &joined, &SPACING);
    assert_eq!(laid.rects[0].height(), 792.0);
    assert_eq!(laid.rects[0].y1, laid.rects[1].y0);
}

#[test]
fn spreads_follow_the_cover_rule() {
    let options = LayoutOptions { display: PageDisplay::TwoUp, cover: true, ..LayoutOptions::ACROBAT };
    let laid = place_pages(&[LETTER; 5], 0.5, viewport(), &options, &SPACING);
    assert!(laid.rects[0].x0 >= 500.0);
    assert_eq!(laid.rects[1].y0, laid.rects[2].y0);
    assert!(laid.rects[3].y0 > laid.rects[1].y0);

    let rows = |cover| -> Vec<Range<usize>> {
        let options = LayoutOptions { cover, ..options };
        (0..5).map(|p| shown_row(5, p, &options)).collect()
    };
    assert_eq!(rows(true), [0..1, 1..3, 1..3, 3..5, 3..5]);
    assert_eq!(rows(false), [0..2, 0..2, 2..4, 2..4, 4..5]);
    for cover in [true, false] {
        for (page, row) in rows(cover).iter().enumerate() {
            assert_eq!(row.start, spread_first(page, cover));
        }
    }
    assert_eq!(shown_row(0, 0, &options), 0..0);
}

#[test]
fn column_count_and_fits_follow_the_width_and_largest_page() {
    let wide = Size::new(2000.0, 800.0);
    let laid = place_pages(&[LETTER; 10], 0.75, wide, &LayoutOptions::WORD, &SPACING);
    assert_eq!(laid.columns, 4);
    assert_eq!(laid.rects[3].y0, laid.rects[0].y0);
    assert!(laid.rects[4].y0 > laid.rects[0].y0);

    assert_eq!(place_pages(&[LETTER; 2], 0.75, wide, &LayoutOptions::WORD, &SPACING).columns, 2);
    let at_least = LayoutOptions { min_columns: 3, ..LayoutOptions::WORD };
    assert_eq!(place_pages(&[LETTER; 10], 2.0, wide, &at_least, &SPACING).columns, 3);

    let sizes = [LETTER, LETTER, Size::new(1224.0, 792.0)];
    let options = LayoutOptions::ACROBAT;
    let widest = fit_scale(&sizes, viewport(), Fit::Width, &options, &SPACING, Some(0)).unwrap();
    assert_eq!(widest, 980.0 / 1224.0);
    let single = LayoutOptions { continuous: false, ..options };
    assert_eq!(fit_scale(&sizes, viewport(), Fit::Width, &single, &SPACING, Some(0)).unwrap(), 980.0 / 612.0);
    assert_eq!(fit_scale(&sizes, viewport(), Fit::None, &options, &SPACING, Some(0)), None);

    let huge = [Size::new(100_000.0, 100_000.0)];
    let scale = fit_scale(&huge, viewport(), Fit::Width, &options, &SPACING, None).unwrap();
    assert_eq!(scale, ZOOM_RANGE.start() * SCREEN_POINTS_PER_POINT);
}

#[test]
fn hide_white_space_cuts_margins_keeps_coordinates_and_scales_pad() {
    let mut laid = place_pages(&[LETTER; 3], 1.0, viewport(), &LayoutOptions::ACROBAT, &SPACING);
    let before = laid.rects[1].size();
    laid.hide_white_space(&[(72.0, 720.0); 3], 1.0, 800.0, &SPACING);
    assert_eq!(laid.bands, [(0.0, 726.0), (66.0, 726.0), (66.0, 792.0)]);
    assert_eq!(laid.rects[1].size(), before);
    assert_eq!(laid.rects[1].y0 + 66.0, laid.rects[0].y0 + 726.0 + SPACING.seam);

    let centre = 400.0;
    assert_eq!(page_at(&laid, Point::new(centre, 1000.0)), Some(1));
    assert_eq!(page_at(&laid, Point::new(centre, 754.5)), None);
    assert!(laid.on_page_gap(Point::new(centre, 754.5)));
    assert!(!laid.on_page_gap(Point::new(centre, 1000.0)));

    // The pad is in page points: zoomed to 2, 6 page points of pad are 12 screen points; the seam stays 1.
    let mut laid = place_pages(&[LETTER; 3], 2.0, viewport(), &LayoutOptions::ACROBAT, &SPACING);
    laid.hide_white_space(&[(72.0, 720.0); 3], 2.0, 800.0, &SPACING);
    assert_eq!(laid.bands[1], (72.0 * 2.0 - 6.0 * 2.0, 720.0 * 2.0 + 6.0 * 2.0));
    assert_eq!(laid.rects[1].y0 + laid.bands[1].0, laid.rects[0].y0 + laid.bands[0].1 + SPACING.seam);
}

// Every zoom the view offers lays out as shown: 6400 % is not cut to a smaller scale.
#[test]
fn the_largest_zoom_lays_out_at_its_own_scale() {
    let scale = ZOOM_RANGE.end() * SCREEN_POINTS_PER_POINT;
    let laid = place_pages(&[LETTER], scale, viewport(), &LayoutOptions::ACROBAT, &SPACING);
    assert_eq!(laid.rects[0].width(), LETTER.width * scale);
}

#[test]
fn tokens_and_zoom_names_round_trip_and_refuse_bad_input() {
    for display in PageDisplay::ORDER {
        for continuous in [false, true] {
            // Columns always scroll, so both flags name `columns` and read back as scrolling.
            let expected = if display == PageDisplay::Columns { (display, true) } else { (display, continuous) };
            assert_eq!(parse_display_token(display_token(display, continuous)), Some(expected));
        }
    }
    assert_eq!(parse_display_token("Two-Up-Single"), Some((PageDisplay::TwoUp, false)));
    assert_eq!(parse_display_token("twoup"), None);

    let defaults = ViewDefaults::default();
    for name in ["fit-width", "fit-page", "125%"] {
        assert_eq!(defaults.with_zoom(name).unwrap().zoom_name(), name);
    }
    assert_eq!(defaults.with_zoom("FIT-PAGE").unwrap().fit, Fit::Page);
    assert_eq!(defaults.with_zoom("125%").unwrap().zoom, 1.25);
    assert_eq!(defaults.with_zoom("6500%"), None);
    assert_eq!(defaults.with_zoom("fit"), None);
}
