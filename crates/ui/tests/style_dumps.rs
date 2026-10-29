//! Style dumps: `style_for` of every built-in and mode it has, at the default size, contrast and
//! text size, as text under `tests/dumps/style/`. Rewrite with
//! `UPDATE_DUMPS=1 cargo test -p craft-ui dump`.
//!
//! The dump itself is `craft_ui::testing::style_dump::style_dump`.

use craft_theme::{Appearance, Contrast, Mode, Size, TextSize, Variants, builtins, resolve};
use craft_ui::testing::dump::check_dir;
use craft_ui::testing::style_dump::style_dump;
use craft_ui::theme::{FontFamilies, style_for};

#[test]
fn style_dumps() {
    let mut dumps = Vec::new();
    for theme in builtins() {
        let modes = match theme.variants {
            Variants::Both => &[Mode::Light, Mode::Dark][..],
            Variants::LightOnly => &[Mode::Light],
            Variants::DarkOnly => &[Mode::Dark],
        };
        for &mode in modes {
            let tokens = resolve(
                theme,
                Appearance { mode, contrast: Contrast::Standard, size: Size::default(), text_size: TextSize::default(), reduced_motion: false },
                None,
            )
            .tokens;
            let id = theme.id.as_str();
            let name = id.strip_prefix("craft.").unwrap_or(id);
            let mode_name = match mode {
                Mode::Light => "light",
                Mode::Dark => "dark",
            };
            dumps.push((format!("{name}-{mode_name}.txt"), style_dump(&style_for(&tokens, &FontFamilies::default()))));
        }
    }
    check_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/dumps/style"), &dumps);
}
