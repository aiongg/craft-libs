//! Tests of the filter rule (L1).

use super::{FilterBy, FilterKind, LayerFilter};
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
use crate::ui::panel::LayersStyle;
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

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod bar {
    use craft_ui::theme::{Base, current, install};
    use egui::accesskit::Role;
    use egui::{Context, vec2};
    use egui_kittest::HarnessBuilder;
    use egui_kittest::kittest::Queryable as _;

    use super::{LayerFilter, LayersStyle};
    use crate::ui::filter::LayerFilterBar;

    // The first control after the type select is `layers.filter.gap` plus `layers.filter.type_gap`
    // from it (PhotoCraft adds 4 pt after the select).
    #[test]
    fn the_type_select_is_followed_by_the_gap_and_the_type_gap() {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let style = LayersStyle::of(&current(&ctx));
        let mut filter = LayerFilter::default();
        let mut harness = HarnessBuilder::default()
            .with_size(vec2(400.0, 100.0))
            .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
            .build_ui(move |ui| {
                let _ = LayerFilterBar::new("filter", &mut filter).show(ui);
            });
        harness.run();
        let select = harness.get_by_role(Role::ComboBox).rect();
        let next = harness
            .get_all_by_role(Role::Button)
            .map(|n| n.rect())
            .filter(|r| r.left() >= select.right())
            .fold(f32::INFINITY, |m, r| m.min(r.left()));
        let gap = next - select.right();
        assert!((gap - (style.filter_gap + style.filter_type_gap)).abs() < 1.5, "gap {gap}");
    }
}
