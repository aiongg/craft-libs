use super::InterfaceScale;
use egui::{Context, RawInput, ViewportId, ViewportInfo};

#[test]
fn auto_follows_the_native_scale_and_a_percent_is_absolute() {
    for native in [1.0_f32, 1.25, 2.0] {
        let n = Some(native);
        assert_eq!(InterfaceScale::Auto.pixels_per_point(n), native);
        assert_eq!(InterfaceScale::Auto.zoom_factor(n), 1.0);
        for (p, ppp) in [(100, 1.0), (150, 1.5), (200, 2.0)] {
            let s = InterfaceScale::Percent(p);
            assert_eq!(s.pixels_per_point(n), ppp);
            assert!((s.zoom_factor(n) - ppp / native).abs() < 1e-6);
        }
    }
    assert_eq!(InterfaceScale::Auto.pixels_per_point(None), 1.0);
}

#[test]
fn zoom_factor_times_native_is_the_wanted_pixels_per_point() {
    for native in [None, Some(1.0), Some(1.25), Some(1.5), Some(2.0), Some(3.0)] {
        for s in std::iter::once(InterfaceScale::Auto).chain(InterfaceScale::STEPS.map(InterfaceScale::Percent)) {
            let got = s.zoom_factor(native) * native.unwrap_or(1.0);
            assert!((got - s.pixels_per_point(native)).abs() < 1e-5, "{s:?} at {native:?}");
        }
    }
}

#[test]
fn codes_round_trip_and_off_step_values_are_kept() {
    for code in ["auto", "75", "300", "112"] {
        let s = InterfaceScale::from_code(code);
        assert_eq!(s.map(InterfaceScale::code).as_deref(), Some(code));
    }
    assert_eq!(InterfaceScale::from_code("112"), Some(InterfaceScale::Percent(112)));
    assert_eq!(InterfaceScale::from_code("x"), None);
    assert_eq!(InterfaceScale::from_code("20"), Some(InterfaceScale::Percent(50)));
    assert_eq!(InterfaceScale::from_code("999"), Some(InterfaceScale::Percent(300)));
    assert_eq!(InterfaceScale::percent(20), InterfaceScale::Percent(50));
}

#[cfg(feature = "serde")]
#[test]
fn serde_writes_the_pc_strings() {
    assert_eq!(serde_json::to_string(&InterfaceScale::Auto).unwrap_or_default(), "\"auto\"");
    assert_eq!(serde_json::to_string(&InterfaceScale::Percent(125)).unwrap_or_default(), "\"125\"");
    assert_eq!(serde_json::from_str::<InterfaceScale>("\"auto\"").ok(), Some(InterfaceScale::Auto));
    assert_eq!(serde_json::from_str::<InterfaceScale>("\"300\"").ok(), Some(InterfaceScale::Percent(300)));
    assert!(serde_json::from_str::<InterfaceScale>("\"x\"").is_err());
}

#[test]
fn from_relative_maps_dcs_factor() {
    assert_eq!(InterfaceScale::from_relative(1.0, Some(2.0)), InterfaceScale::Auto);
    assert_eq!(InterfaceScale::from_relative(1.5, Some(1.0)), InterfaceScale::Percent(150));
    assert_eq!(InterfaceScale::from_relative(1.5, Some(2.0)), InterfaceScale::Percent(300));
    assert_eq!(InterfaceScale::from_relative(1.5, None), InterfaceScale::Percent(150));
    assert_eq!(InterfaceScale::from_relative(0.5, Some(1.0)), InterfaceScale::Percent(75));
    assert_eq!(InterfaceScale::from_relative(f32::NAN, Some(1.0)), InterfaceScale::Auto);
}

fn frame(ctx: &Context, native: Option<f32>) {
    let mut input = RawInput::default();
    input.viewports.insert(ViewportId::ROOT, ViewportInfo { native_pixels_per_point: native, ..Default::default() });
    let mut output = ctx.run_ui(input, |_| {});
    output.textures_delta.clear();
}

#[test]
fn apply_sets_the_zoom_once_and_not_every_frame() {
    let ctx = Context::default();
    let scale = InterfaceScale::Percent(150);
    let mut writes = Vec::new();

    // Native is unknown on the first frame and arrives on the second.
    for native in [None, Some(2.0), Some(2.0)] {
        frame(&ctx, native);
        writes.push(scale.sync(&ctx));
        frame(&ctx, native);
    }
    assert_eq!(writes, [true, true, false]);
    assert!((ctx.pixels_per_point() - 1.5).abs() < 1e-6);

    // Further frames write nothing.
    frame(&ctx, Some(2.0));
    assert!(!scale.sync(&ctx));
}

#[test]
fn keyboard_zoom_can_be_turned_off() {
    let ctx = Context::default();
    assert!(ctx.options(|o| o.zoom_with_keyboard));
    InterfaceScale::disable_keyboard_zoom(&ctx);
    assert!(!ctx.options(|o| o.zoom_with_keyboard));
}
