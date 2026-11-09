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

