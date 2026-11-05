# 2. Installation

## 2.1 Run the component gallery

The gallery holds every theme. Themes switch at run time. It needs Rust 1.95 or later.

```
git switch design-system
cargo run --release -p craft-ui-gallery
```

- Update with `git pull` and rebuild.
- Candidate fonts are optional: check out the `craft-fonts` repository at branch `design-system-fonts` and set `CRAFT_FONTS_DIR=<absolute path>` before `cargo run`.
- Web build: `cd crates/ui-gallery && trunk serve --release true index.html`. It needs `trunk` and the `wasm32` target.
- Contents: an overview with one tile per component of a layer, a tab per component with all its stories, a variants view (the selected story once per theme, mode, contrast or size), the appearance bar and its More popover.

## 2.2 Depend on `craft` from an app

An app has one dependency, one pin and one import root.

```toml
[dependencies]
craft = { git = "https://github.com/storytold/craft-libs", rev = "...", features = [
    "ui", "color", "layers", "theme-store", "platform-job", "platform-clipboard",
] }
```

```rust
use craft::prelude::*;
use craft::color::ui::{ColorPicker, SwatchPalette};
```

Feature and type names in the examples show the path convention, not settled APIs. Section [3.2](3-architecture.md#32-the-craft-facade) lists the facade's rules and features. An app's `xtask` checks that its lockfile uses the library's egui revision ([3.5](3-architecture.md#35-one-egui-revision)).
