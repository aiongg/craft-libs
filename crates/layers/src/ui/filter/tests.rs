//! Tests of the filter rule (L1).

use super::{FilterBy, FilterKind, LayerFilter};
use crate::ui::tree::{LayerInfo, LayerKind};

fn layer(name: &str, kind: LayerKind, blend_key: &str) -> LayerInfo {
    let mut info = LayerInfo::new(name, kind);
    info.blend_key = blend_key.to_owned();
    info
}

// Mode always filters, Name trims and ignores case, Kind matches `filter_kind`, groups match no kind,
// an inactive filter shows every layer.
#[test]
fn filter_rules_for_kind_name_and_mode() {
    let pixel = layer("Sky Photo", LayerKind::Pixel, "normal");
    let fill = layer("Colour", LayerKind::Fill, "multiply");
    let group = layer("Group 1", LayerKind::Group, "passThrough");

    let none = LayerFilter::default();
    assert!(!none.active());
    assert!([&pixel, &fill, &group].into_iter().all(|info| none.shows(info)));

    let kinds = LayerFilter { kinds: vec![FilterKind::Adjustment], ..LayerFilter::default() };
    assert!(kinds.active());
    assert!(kinds.shows(&fill) && !kinds.shows(&pixel) && !kinds.shows(&group), "fill layers are adjustments; groups match no kind");

    let blank = LayerFilter { by: FilterBy::Name, name: String::from("   "), ..LayerFilter::default() };
    assert!(!blank.active() && blank.shows(&group));
    let name = LayerFilter { by: FilterBy::Name, name: String::from("  sky "), ..LayerFilter::default() };
    assert!(name.shows(&pixel) && !name.shows(&fill));

    let mode = LayerFilter { by: FilterBy::Mode, ..LayerFilter::default() };
    assert!(mode.active(), "Mode always filters, by Normal at first");
    assert!(mode.shows(&pixel) && !mode.shows(&fill));
    let multiply = LayerFilter { mode: String::from("multiply"), ..mode };
    assert!(multiply.shows(&fill) && !multiply.shows(&pixel));
}
