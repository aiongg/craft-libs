//! The size model of the size dialogs: linked fields, units, the pixel cap and the export size rules.

use craft_ui::patterns::form_dialogs::size_model::{LinkedSize, MAX_PIXELS, PixelSize, SizeError, resize_from_scale, resize_from_side};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::trial;

fn document() -> LinkedSize {
    LinkedSize::new(PixelSize::new(1200, 800))
}

fn pixels(size: &LinkedSize) -> (u32, u32) {
    (size.pixels().width, size.pixels().height)
}

/// With the lock on a side scales the other by the opening ratio, in pixels and in a unit; with it off the other side
/// stays, and a size is at least one pixel.
#[trial]
fn the_lock_scales_the_other_side_by_the_opening_ratio() {
    let mut size = document();
    size.set_width(600.0);
    assert_eq!(pixels(&size), (600, 400));
    size.set_height(200.0);
    assert_eq!(pixels(&size), (300, 200));
    assert!(size.set_unit(UnitKind::Percent));
    size.set_width(50.0);
    assert_eq!(pixels(&size), (600, 400), "a value in a unit goes through the same rule");

    size.lock(false);
    assert!(size.set_unit(UnitKind::Pixels));
    size.set_width(900.0);
    assert_eq!(pixels(&size), (900, 400));
    size.set_height(0.0);
    assert_eq!(pixels(&size).1, 1);
}

/// A side stops at 300,000 pixels, and the field's range ends there.
#[trial]
fn the_pixel_cap_stops_a_size_above_300000() {
    let mut size = document();
    size.lock(false);
    size.set_width(400_000.0);
    assert_eq!(f64::from(size.pixels().width), MAX_PIXELS);
    assert_eq!(*size.width_range().end(), MAX_PIXELS);
    assert_eq!(resize_from_side((1200, 800), 300_001.0, 10.0, false, true), Err(SizeError::Pixels));
    assert_eq!(resize_from_scale((100_000, 10), 400.0), Err(SizeError::Pixels));
}

/// The resolution typed in pixels per centimetre reads back in pixels per inch, and the pixel count stays in a pixel
/// unit.
#[trial]
fn a_resolution_in_ppcm_reads_in_ppi() {
    let mut size = document();
    assert!(size.set_resolution_unit(UnitKind::PixelsPerCentimetre));
    size.set_resolution(100.0);
    assert!((size.pixels_per_inch() - 254.0).abs() < 1e-9);
    assert_eq!(pixels(&size), (1200, 800));
    assert!(size.set_resolution_unit(UnitKind::PixelsPerInch));
    assert!((size.resolution() - 254.0).abs() < 1e-9);
    assert!(!size.set_resolution_unit(UnitKind::Pixels), "a length is not a resolution unit");
    assert!(!size.set_unit(UnitKind::PixelsPerInch), "a resolution is not a length unit");
}

/// A physical size keeps its print size when the resolution changes: the pixels follow it.
#[trial]
fn a_new_resolution_resamples_a_physical_size() {
    let mut size = document();
    assert!(size.set_unit(UnitKind::Inches));
    size.set_resolution(144.0);
    assert_eq!(pixels(&size), (2400, 1600));
    assert!((size.width() - 2400.0 / 144.0).abs() < 1e-9);
}

/// Percent is of the opening size, whatever the size has become.
#[trial]
fn percent_is_relative_to_the_opening_size() {
    let mut size = document();
    size.set_width(300.0);
    assert!(size.set_unit(UnitKind::Percent));
    assert!((size.width() - 25.0).abs() < 1e-9);
    assert!((size.height() - 25.0).abs() < 1e-9);
}

/// A typed value shows back while the pixel size it made stands: 72.27 pt at 72 ppi is 72 pixels and still reads
/// 72.27 pt, and another unit shows the pixels.
#[trial]
fn a_typed_value_shows_back_while_its_pixel_size_stands() {
    let mut size = document();
    size.lock(false);
    assert!(size.set_unit(UnitKind::Points));
    size.set_width(72.27);
    assert_eq!(size.pixels().width, 72);
    assert_eq!(size.width(), 72.27);
    assert!(size.set_unit(UnitKind::Pixels));
    assert_eq!(size.width(), 72.0);
    assert!(size.set_unit(UnitKind::Points));
    size.set_resolution(100.0);
    assert_ne!(size.width(), 72.27, "a new resolution makes the stored value stale");
}

/// The Point/Pica Size preference sets the points in an inch.
#[trial]
fn points_per_inch_sets_the_point_conversion() {
    let mut size = document().opening_resolution(300.0).points_per_inch(72.27);
    size.lock(false);
    assert!(size.set_unit(UnitKind::Points));
    size.set_width(144.54);
    assert_eq!(size.pixels().width, 600);
    assert!(size.set_unit(UnitKind::Picas));
    assert!((size.width() - 144.54 / 12.0).abs() < 1e-9);
    assert_eq!(document().points_per_inch(0.0), document(), "a value that is not positive keeps 72");
}

/// With Resample off the pixel count is fixed: a size in pixels cannot change, and a physical size changes the
/// resolution.
#[trial]
fn without_resampling_a_physical_size_changes_the_resolution() {
    let mut size = document();
    size.resampling(false);
    assert!(!size.is_editable());
    size.set_width(600.0);
    assert_eq!(pixels(&size), (1200, 800));

    assert!(size.set_unit(UnitKind::Inches));
    assert!(size.is_editable());
    size.set_width(8.0);
    assert_eq!(pixels(&size), (1200, 800));
    assert!((size.pixels_per_inch() - 150.0).abs() < 1e-9);
    assert!((size.height() - 800.0 / 150.0).abs() < 1e-9);
}

/// A relative size shows the change: typing 2 in on the height adds 2 in to it.
#[trial]
fn a_relative_size_holds_the_change() {
    let mut size = document().chain(false);
    size.relative(true);
    size.lock(false);
    assert_eq!((size.width(), size.height()), (0.0, 0.0));
    assert!(size.set_unit(UnitKind::Inches));
    size.set_height(2.0);
    assert_eq!(pixels(&size), (1200, 944));
    assert_eq!(size.height(), 2.0);
    size.set_width(-200.0 / 72.0);
    assert_eq!(size.pixels().width, 1000);
}

/// A scale outside 1 to 1000 percent, a side outside 1 to 300,000 pixels and a value that is not a number are refused;
/// a locked side follows the source's aspect.
#[trial]
fn the_export_rules_refuse_sizes_the_engine_cannot_make() {
    assert_eq!(resize_from_scale((101, 3), 50.0), Ok((51, 2)));
    assert_eq!(resize_from_scale((0, 5), 100.0), Ok((1, 5)));
    assert_eq!(resize_from_scale((100, 100), 0.0), Err(SizeError::Scale));
    assert_eq!(resize_from_scale((100, 100), 1001.0), Err(SizeError::Scale));
    assert_eq!(resize_from_scale((100, 100), f64::NAN), Err(SizeError::Scale));

    assert_eq!(resize_from_side((1200, 800), 600.0, 0.0, true, true), Ok(((600, 400), 50.0)));
    assert_eq!(resize_from_side((1200, 800), 0.0, 400.0, true, false), Ok(((600, 400), 50.0)));
    assert_eq!(resize_from_side((1200, 800), 600.0, 123.0, false, true), Ok(((600, 123), 50.0)));
    assert_eq!(resize_from_side((1200, 800), 0.4, 10.0, false, true), Err(SizeError::Pixels));
    assert_eq!(resize_from_side((1200, 800), 0.0, 10.0, false, true), Err(SizeError::Pixels));
    assert_eq!(resize_from_side((1200, 800), f64::NAN, 10.0, false, true), Err(SizeError::Pixels));
    assert_eq!(resize_from_side((1200, 800), 600.0, f64::INFINITY, false, true), Err(SizeError::Pixels));
    assert_eq!(resize_from_side((800, 1200), 300_000.0, 0.0, true, true), Err(SizeError::Pixels));
}
