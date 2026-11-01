//! `arrange`, `TileArrangement`, `fit_name` and `TileZoom` (task B3).

use egui::Rect;

use super::layout::{ArrangeSpacing, TileGroup, TileLayout, arrange, fit_name};
use super::zoom::{TileZoom, ZoomStep};

const SPACING: ArrangeSpacing = ArrangeSpacing { gap: 6.0, inset: 4.0, group_header: 40.0, caption: 20.0 };

fn group(start: usize, count: usize) -> TileGroup {
    TileGroup { start, count, title: String::new(), detail: String::new() }
}

// LightCraft's `ungrouped_justified_rows_fill_the_width` and `groups_start_new_rows_below_their_headers`.
#[test]
fn justified_rows_fill_the_width_except_a_groups_last_row() {
    let aspects = [1.5, 0.75, 1.0, 2.0, 1.5, 1.33, 0.8, 1.0, 1.5, 1.2];
    let groups = [group(0, 6), group(6, 4)];
    let layout = TileLayout::Justified { row_height: 100.0 };
    let a = arrange(&aspects, &groups, 600.0, layout, SPACING);
    assert_eq!(a.headers.len(), 2);
    for (index, row) in a.rows.iter().enumerate() {
        let tiles = &a.tiles[row.tiles.clone()];
        let right = tiles.last().unwrap().right();
        let last_of_group = a.rows.get(index + 1).is_none_or(|next| next.group != row.group);
        if last_of_group {
            assert!((row.height - 100.0).abs() < 1e-3, "a group's last row keeps the row height");
            assert!(right <= 596.0 + 1e-3);
        } else {
            assert!((right - 596.0).abs() < 1e-3, "row {index} ends at {right}");
        }
    }
    for (header, group_index) in a.headers.iter().zip(0..) {
        let first = a.rows.iter().find(|r| r.group == Some(group_index)).unwrap();
        assert!((first.top - header.bottom()).abs() < 1e-3, "rows start below their header");
    }
    assert_eq!(a.row_lengths().iter().sum::<usize>(), aspects.len());
}

// 500 points, inset 4, gap 6, side 120: 3 columns; one group starts a new row.
#[test]
fn cards_columns_follow_the_width_and_the_gap() {
    let aspects = [1.0; 4];
    let layout = TileLayout::Cards { side: 120.0, aspect: 1.0, caption: true };
    let a = arrange(&aspects, &[], 500.0, layout, SPACING);
    assert_eq!(a.row_lengths(), vec![3, 1]);
    assert_eq!(a.tiles[1].left() - a.tiles[0].right(), 6.0);
    assert_eq!(a.rows[0].height, 140.0);
    let grouped = arrange(&aspects, &[group(0, 2), group(2, 2)], 500.0, layout, SPACING);
    assert_eq!(grouped.row_lengths(), vec![2, 2]);
    assert_eq!(grouped.headers.len(), 2);
}

// LightCraft's `rows_index_every_cell_top_to_bottom`: `rows_between` over 10 000 rows.
#[test]
fn rows_index_every_tile_top_to_bottom() {
    let aspects = vec![1.0; 10_000];
    let layout = TileLayout::Cards { side: 100.0, aspect: 1.0, caption: false };
    let a = arrange(&aspects, &[], 110.0, layout, SPACING);
    assert_eq!(a.rows.len(), 10_000);
    for at in [0, 1, 4999, 9999] {
        let row = a.row_of(at).unwrap();
        let tile: Rect = a.tiles[at];
        let hit = a.rows_between(tile.center().y, tile.center().y);
        assert_eq!(hit, row..row + 1);
    }
    assert_eq!(a.row_of(10_000), None);
    assert_eq!(a.rows_between(0.0, a.height), 0..10_000);
}

// `IMG_20261101_123456.jpeg` narrow gives `IMG_20…456.jpeg`; multibyte names cut on boundaries.
#[test]
fn a_long_name_is_shortened_in_the_middle_keeping_its_extension() {
    let long = "IMG_20261101_123456.jpeg";
    assert_eq!(fit_name(long, |t| t.chars().count() <= 15), "IMG_20…456.jpeg");
    assert_eq!(fit_name(long, |_| true), long);
    assert_eq!(fit_name(long, |t| t.chars().count() <= 6), "….jpeg");
    let multibyte = fit_name("日本語のファイル名前.png", |t| t.chars().count() <= 8);
    assert!(multibyte.ends_with(".png") && multibyte.contains('…') && multibyte.chars().count() <= 8);
}

// PdfCraft's ladder: In from 1.1 gives 1.25, Out from 0.6 stays, Reset gives 1.0; Range clamps.
#[test]
fn zoom_steps_follow_the_ladder() {
    let ladder = TileZoom::Ladder;
    assert_eq!(ladder.step(1.1, ZoomStep::In), 1.25);
    assert_eq!(ladder.step(0.6, ZoomStep::Out), 0.6);
    assert_eq!(ladder.step(5.0, ZoomStep::In), 5.0);
    assert_eq!(ladder.step(3.7, ZoomStep::Reset), 1.0);
    let range = TileZoom::Range { min: 48.0, max: 400.0, step: 16.0, default: 120.0 };
    assert_eq!(range.step(50.0, ZoomStep::Out), 48.0);
    assert_eq!(range.step(395.0, ZoomStep::In), 400.0);
    assert_eq!(range.step(300.0, ZoomStep::Reset), 120.0);
    assert_eq!(range.range(), 48.0..=400.0);
}
