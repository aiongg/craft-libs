//! Variants and their acceptance by a component's style key.

use craft_theme::variant::Tone;
use craft_theme::{Contrast, ControlSize, CustomVariant, Emphasis, Mode, States, Tokens, Variant, VariantLook};
use craft_ui::style::{ComponentStyle, Metrics, StyleKey, accepted_variant};
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Frames, trial};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ButtonKey;
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct OtherKey;
// derive output
impl StyleKey for ButtonKey {}
// derive output
impl StyleKey for OtherKey {}

fn record_look(tokens: &Tokens) -> VariantLook {
    let bad = tokens.color.status.bad;
    VariantLook::new(States::new(bad.solid, bad.solid_hover, bad.solid_pressed, bad.bg), States::new(bad.fg, bad.fg, bad.fg, bad.fg))
}

const RECORD: CustomVariant = CustomVariant::new::<ButtonKey>("craft_ui_tests.record", record_look);

#[derive(Clone, PartialEq)]
struct ButtonStyle;

// derive output
impl ComponentStyle for ButtonStyle {
    const VARIANTS: &'static [Variant] = &[Variant::SECONDARY, Variant::PRIMARY, Variant::GHOST, Variant::DANGER];

    fn metrics(&self) -> Metrics {
        Metrics::new()
    }
}

#[trial]
fn each_listed_variant_differs_from_the_default() {
    let frames = Frames::new();
    for (mode, contrast) in [(Mode::Light, Contrast::Standard), (Mode::Dark, Contrast::Standard), (Mode::Light, Contrast::High)] {
        let tokens = tokens(mode, contrast);
        let looks: Vec<_> =
            ButtonStyle::VARIANTS.iter().map(|variant| with_cx(&frames.ctx, &tokens, *variant, ControlSize::Md, |cx| cx.variant_look())).collect();
        for (i, look) in looks.iter().enumerate().skip(1) {
            assert_ne!(*look, looks[0], "variant {i} falls back to the default look in {mode:?} {contrast:?}");
        }
    }
}

#[trial]
fn a_component_accepts_its_variants_and_the_custom_ones_built_for_its_key() {
    let accepted = accepted_variant::<ButtonKey, ButtonStyle>(Variant::PRIMARY);
    assert_eq!((accepted.variant, accepted.fell_back), (Variant::PRIMARY, false));
    let custom = accepted_variant::<ButtonKey, ButtonStyle>(Variant::Custom(RECORD));
    assert_eq!((custom.variant, custom.fell_back), (Variant::Custom(RECORD), false), "a custom variant built for this key");
    let foreign = accepted_variant::<OtherKey, ButtonStyle>(Variant::Custom(RECORD));
    assert_eq!((foreign.variant, foreign.fell_back), (Variant::SECONDARY, true), "a custom variant built for another key");
    let unlisted = accepted_variant::<ButtonKey, ButtonStyle>(Variant::look(Emphasis::Soft, Tone::Good));
    assert_eq!((unlisted.variant, unlisted.fell_back), (Variant::SECONDARY, true), "an unlisted built-in variant");
}

#[derive(Clone, PartialEq)]
struct BareStyle;

// derive output
impl ComponentStyle for BareStyle {
    fn metrics(&self) -> Metrics {
        Metrics::new()
    }
}

#[trial]
fn a_component_without_listed_variants_still_accepts_the_custom_ones_built_for_its_key() {
    let custom = accepted_variant::<ButtonKey, BareStyle>(Variant::Custom(RECORD));
    assert_eq!((custom.variant, custom.fell_back), (Variant::Custom(RECORD), false));
    let other = accepted_variant::<ButtonKey, BareStyle>(Variant::PRIMARY);
    assert_eq!((other.variant, other.fell_back), (Variant::SECONDARY, false), "every other request is the default look");
}
