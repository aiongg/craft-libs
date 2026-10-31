//! The pixel grid.

use egui::Color32;

use super::{PixelGrid, PixelRect};

// shows_at(5.0) is false, shows_at(5.01) true.
#[test]
fn the_grid_shows_only_above_five_hundred_percent() {
    assert!(!PixelGrid::shows_at(5.0));
    assert!(!PixelGrid::shows_at(1.0));
    assert!(PixelGrid::shows_at(5.01));
    assert!(PixelGrid::shows_at(128.0));
}

// cell_color of alpha 0 or NaN is None; opaque dark gives a light line, opaque light a dark one.
#[test]
fn no_line_over_transparent_pixels() {
    assert_eq!(PixelGrid::cell_color([0.0; 4]), None);
    assert_eq!(PixelGrid::cell_color([1.0, 1.0, 1.0, 0.0]), None);
    assert_eq!(PixelGrid::cell_color([0.5, 0.5, 0.5, f32::NAN]), None);
    assert_eq!(PixelGrid::cell_color([0.0, 0.0, 0.0, 0.001]), None);
    assert_eq!(PixelGrid::cell_color([0.1, 0.1, 0.1, 1.0]), Some(Color32::from_white_alpha(64)));
    assert_eq!(PixelGrid::cell_color([0.9, 0.9, 0.9, 1.0]), Some(Color32::from_black_alpha(41)));
    assert_eq!(PixelGrid::light_share(0.2), 0.0);
    assert_eq!(PixelGrid::light_share(0.9), 1.0);
    assert_eq!(PixelGrid::light_share(f32::NAN), 0.0);
}

// A uniform 4 × 4 region gives one segment per grid line.
#[test]
fn runs_of_one_colour_merge_into_one_segment() {
    let px = [[0.1, 0.1, 0.1, 1.0]; 16];
    let segments = PixelGrid::segments(&px, PixelRect::new(10, 20, 14, 24));
    // Four vertical lines and four horizontal ones.
    assert_eq!(segments.len(), 8);
    assert!(segments.iter().any(|s| s.from == [10, 20] && s.to == [10, 24]));
    assert!(segments.iter().any(|s| s.from == [10, 23] && s.to == [14, 23]));
    // A transparent cell splits the run.
    let mut holey = px;
    holey[5] = [0.0; 4];
    assert!(PixelGrid::segments(&holey, PixelRect::new(0, 0, 4, 4)).len() > 8);
}
