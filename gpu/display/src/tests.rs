//! Display colour checks that need no GPU. Fixtures are built-in (CC0) or synthesized profiles.

use std::sync::Arc;

use craft_cms::{Builtin, Profile, ProfileClass};
use craft_wgsl::{linear_to_srgb, powz};

use crate::gpu::{DISPLAY, texture_access};
use crate::pass::{compand, shaper_apply, tone_map};
use crate::{
    Channel, Curve, Detection, DisplayCaches, DisplayPass, DisplayTransform, FrameEncoding, HdrStep, Monitor, MonitorProblem, MonitorStatus,
    MonitorsError, display_at, rgb_view, shaper_from_curves,
};

const SRGB: FrameEncoding = FrameEncoding { curve: Curve::Srgb, hdr: HdrStep::None };

fn profile(b: Builtin) -> Arc<Profile> {
    Arc::new(b.profile().clone())
}

fn monitor(id: u32, frame: [f64; 4], icc: Option<Vec<u8>>) -> Monitor {
    Monitor { id, name: format!("Display {id}"), frame, primary: false, profile_name: None, icc: icc.map(Arc::new) }
}

fn p3_bytes() -> Option<Vec<u8>> {
    Some(Builtin::DisplayP3.profile().to_bytes().to_vec())
}

fn close(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
}

fn q(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8
}

/// An opaque grey of linear light `l`, sRGB-encoded as float frames store it.
fn grey(l: f32) -> [f32; 4] {
    let e = Curve::Srgb.encode(l);
    [e, e, e, 1.0]
}

fn rgb(encoding: FrameEncoding, stops: f32) -> DisplayPass {
    DisplayPass::new(Channel::Rgb, false, stops, encoding, None)
}

/// Guards the whole chain from a composite profile to the screen, exact and through the form the
/// GPU runs (shaper tables or LUT): a wrong intent or BPC, a dropped matrix or a gray composite
/// shown raw would show a P3, Adobe RGB, ProPhoto or Gray 2.2 colour desaturated.
#[test]
fn known_profiles_show_as_computed() {
    let srgb = profile(Builtin::Srgb);
    // P3 (0.8, 0.5, 0.3): linear (0.6038, 0.2140, 0.0732) → linear sRGB through the published
    // P3 → sRGB matrix (0.6915, 0.1976, 0.0517) → encoded (216.7, 122.9, 64.3).
    let p3 = DisplayTransform::new(profile(Builtin::DisplayP3), srgb.clone()).unwrap();
    let got = p3.apply([0.8, 0.5, 0.3]).map(|v| v * 255.0);
    assert!(close(got, [216.7, 122.9, 64.3], 2.0), "{got:?}");
    // Gray Gamma 2.2: gray 0.1 is L = 0.1^2.2 = 0.00631 → sRGB 18.6 (raw would be 25.5).
    let gray = DisplayTransform::new(rgb_view(Builtin::GrayGamma22.profile()), srgb.clone()).unwrap();
    assert!(close(gray.apply([0.1; 3]).map(|v| v * 255.0), [18.6; 3], 2.0));

    let cases: [(Arc<Profile>, [f32; 3]); 4] = [
        (profile(Builtin::DisplayP3), [0.8, 0.5, 0.3]),
        (profile(Builtin::AdobeRgbCompat), [0.3, 0.7, 0.4]),
        (profile(Builtin::ProPhotoCompat), [0.55, 0.35, 0.25]),
        (rgb_view(Builtin::GrayGamma22.profile()), [0.1; 3]),
    ];
    for (composite, v) in cases {
        let t = DisplayTransform::new(composite.clone(), srgb.clone()).unwrap();
        let exact = t.apply(v).map(|x| x * 255.0);
        let pass = DisplayPass::new(Channel::Rgb, false, 0.0, SRGB, t.pass_color().unwrap());
        let px = pass.pixel([v[0], v[1], v[2], 1.0]);
        let shown = [px[0] as f32, px[1] as f32, px[2] as f32];
        assert!(close(shown, exact, 2.0), "{}: pass {shown:?}, exact {exact:?}", composite.description);
        assert!(!close(shown, v.map(|x| x * 255.0), 4.0), "{}: shown raw {shown:?}", composite.description);
    }
}

/// Guards the identity check (9³ lattice, half an 8-bit step; the neutral axis for gray): sRGB on
/// sRGB must cost no conversion, and a real difference must never be skipped.
#[test]
fn same_profiles_are_the_identity() {
    let srgb = profile(Builtin::Srgb);
    let same = DisplayTransform::new(srgb.clone(), srgb.clone()).unwrap();
    assert!(same.is_identity() && same.pass_color().unwrap().is_none());
    assert!(DisplayTransform::new(profile(Builtin::DisplayP3), profile(Builtin::DisplayP3)).unwrap().is_identity());
    assert!(DisplayTransform::new(rgb_view(Builtin::SGray.profile()), srgb.clone()).unwrap().is_identity(), "sGray's RGB view is sRGB");
    assert!(!DisplayTransform::new(rgb_view(Builtin::GrayGamma22.profile()), srgb.clone()).unwrap().is_identity());
    assert!(!DisplayTransform::new(srgb.clone(), profile(Builtin::DisplayP3)).unwrap().is_identity());
}

/// Guards the texture encoding of linear composites: a linear profile no longer detected, or the
/// chunked parallel encode drifting from the scalar curve on a ragged last piece or on NaN,
/// negative or over-range values.
#[test]
fn linear_composites_are_encoded_for_the_texture() {
    let t = DisplayTransform::new(profile(Builtin::LinearSrgb), profile(Builtin::Srgb)).unwrap();
    assert!(t.encode_srgb && t.transform.is_none() && !t.is_identity(), "linear sRGB on sRGB: encoding is all it takes");
    let n: usize = 3 * 16 * 1024 + 77;
    let px: Vec<[f32; 4]> = (0..n)
        .map(|i| {
            let v = match i % 9 {
                0 => f32::NAN,
                1 => -0.5,
                2 => 3.25,
                3 => 0.2,
                _ => (i % 1001) as f32 / 1000.0,
            };
            [v, (i % 257) as f32 / 256.0, 1.0 - (i % 113) as f32 / 112.0, (i % 5) as f32 / 4.0]
        })
        .collect();
    let mut tex = px.clone();
    t.encode_texture(&mut tex);
    let enc = |v: f32| linear_to_srgb(v.clamp(0.0, 65504.0));
    for (o, p) in tex.iter().zip(&px) {
        let want = [enc(p[0]), enc(p[1]), enc(p[2]), p[3]];
        assert!(o.iter().zip(want).all(|(a, b)| a.to_bits() == b.to_bits()), "{p:?} -> {o:?}, want {want:?}");
    }
    assert!((tex[3][0] - 0.4845).abs() < 1e-3, "linear 0.2 is sRGB 0.4845");
    let plain = DisplayTransform::new(profile(Builtin::Srgb), profile(Builtin::Srgb)).unwrap();
    let mut same = px.clone();
    plain.encode_texture(&mut same);
    assert!(same.iter().flatten().zip(px.iter().flatten()).all(|(a, b)| a.to_bits() == b.to_bits()));
}

/// Guards per-display resolution: overlap picking the wrong display, unknown displays not using
/// the primary, a stale cached transform after the monitor profile changes, or the texture key
/// following the monitor.
#[test]
fn each_display_resolves_its_own_profile() {
    let caches = DisplayCaches::new();
    // A built-in primary display left, a wide-gamut one right and higher up (y down).
    let mut primary = monitor(1, [0.0, 0.0, 1728.0, 1117.0], None);
    primary.primary = true;
    caches.set_monitors(Ok(vec![primary, monitor(4, [1728.0, -667.0, 3008.0, 1692.0], p3_bytes())]));
    let list = caches.monitors();
    let at = |r: [f64; 4]| display_at(&list, r);
    assert_eq!(at([100.0, 100.0, 800.0, 600.0]), Some(1));
    assert_eq!(at([2000.0, -500.0, 800.0, 600.0]), Some(4));
    assert_eq!(at([1500.0, 100.0, 800.0, 600.0]), Some(4), "spanning both: the one showing more");
    assert_eq!(at([1200.0, 100.0, 800.0, 600.0]), Some(1));
    assert_eq!(at([-5000.0, 0.0, 10.0, 10.0]), None);

    let srgb_hash = Builtin::Srgb.profile().content_hash();
    assert_eq!(caches.monitor_for(Some(4)).description, "Display P3");
    assert_eq!(caches.monitor_for(Some(1)).content_hash(), srgb_hash);
    assert_eq!(caches.monitor_for(None).content_hash(), srgb_hash, "unknown: the primary");
    assert_eq!(caches.monitor_for(Some(99)).content_hash(), srgb_hash);

    let src = profile(Builtin::Srgb);
    let a = caches.canvas_display_for(&src, Some(1)).unwrap();
    let b = caches.canvas_display_for(&src, Some(4)).unwrap();
    assert!(a.is_identity() && !b.is_identity());
    assert_ne!(a.key, b.key);
    assert_eq!(a.texture_key, b.texture_key, "one texture serves both displays");
    assert_eq!(b.key, caches.display_signature(&src, Some(4)));
    assert!(Arc::ptr_eq(&a, &caches.canvas_display_for(&src, Some(1)).unwrap()));

    // Choosing a monitor profile changes the signature and drops the cached transform.
    let before = caches.display_signature(&src, Some(1));
    caches.choose_monitor(Some(profile(Builtin::DisplayP3)));
    assert_eq!(caches.monitor_report(Some(1)).status, MonitorStatus::Chosen);
    assert_ne!(caches.display_signature(&src, Some(1)), before);
    let chosen = caches.canvas_display_for(&src, Some(1)).unwrap();
    assert!(!Arc::ptr_eq(&a, &chosen) && !chosen.is_identity());
    caches.choose_monitor(None);
    assert_eq!(caches.display_signature(&src, Some(1)), before);
}

/// Guards the fallbacks and their reasons: a bad, non-RGB or unlinkable profile used anyway, a
/// failed re-read discarding a good list, or the wrong status before or without a reading.
#[test]
fn monitor_status_says_why() {
    let c = DisplayCaches::new();
    assert_eq!(c.monitor_report(None).status, MonitorStatus::Unsupported);
    c.set_pending();
    assert_eq!(c.monitor_report(None).status, MonitorStatus::Pending);

    let mut studio = monitor(7, [0.0, 0.0, 100.0, 100.0], p3_bytes());
    studio.name = "Studio Display".into();
    studio.profile_name = Some("Studio Display (calibrated)".into());
    c.set_monitors(Ok(vec![studio]));
    let r = c.monitor_report(None);
    assert_eq!((r.status, r.profile.as_str(), r.display.as_deref()), (MonitorStatus::Detected, "Display P3", Some("Studio Display")));
    assert_eq!(
        (r.profile_name.as_deref(), r.fingerprint, r.detection),
        (Some("Studio Display (calibrated)"), Builtin::DisplayP3.profile().content_hash(), Detection::Found)
    );

    let mut link = Builtin::Srgb.profile().clone();
    link.class = ProfileClass::DeviceLink;
    let link = link.with_encoded_bytes();
    let cases: [(Option<Vec<u8>>, fn(&MonitorStatus) -> bool); 4] = [
        (Some(vec![0u8; 16]), |s| matches!(s, MonitorStatus::Unusable(MonitorProblem::Unreadable { .. }))),
        (Some(Builtin::SGray.profile().to_bytes().to_vec()), |s| matches!(s, MonitorStatus::Unusable(MonitorProblem::NotRgb { .. }))),
        (Some(link.to_bytes().to_vec()), |s| matches!(s, MonitorStatus::Unusable(MonitorProblem::CannotLink { .. }))),
        (None, |s| *s == MonitorStatus::NoProfile),
    ];
    for (icc, expected) in cases {
        c.set_monitors(Ok(vec![monitor(2, [0.0, 0.0, 100.0, 100.0], icc)]));
        let status = c.monitor_report(None).status;
        assert!(expected(&status), "{status:?}");
        assert_eq!(c.monitor_for(None).content_hash(), Builtin::Srgb.profile().content_hash());
    }

    // A failed re-read keeps the list read before and is returned for the log.
    c.set_monitors(Ok(vec![monitor(2, [0.0, 0.0, 100.0, 100.0], p3_bytes())]));
    assert_eq!(c.set_monitors(Err(MonitorsError::Failed("reader failed".into()))).as_deref(), Some("reader failed"));
    let r = c.monitor_report(None);
    assert_eq!((r.status, r.detection), (MonitorStatus::Detected, Detection::Retained { reason: "reader failed".into() }));
    assert_eq!(c.monitors().len(), 1);

    // Without a list read before, the failure is the reason.
    let fresh = DisplayCaches::new();
    assert_eq!(fresh.set_monitors(Err(MonitorsError::Failed("reader failed".into()))), None);
    assert_eq!(fresh.monitor_report(None).status, MonitorStatus::ReadFailed { reason: "reader failed".into() });
}

/// Guards exposure on the float range: applied after clamping, over-range values could never be
/// brought back (4.0 at −2 EV is white), and values of the 8-bit range must keep their bytes.
#[test]
fn exposure_reaches_over_range_values() {
    let table: [(f32, [u8; 5]); 6] = [
        (0.25, [137, 99, 71, 34, 188]),
        (0.5, [188, 137, 99, 49, 255]),
        (1.0, [255, 188, 137, 71, 255]),
        (2.0, [255, 255, 188, 99, 255]),
        (4.0, [255, 255, 255, 137, 255]),
        (16.0, [255, 255, 255, 255, 255]),
    ];
    for (l, want) in table {
        for (stops, w) in [0.0, -1.0, -2.0, -4.0, 1.0].into_iter().zip(want) {
            assert_eq!(rgb(SRGB, stops).pixel(grey(l))[0], w, "linear {l} at {stops} EV");
        }
    }
    assert_eq!(rgb(SRGB, -2.0).pixel(grey(4.0)), [255, 255, 255, 255]);
    // Premultiplied: alpha stays, colour scales with it.
    let e = Curve::Srgb.encode(4.0);
    let half = rgb(SRGB, -2.0).pixel([e * 0.5, e * 0.5, e * 0.5, 0.5]);
    assert!(half[3] == 128 && half[..3].iter().all(|c| (127..=128).contains(c)), "{half:?}");
}

/// Guards the order of the light steps and the display colour: exposure, then the HDR step, then
/// the preview gamma, then the display conversion, and the conversion only on colour views.
#[test]
fn light_steps_run_in_order() {
    let compand_enc = FrameEncoding { curve: Curve::Srgb, hdr: HdrStep::Compand };
    // 0.6 is below the 0.8 knee: unchanged at 0 EV, companded (not clipped) at +1 EV.
    assert_eq!(rgb(compand_enc, 0.0).pixel(grey(0.6))[0], q(linear_to_srgb(0.6)));
    let up = rgb(compand_enc, 1.0).pixel(grey(0.6))[0];
    assert_eq!(up, q(linear_to_srgb(compand(1.2))));
    assert!(up < 255, "{up}");
    // 3.0 at −2 EV is 0.75, under the knee: companding first would have made it about 0.24.
    let down = rgb(compand_enc, -2.0).pixel(grey(3.0))[0];
    assert_eq!(down, q(linear_to_srgb(0.75)));
    let tm = FrameEncoding { curve: Curve::Srgb, hdr: HdrStep::ToneMap };
    for (l, stops) in [(0.6f32, 1.0f32), (3.0, -2.0), (16.0, -1.0), (0.1, 0.0)] {
        let k = 2f32.powf(stops);
        assert_eq!(rgb(tm, stops).pixel(grey(l))[0], q(linear_to_srgb(tone_map([l * k; 3])[0])), "{l} at {stops}");
    }
    // The preview gamma comes after the HDR step.
    let mut gamma = rgb(compand_enc, 1.0);
    gamma.gamma = 2.0;
    assert_eq!(gamma.pixel(grey(0.6))[0], q(linear_to_srgb(powz(compand(1.2), 0.5))));
    // The display colour converts the exposed values.
    let color = DisplayTransform::new(profile(Builtin::Srgb), profile(Builtin::DisplayP3)).unwrap().pass_color().unwrap().unwrap();
    let pass = DisplayPass::new(Channel::Rgb, false, 1.0, SRGB, Some(color.clone()));
    let red = [Curve::Srgb.encode(0.4), 0.0, 0.0, 1.0];
    let want = color.apply([linear_to_srgb(0.8), 0.0, 0.0]).map(q);
    assert_eq!(pass.pixel(red)[..3], want);
    assert!(DisplayPass::new(Channel::Red, false, 0.0, SRGB, Some(color)).color.is_none());
}

/// Guards the CPU path's table shortcut: it must give what `pixel` gives for every value of every
/// channel view, opaque and not, with and without exposure.
#[test]
fn the_8_bit_tables_are_the_pass() {
    for (channel, colorized) in [
        (Channel::Rgb, false),
        (Channel::RgbStraight, false),
        (Channel::Red, false),
        (Channel::Green, true),
        (Channel::Blue, false),
        (Channel::Alpha, false),
    ] {
        for stops in [0.0f32, 1.0, -2.5] {
            let pass = DisplayPass::new(channel, colorized, stops, SRGB, None);
            let px: Vec<[u8; 4]> = (0..256 * 4)
                .map(|i| {
                    let v = (i % 256) as u8;
                    let a = [255u8, 255, 128, 3][i / 256];
                    let c = |k: u8| ((v as u32 * k as u32) % 256 * a as u32 / 255) as u8;
                    [c(1), c(7), c(13), a]
                })
                .collect();
            let mut got = px.clone();
            pass.apply_rgba8(&mut got);
            for (p, g) in px.iter().zip(&got) {
                assert_eq!(*g, pass.pixel(p.map(|v| v as f32 / 255.0)), "{channel:?} {colorized} {stops}: {p:?}");
            }
        }
    }
}

/// Guards the lowering of conversions given as formulas: shaper tables sampled from a decode
/// curve, a matrix and an encode curve must give the formula's result on the display's range.
#[test]
fn formula_shapers_match_their_curves() {
    // The published Display P3 → sRGB matrix (linear light).
    let m = [[1.2249, -0.2247, 0.0], [-0.0420, 1.0419, 0.0], [-0.0197, -0.0786, 1.0979]];
    for (from, matrix, to) in [(Curve::Srgb, Some(m), Curve::Srgb), (Curve::Pq, None, Curve::Gamma(2.4)), (Curve::Hlg, Some(m), Curve::Srgb)] {
        let s = shaper_from_curves(from, matrix, to).unwrap();
        for i in 0..=16 {
            for c in [[i as f32 / 16.0, 0.3, 0.9], [0.05, i as f32 / 16.0, 0.5], [1.0, 1.0, i as f32 / 16.0]] {
                let l = c.map(|v| from.decode(v));
                let l = match matrix {
                    Some(m) => std::array::from_fn(|r| m[r][0] * l[0] + m[r][1] * l[1] + m[r][2] * l[2]),
                    None => l,
                };
                let want = l.map(|v| to.encode(v.clamp(0.0, 1.0)));
                let got = shaper_apply(&s, c);
                assert!(close(got, want, 1e-3), "{from:?} → {to:?} at {c:?}: {got:?} vs {want:?}");
            }
        }
    }
}

/// Guards the WGSL pass where CI has no GPU: a module that doesn't parse or validate together
/// with the lookups `texture_access` writes.
#[test]
fn display_module_validates() {
    let entry = "@group(0) @binding(2) var<uniform> display_params: DisplayParams;\n\
                 @fragment fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {\n\
                 \x20   let t = display_lut(0u, 0u, 0u);\n\
                 \x20   return display_pixel(display_params, vec4<f32>(display_table(u32(pos.x)), t.w));\n\
                 }\n";
    let src = craft_wgsl::assemble(&[&DISPLAY], &[&texture_access(0, 0, 1), entry]);
    let module = naga::front::wgsl::parse_str(&src).unwrap_or_else(|e| panic!("{}", e.emit_to_string(&src)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}"));
}
