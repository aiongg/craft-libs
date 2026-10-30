use super::InterfaceScale;
use egui::{Context, RawInput, ViewportId, ViewportInfo};

#[test]
fn auto_follows_the_native_scale_and_a_percent_is_absolute() {
    for native in [1.0_f32, 1.25, 2.0] {
        let n = Some(native);
        assert_eq!(InterfaceScale::Auto.pixels_per_point(n), native);
        assert_eq!(InterfaceScale::Auto.zoom_factor(n), 1.0);
        for (p, ppp) in [(100, 1.0), (150, 1.5), (200, 2.0)] {
            let s = InterfaceScale::percent(p);
            assert_eq!(s.pixels_per_point(n), ppp);
            assert!((s.zoom_factor(n) - ppp / native).abs() < 1e-6);
        }
    }
    assert_eq!(InterfaceScale::Auto.pixels_per_point(None), 1.0);
}

#[test]
fn zoom_factor_times_native_is_the_wanted_pixels_per_point() {
    for native in [None, Some(1.0), Some(1.25), Some(1.5), Some(2.0), Some(3.0)] {
        for s in std::iter::once(InterfaceScale::Auto).chain(InterfaceScale::STEPS.map(InterfaceScale::percent)) {
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
    assert_eq!(InterfaceScale::from_code("112"), Some(InterfaceScale::percent(112)));
    assert_eq!(InterfaceScale::from_code("x"), None);
    assert_eq!(InterfaceScale::from_code("20"), Some(InterfaceScale::percent(50)));
    assert_eq!(InterfaceScale::from_code("999"), Some(InterfaceScale::percent(300)));
    assert_eq!(InterfaceScale::percent(20), InterfaceScale::percent(50));
    assert_eq!(InterfaceScale::percent(1000).code(), "300");
}

#[cfg(feature = "serde")]
#[test]
fn serde_writes_the_pc_strings() {
    assert_eq!(serde_json::to_string(&InterfaceScale::Auto).unwrap_or_default(), "\"auto\"");
    assert_eq!(serde_json::to_string(&InterfaceScale::percent(125)).unwrap_or_default(), "\"125\"");
    assert_eq!(serde_json::from_str::<InterfaceScale>("\"auto\"").ok(), Some(InterfaceScale::Auto));
    assert_eq!(serde_json::from_str::<InterfaceScale>("\"300\"").ok(), Some(InterfaceScale::percent(300)));
    assert!(serde_json::from_str::<InterfaceScale>("\"x\"").is_err());
}

#[test]
fn from_relative_maps_dcs_factor() {
    assert_eq!(InterfaceScale::from_relative(1.0, Some(2.0)), InterfaceScale::Auto);
    assert_eq!(InterfaceScale::from_relative(1.5, Some(1.0)), InterfaceScale::percent(150));
    assert_eq!(InterfaceScale::from_relative(1.5, Some(2.0)), InterfaceScale::percent(300));
    assert_eq!(InterfaceScale::from_relative(1.5, None), InterfaceScale::percent(150));
    assert_eq!(InterfaceScale::from_relative(0.5, Some(1.0)), InterfaceScale::percent(75));
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
    let scale = InterfaceScale::percent(150);
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

mod fit_and_options {
    use super::super::{Fit, InterfaceScale, InterfaceScaleLabels};
    use crate::localized::LocaleNumbers;
    use egui::vec2;

    fn small_screen() -> Fit {
        Fit { monitor_px: Some(vec2(1366.0, 768.0)), ..Fit::default() }
    }

    fn codes(options: &[(String, String)]) -> Vec<&str> {
        options.iter().map(|(c, _)| c.as_str()).collect()
    }

    #[test]
    fn fit_hides_scales_that_leave_too_small_a_window_and_keeps_the_current_one() {
        let fit = small_screen();
        let allowed = |p| fit.allows(InterfaceScale::percent(p));
        assert!(allowed(125));
        assert!(!allowed(150) && !allowed(300));
        assert!(fit.allows(InterfaceScale::Auto));
        assert!(Fit::default().allows(InterfaceScale::percent(300)));

        let (labels, numbers) = (InterfaceScaleLabels::default(), LocaleNumbers::from_tag("en"));
        let list = InterfaceScale::options(InterfaceScale::Auto, &labels, &numbers, fit);
        assert_eq!(codes(&list).last(), Some(&"125"));
        let list = InterfaceScale::options(InterfaceScale::percent(300), &labels, &numbers, fit);
        assert_eq!(codes(&list).last(), Some(&"300"));
        assert!(!codes(&list).contains(&"150"));
    }

    #[test]
    fn options_add_the_current_off_step_value_once() {
        let (labels, numbers) = (InterfaceScaleLabels::default(), LocaleNumbers::from_tag("en"));
        let list = InterfaceScale::options(InterfaceScale::percent(112), &labels, &numbers, Fit::default());
        assert_eq!(list.iter().filter(|(c, _)| c == "112").count(), 1);
        assert_eq!(list[0], ("auto".to_owned(), "Auto".to_owned()));
        let at = list.iter().position(|(c, _)| c == "112").unwrap();
        assert_eq!(list[at].1, "112%");
        assert_eq!((list[at - 1].0.as_str(), list[at + 1].0.as_str()), ("110", "115"));

        let on_step = InterfaceScale::options(InterfaceScale::percent(100), &labels, &numbers, Fit::default());
        assert_eq!(on_step.len(), 1 + InterfaceScale::STEPS.len());
    }
}
