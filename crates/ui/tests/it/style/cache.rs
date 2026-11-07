//! The style cache, the env key and app style replacements.

use craft_host::target::Os;
use craft_i18n::Direction;
use craft_theme::{Contrast, ControlSize, Mode, Variant};
use craft_ui::runtime::env::{RootEnv, Scale};
use craft_ui::runtime::{InstallError, InstallSpec, install};
use craft_ui::style::{ComponentStyle, EnvKey, Metrics, StyleCx, StyleKey, StyleReplacements};
use craft_ui::testing::style::{root_token_entries, style_of, tokens, with_cx};
use craft_ui::testing::{Frames, trial};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FirstKey;
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SecondKey;
// derive output
impl StyleKey for FirstKey {}
// derive output
impl StyleKey for SecondKey {}

/// One style type shared by two components.
#[derive(Clone, PartialEq, Debug)]
struct SharedStyle {
    built_by: &'static str,
}

// derive output
impl ComponentStyle for SharedStyle {
    fn metrics(&self) -> Metrics {
        Metrics::new()
    }
}

#[trial]
fn two_components_sharing_a_style_type_get_their_own_style() {
    static BUILDS: AtomicUsize = AtomicUsize::new(0);
    fn first(_: &StyleCx<'_>, _: FirstKey) -> SharedStyle {
        BUILDS.fetch_add(1, Ordering::SeqCst);
        SharedStyle { built_by: "first" }
    }
    fn second(_: &StyleCx<'_>, _: SecondKey) -> SharedStyle {
        BUILDS.fetch_add(1, Ordering::SeqCst);
        SharedStyle { built_by: "second" }
    }
    let frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    with_cx(&frames.ctx, &tokens, Variant::SECONDARY, ControlSize::Md, |cx| {
        for _ in 0..3 {
            assert_eq!(style_of(&frames.ctx, cx, FirstKey, first).built_by, "first");
            assert_eq!(style_of(&frames.ctx, cx, SecondKey, second).built_by, "second");
        }
    });
    assert_eq!(BUILDS.load(Ordering::SeqCst), 2, "each key built once, every later read hit the cache");
}

#[trial]
fn a_style_is_rebuilt_only_when_an_input_of_its_key_changes() {
    static BUILDS: AtomicUsize = AtomicUsize::new(0);
    fn build(_: &StyleCx<'_>, _: FirstKey) -> SharedStyle {
        BUILDS.fetch_add(1, Ordering::SeqCst);
        SharedStyle { built_by: "first" }
    }
    let frames = Frames::new();
    let light = tokens(Mode::Light, Contrast::Standard);
    let dark = tokens(Mode::Dark, Contrast::Standard);
    let read = |tokens, variant, size| with_cx(&frames.ctx, tokens, variant, size, |cx| drop(style_of(&frames.ctx, cx, FirstKey, build)));
    read(&light, Variant::SECONDARY, ControlSize::Md);
    read(&light, Variant::SECONDARY, ControlSize::Md);
    assert_eq!(BUILDS.load(Ordering::SeqCst), 1, "same inputs hit");
    read(&light, Variant::SECONDARY, ControlSize::Sm);
    assert_eq!(BUILDS.load(Ordering::SeqCst), 2, "another size is another entry");
    read(&dark, Variant::SECONDARY, ControlSize::Md);
    assert_eq!(BUILDS.load(Ordering::SeqCst), 3, "another token set is another entry");
}

#[trial]
fn every_style_cx_input_changes_the_env_key() {
    let base = (7_u64, Direction::Ltr, Scale::new(1.0), Os::Linux, true, 0_u64);
    let key = |(tokens, direction, scale, os, active, values)| EnvKey::new(tokens, direction, scale, os, active, values);
    let reference = key(base);
    assert_eq!(reference, key(base));
    let changed = [
        key((8, base.1, base.2, base.3, base.4, base.5)),
        key((base.0, Direction::Rtl, base.2, base.3, base.4, base.5)),
        key((base.0, base.1, Scale::new(2.0), base.3, base.4, base.5)),
        key((base.0, base.1, base.2, Os::Mac, base.4, base.5)),
        key((base.0, base.1, base.2, base.3, false, base.5)),
        key((base.0, base.1, base.2, base.3, base.4, 1)),
    ];
    for (i, other) in changed.iter().enumerate() {
        assert_ne!(*other, reference, "input {i} left the key alone");
    }
}

#[trial]
fn an_app_replacement_wins_and_two_for_one_key_fail_the_install() {
    fn default(_: &StyleCx<'_>, _: FirstKey) -> SharedStyle {
        SharedStyle { built_by: "library" }
    }
    fn replacement(_: &StyleCx<'_>, _: FirstKey) -> SharedStyle {
        SharedStyle { built_by: "app" }
    }
    fn second(_: &StyleCx<'_>, _: SecondKey) -> SharedStyle {
        SharedStyle { built_by: "library" }
    }
    let frames = Frames::uninstalled();
    let mut looks = StyleReplacements::new();
    looks.push::<FirstKey, SharedStyle>(replacement);
    assert!(install(&frames.ctx, InstallSpec::new().looks(looks)).is_ok());
    let tokens = tokens(Mode::Light, Contrast::Standard);
    with_cx(&frames.ctx, &tokens, Variant::SECONDARY, ControlSize::Md, |cx| {
        assert_eq!(style_of(&frames.ctx, cx, FirstKey, default).built_by, "app");
        assert_eq!(style_of(&frames.ctx, cx, SecondKey, second).built_by, "library", "a key with no replacement keeps its own style");
    });

    let other = Frames::uninstalled();
    let mut twice = StyleReplacements::new();
    twice.push::<FirstKey, SharedStyle>(replacement).push::<FirstKey, SharedStyle>(replacement);
    assert!(matches!(install(&other.ctx, InstallSpec::new().looks(twice)), Err(InstallError::DuplicateStyle(_))));
}

/// The key of a token set is its content, so rebuilding an equal set adds no cache entries.
#[trial]
fn equal_token_sets_share_a_key_and_different_ones_do_not() {
    let light = tokens(Mode::Light, Contrast::Standard);
    assert_eq!(light.key(), tokens(Mode::Light, Contrast::Standard).key());
    assert_ne!(light.key(), tokens(Mode::Dark, Contrast::Standard).key());
    let mut values = craft_theme::GroupValues::new();
    values.set_color("clip", "audio", craft_theme::Rgba::rgb(200, 100, 50));
    assert_ne!(light.key(), light.with_groups(values).key(), "token group values are part of the set");
}

/// An app that builds a fresh `RootEnv` (and so a fresh sizes table) every frame leaves a bounded root-token cache.
#[trial]
fn root_tokens_are_cached_for_a_bounded_number_of_inputs() {
    let mut frames = Frames::new();
    for _ in 0..20 {
        frames.root(RootEnv::default());
        frames.idle(1, |_| {});
    }
    let kept = root_token_entries(&frames.ctx);
    assert!(kept > 0 && kept < 20, "{kept} entries after 20 frames with 20 distinct roots");
}
