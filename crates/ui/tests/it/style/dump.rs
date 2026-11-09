//! The dump writer: common values once, differing themes as extra lines, and the style groups' dump.

use craft_theme::ControlSize;
use craft_ui::style::SurfaceKind;
use craft_ui::testing::dump::{Dump, Themed, dump_resolved, flatten_debug};
use craft_ui::testing::trial;

fn entries(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

/// Fields the dump reads through `Debug` only.
#[derive(Debug)]
#[allow(dead_code, reason = "the fields are read through the derived Debug the dump prints")]
struct Inner {
    gap: f32,
    pair: (u8, u8),
}

#[derive(Debug)]
#[allow(dead_code, reason = "the fields are read through the derived Debug the dump prints")]
struct Outer {
    height: f32,
    inner: Inner,
    items: Vec<u8>,
    none: Option<u8>,
}

#[trial]
fn a_key_is_written_once_with_its_common_value_and_one_line_per_differing_theme() {
    let mut themed = Themed::new();
    themed
        .theme("a.light", entries(&[("x", "1"), ("y", "2")]))
        .theme("a.dark", entries(&[("x", "1"), ("y", "3")]))
        .theme("b.light", entries(&[("x", "1"), ("y", "2"), ("z", "9")]))
        .theme("b.dark", entries(&[("x", "4"), ("y", "2")]));
    let mut dump = Dump::new();
    themed.write(&mut dump);
    assert_eq!(dump.text(), "x = 1\nx@b.dark = 4\ny = 2\ny@a.dark = 3\nz = 9\nz@a.light = <absent>\nz@a.dark = <absent>\nz@b.dark = <absent>\n");
}

#[trial]
fn debug_values_flatten_to_dotted_keys() {
    let value = Outer { height: 24.0, inner: Inner { gap: 4.0, pair: (1, 2) }, items: vec![7, 8], none: None };
    let flat = flatten_debug(&value);
    let keys: Vec<(&str, &str)> = flat.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    assert_eq!(
        keys,
        [
            ("height", "24.0"),
            ("inner.gap", "4.0"),
            ("inner.pair.0", "1"),
            ("inner.pair.1", "2"),
            ("items.0", "7"),
            ("items.1", "8"),
            ("none", "None")
        ]
    );
}

/// Every surface kind and control size resolves in each built-in theme and appears in the dump.
#[trial]
fn style_group_dump_covers_every_surface_and_control() {
    const KINDS: [SurfaceKind; 14] = [
        SurfaceKind::Background,
        SurfaceKind::Canvas,
        SurfaceKind::Chrome,
        SurfaceKind::Panel,
        SurfaceKind::Raised,
        SurfaceKind::Pressed,
        SurfaceKind::Sunken,
        SurfaceKind::Well,
        SurfaceKind::Knob,
        SurfaceKind::Popup,
        SurfaceKind::Dialog,
        SurfaceKind::Floating,
        SurfaceKind::Scrim,
        SurfaceKind::Frame,
    ];
    let frames = craft_ui::testing::Frames::new();
    let mut dump = Dump::new();
    for kind in KINDS {
        dump_resolved(&mut dump, &format!("surface.{}", format!("{kind:?}").to_lowercase()), |tokens| {
            craft_ui::testing::style::with_cx(&frames.ctx, tokens, craft_ui::style::Variant::SECONDARY, ControlSize::Md, |cx| cx.surface(kind))
        });
    }
    for size in [ControlSize::Sm, ControlSize::Md] {
        dump_resolved(&mut dump, &format!("control.{}", format!("{size:?}").to_lowercase()), |tokens| {
            craft_ui::testing::style::with_cx(&frames.ctx, tokens, craft_ui::style::Variant::SECONDARY, size, |cx| cx.control())
        });
    }
    let keys: Vec<String> = dump.entries().map(|(key, _)| key).collect();
    let covered = |prefix: String| keys.iter().any(|key| key.starts_with(&prefix));
    for kind in KINDS {
        assert!(covered(format!("surface.{}", format!("{kind:?}").to_lowercase())), "{kind:?} is in the dump");
    }
    for size in [ControlSize::Sm, ControlSize::Md] {
        assert!(covered(format!("control.{}", format!("{size:?}").to_lowercase())), "{size:?} is in the dump");
    }
}
