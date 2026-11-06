//! Token groups: literal defaults reach paint through `cx.color`, a theme's values replace them, and a group is
//! built once per token set.

use craft_theme::{Contrast, ControlSize, GroupValues, Mode, Rgba, TokenGroup, Tokens, Variant};
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Frames, trial};
use std::sync::atomic::{AtomicUsize, Ordering};

static DERIVES: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, PartialEq, Debug)]
struct ClipColors {
    audio: Rgba,
}

// derive output
impl TokenGroup for ClipColors {
    const PATH: &'static str = "craft_ui_tests.clips";

    fn derive(_: &Tokens) -> Self {
        DERIVES.fetch_add(1, Ordering::SeqCst);
        ClipColors { audio: Rgba::rgb(10, 20, 30) }
    }

    fn overlay(&mut self, values: &GroupValues) {
        if let Some(color) = values.color(Self::PATH, "audio") {
            self.audio = color;
        }
    }
}

#[trial]
fn a_group_colour_reaches_paint_and_a_theme_value_under_its_path_replaces_it() {
    let frames = Frames::new();
    let base = tokens(Mode::Light, Contrast::Standard);
    let mut values = GroupValues::new();
    values.set_color(ClipColors::PATH, "audio", Rgba::rgb(200, 100, 50));
    let themed = base.with_groups(values);
    with_cx(&frames.ctx, &base, Variant::SECONDARY, ControlSize::Md, |cx| {
        assert_eq!(cx.color(cx.token_group::<ClipColors>().audio), cx.color(Rgba::rgb(10, 20, 30)));
    });
    with_cx(&frames.ctx, &themed, Variant::SECONDARY, ControlSize::Md, |cx| {
        assert_eq!(cx.color(cx.token_group::<ClipColors>().audio), cx.color(Rgba::rgb(200, 100, 50)));
    });
}

static COUNTED: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct CountedColors;

// derive output
impl TokenGroup for CountedColors {
    const PATH: &'static str = "craft_ui_tests.counted";

    fn derive(_: &Tokens) -> Self {
        COUNTED.fetch_add(1, Ordering::SeqCst);
        CountedColors
    }
}

#[trial]
fn a_group_is_derived_once_per_token_set() {
    let frames = Frames::new();
    let set = tokens(Mode::Dark, Contrast::Standard);
    let before = COUNTED.load(Ordering::SeqCst);
    with_cx(&frames.ctx, &set, Variant::SECONDARY, ControlSize::Md, |cx| {
        for _ in 0..4 {
            cx.token_group::<CountedColors>();
        }
    });
    assert_eq!(COUNTED.load(Ordering::SeqCst) - before, 1);
}
