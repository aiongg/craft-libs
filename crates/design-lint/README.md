# craft-design-lint

Design-system lint for Craft apps. Two halves:

- **clippy**: `clippy.template.toml` lists the stock egui widgets and raw colour constructors apps must not call. `check-clippy` fails unless the app's `clippy.toml` carries every entry and no other `clippy.toml` (a package directory or a parent below the root) or `CLIPPY_CONF_DIR` makes clippy read a different file. `clippy --write` merges the template in.
- **syntax**: `check` parses the configured directories with `syn` (each file once) and reports literal sizes, named colour constants, style mutation and crate-wide clippy allows.

```
craft-design-lint check [--config design-lint.toml] [--summary] [--deny-unused] [--baseline FILE | --write-baseline FILE]
craft-design-lint clippy [--write] [--clippy-toml PATH]
craft-design-lint check-clippy [--clippy-toml PATH]
```

Exit code: 0 clean, 1 findings (or a rise over the baseline, or clippy drift or shadowing), 2 usage, configuration or I/O error.

Craft-libs gallery: `cargo run -p craft-design-lint -- check --config crates/ui-gallery/design-lint.toml --deny-unused --summary`.

## Config

```toml
[lint]
paths = ["src"]          # relative to the config; tests are skipped unless include-tests = true
deny-unused = true       # marker/unused and exemption/unused become errors

[[exempt]]               # path-wide allowance
path = "src/canvas"
rules = ["literal-size"] # ids or families; omit for every rule except parse
reason = "document-space drawing"
```

`parse` findings are never exempt. An exemption that matches no checked file or silences no finding is reported as `exemption/unused`. The report lists every exemption with its rules and reason.

## Rules

| Rule | Flags |
|---|---|
| `literal-size/vec2` | `vec2`, `Vec2::new/splat`, `Vec2 { x, y }` with a literal |
| `literal-size/add-space` | `.add_space(lit)` |
| `literal-size/corner-radius` | `CornerRadius::same/from`, struct form, `.corner_radius`, `theme::corner_radius`, the radius argument of `.rect`, `.rect_filled`, `.rect_stroke`, `Shape::rect_*`, `RectShape::new/filled/stroke` |
| `literal-size/stroke` | `Stroke::new`, `PathStroke::new`, struct form, `theme::stroke` (width); `(width, colour)` tuples passed to `.stroke`, `.rect*`, `.line_segment`, `.circle*`, `Stroke::from`, or `.into()` |
| `literal-size/margin` | `Margin::same/symmetric/from`, struct form, `.inner_margin`, `.outer_margin`, `theme::margin`, `theme::margin_x` |
| `literal-size/font-id` | `FontId::proportional/monospace/new` (size); `.size(lit)` on a `RichText::..` chain |
| `raw-colour/const` | `Color32::WHITE` and the other named constants; `Rgba::{WHITE, BLACK, RED, GREEN, BLUE}` |
| `style-mutation` | `style_mut`, `spacing_mut`, `visuals_mut`, `set_style`, `set_visuals`, `set_zoom_factor`, `override_*` |
| `tooltip/stock` | `on_hover_text`, `on_disabled_hover_text`, `on_hover_ui`, `on_disabled_hover_ui`, `on_hover_text_at_pointer`, `on_hover_ui_at_pointer`, `show_tooltip_text`, `show_tooltip_ui`, `egui::Tooltip::for_enabled/for_disabled/for_widget/always_open`; the message names the `craft_ui::controls::TooltipExt` method that replaces the call. craft-ui's own tooltip module is not in the paths an app lints |
| `clippy/inner-allow` | `#![allow(clippy::disallowed_*)]` or `#![expect(..)]` in any scope; list the file in an `[[exempt]]` to accept it. Outer `#[allow]`/`#[expect]` of those lints are counted in the summary line. |

Names are resolved through the file's `use` items, per module scope: grouped imports, renames (`use egui::Margin as M`), module aliases (`use craft_ui::theme as t`) and glob imports (`use craft_ui::theme::*`) all reach the rule table, so `margin(4.0)` is flagged after `use craft_ui::theme::margin`. A bare name that a fn in the same module defines is not extended through globs. Imports in a module do not apply in sibling modules.

A literal is a number, optionally negated, parenthesised, cast or in arithmetic; zero is never flagged. A name is flagged when a `const` or `static` in the same file with that name has such a value. Consts are keyed by module and impl scope: a bare name takes the innermost scope that defines it, `Self::X`, `self::X`, `super::X`, `crate::X`, `Type::X` and `module::X` (last two segments) the scope they name, and a name with several candidates is flagged when any is non-zero.

## Markers

| Form | Covers |
|---|---|
| `// design: allow(reason)` at the end of a line | findings starting on that line |
| same, alone on a line | the next statement, match arm, field initialiser or item. For a `fn`, `impl`, `trait`, `mod`, `if`, `for`, `while`, `loop`, `match` or block it covers the header up to the opening brace, not the body |
| `//! design: allow(reason)` | the whole file |
| `allow[rule, family/..](reason)` | the same range, only for the listed rules |

`parse` and `clippy/inner-allow` findings cannot be covered by a marker. Markers with a short reason are `marker/empty`; unknown rules or an unclosed list are `marker/invalid`; a marker that covers nothing is `marker/unused`.

## Baseline

An existing code base can gate on new findings only:

```
craft-design-lint check --write-baseline design-lint-baseline.toml
craft-design-lint check --baseline design-lint-baseline.toml
```

The baseline stores error-finding counts per file and rule. The check fails when a count rises (a new file or rule counts from zero) and prints the findings of the rising files and rules. A file that does not parse always fails, and no baseline is written while one exists.
