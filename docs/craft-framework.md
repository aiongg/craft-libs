# Craft framework

As of 31 Oct 2026

The Craft framework is the shared library layer every Craft app builds on. It holds the design system (tokens, themes, components), the shells and canvas core, the host convention, and shared features an app gets ready-made: colour, files, preferences, dialogs and more. This document describes the design, the proposed crate layout, how the apps migrate, and how the work lands in `craft-libs`.

## Contents

1. [What it is and why](#what-it-is-and-why)
2. [Try it](#try-it)
3. [Design principles](#design-principles)
4. [Architecture](#architecture)
5. [Crate and module layout](#crate-and-module-layout)
6. [Appearance](#appearance)
7. [Colour](#colour)
8. [Shape, size and type](#shape-size-and-type)
9. [Components](#components)
10. [Translation and accessibility](#translation-and-accessibility)
11. [Tests and gates](#tests-and-gates)
12. [Migration strategy](#migration-strategy)
13. [Landing strategy](#landing-strategy)
14. [Status and open questions](#status-and-open-questions)

## What it is and why

One library, `craft-libs`, supplies everything the Craft apps have in common:

- **Tokens and themes.** Colours, sizes, type and shape as data, with built-in themes, light and dark modes, and high contrast.
- **Components.** Primitives, controls, composites and patterns that read those tokens.
- **Shells.** The window skeleton, docking, title bar, panel rails and options bar.
- **Shared features.** Complete features an app gets in one call, for example `ColorPickerDialog::new()`: the picker body, gamut and web-colour chips, OK and Cancel, remembered mode, and colour management through `craft-cms`.

The apps differ in what they edit. The pieces around the editing are the same: a colour picker, docking, preferences, dialogs, file pickers, shortcut editing, font selection. Today each app writes its own version. Copies drift apart in look, behaviour, accessibility and translation, and a fix in one does not reach the others.

The library takes the opposite approach: build each feature once, in one place, and use it the same way everywhere. A feature that only one app needs today is still built in the library. Moving it later, after a second app copies it, costs more than building it once.

An app depends on one crate, `craft`, pins one revision, and imports `craft::<family>::...`. The library owns the one egui revision, so the apps cannot end up with two incompatible copies of egui.

Terms used in this document:

| Term | Meaning |
| --- | --- |
| Design system | The tokens, themes and component layers |
| Craft framework | The whole: design system, shells, canvas core, host convention and shared features |
| Family | A set of features a developer looks for together (colour, layers, timeline, ...). Each family is a crate |
| Kernel | `craft-ui`: the design-system layers every family uses, with nothing that names a document concept |

## Try it

The component gallery runs from the `design-system` branch of this repository. One build holds every theme, and they switch at run time. Rust 1.95+.

```
git switch design-system
cargo run --release -p craft-ui-gallery
```

- Updates: `git pull`, rebuild
- Candidate fonts (optional): check out the craft-fonts repository at branch `design-system-fonts`, set `CRAFT_FONTS_DIR=<absolute path>` before `cargo run`
- Web: `cd crates/ui-gallery && trunk serve --release true index.html` (needs trunk and the wasm32 target)
- Gallery contents: Overview (one tile per component of a level), a tab per component with all its stories, Variants (the selected story once per theme, mode, contrast or size), the appearance bar and its More popover

## Design principles

### What each app keeps, what is shared

Each app keeps the layout, placement, conventions and density of the industry tool it is modelled on. What is shared is the code for the building blocks and how they behave.

| Per app | Shared |
| --- | --- |
| Layout and placement follow each app's industry original | Components (buttons, fields, sliders, swatches, colour wheel, bezier handles, dialogs, menus) |
| Conventions (menu structure, shortcuts, tool behaviour) | Family patterns (dock and tool rail for creative apps, ribbon for office-style apps) |
| Default density (Compact for dense editors, Comfortable for roomier apps such as PDFCraft) | Behaviour (focus and Tab order, Esc cancels, one undo step per drag, number-field scrubbing) |
| Default theme and mode (dark for creative apps, light for office-style apps) | Themes, high contrast, translation, screen-reader support |
| Colour overrides for the app's own look (for example SoundCraft's red playhead) | The token model that carries those overrides |

A theme never changes the layout. Different arrangements are workspaces, and workspaces are separate from themes. An app has one layout in every theme and size: Studio, for example, is Pro's layout with Studio colours and the Comfortable size.

### How an app tunes a component without writing its own

Use the first option that works.

1. **Density.** Compact or Comfortable, per app.
2. **Size tokens.** The app ships a `sizes.json` that overrides base sizes (for example `size.control.md`). A base override carries to every size derived from it. To change one component only, the app pins that component's token. Overrides stay within validator limits.
3. **Component tokens.** Named sizes and colours per component. One value changes it app-wide.
4. **Options on the component.** Builder options cover structural differences. A new need gets a new option, not a new component.
5. **Per-app colour overrides.** A theme file can carry a section per app with colour overrides. The values are references to theme roles or literal colours, so themes, dark mode and high contrast keep working.
6. **One-off app components.** A last resort, built from shared pieces. They are allowed and not policed.

Apps may set corner radii. A theme that has its own shape (Classic, Breeze, Adwaita, Soft) wins over the app's radii. The edge style (flat or bevel) stays with the size set and the theme.

### Principles

**Apps ask for meaning.** An app asks for a raised surface, a primary button or muted text. It never names a colour or a pixel size. A lint enforces this in app UI code (see [Tests and gates](#tests-and-gates)).

**Sizes are runtime tokens.** Every size and spacing value is a token resolved at run time from the Size preset (Compact or Comfortable). Layout numbers inside components derive from base tokens. Only drawing details (stroke widths, chevron geometry, dash lengths) stay constants. Apps can override some tokens. Themes set no sizes, and the token model leaves room for a theme layer later.

**Themes change colour and, for four themes, shape.** A theme supplies seeds and pins for colour. A theme may also define its own shape (corner radii, edge style). Classic, Breeze, Adwaita and Soft do.

**One type scale, named by meaning.** Text uses semantic roles. App text that falls between two roles maps to the nearest role. A real new need gets a new, named role. There is no role for a single use.

| Role | Use | Compact (size / line, pt) | Comfortable |
| --- | --- | --- | --- |
| Caption | Small print, badges | 10.5 / 14 | 11.5 / 16 |
| Label | Control labels | 11.5 / 16 | 12.5 / 18 |
| Body | Body text | 12 / 16 | 13 / 18 |
| Body strong | Emphasised body text (semibold) | 12 / 16 | 13 / 18 |
| Title | Panel and dialog titles | 13 / 18 | 14 / 20 |
| Heading | Section headings | 15 / 20 | 17 / 24 |
| Body large | Large regular text, such as a prominent text field | 15 / 20 | 16 / 22 |
| Mono | Code, numbers in tables | 11.5 / 16 | 12.5 / 18 |

A separate Text size setting (0.9x, 1x, 1.15x, 1.3x) scales the roles only.

**Colour is one base family.** Seeds (neutral, accent and four status hues) produce ramps. Semantic tokens map to ramp steps. Any token can be pinned. See [Colour](#colour).

**Standard themes follow their design; high contrast meets the standard.** In Standard contrast, derived colours meet WCAG 2.2 AA. Pinned colours that fall short are kept and reported (in `craft-theme check`, the gallery token browser and a "Low contrast" label in the theme picker), never rewritten. High contrast always enforces the targets, pinned or not, so users who need it have it.

**One undo step per gesture, and Esc restores.** Value controls report changes as edit events: begin, update, commit, cancel, set. The app owns the value from before the gesture. Esc during a drag cancels it and restores that value. A drag or a typing run commits as one step.

**Keyboard, focus and screen readers are required.** Every control is operable from the keyboard. Keyboard focus is always visible. Every widget exposes AccessKit roles, names and values. Pointer and keyboard behaviour follow the suite's own apps first. WAI-ARIA authoring patterns are a reference where no app has a convention. Beyond these requirements, accessibility is a direction and one consideration among others.

**Translation never blocks.** Library words live in the shared `craft-i18n` catalog. A missing string shows in English. Labels the host app sets win over library words. No test requires a translation to exist.

**The egui builder idiom.** `Type::new(required).option(..).show(ui)`. Panels take `&mut Model` in `new`. Dialogs return an `Outcome<T>`.

**Behaviour follows the industry original.** Where an app's tool has an industry original, the library follows it. Otherwise it follows the app's existing behaviour. Where an app draws one kind of component several ways, the library has one: the app's most common or best version, with the variants it replaces listed.

**Colour management goes through the CMS crate.** Anything that converts, previews or proofs colour (CMYK, gamut checks, display transforms) uses `craft-cms`. A missing capability is added to `craft-cms`, not worked around in a UI crate.

## Architecture

### Layers

Components build up in six layers. Each layer reads the one below.

| Layer | Holds |
| --- | --- |
| Tokens | Scales, ramps, seeds, derivation, contrast, size presets, component tokens, theme files (`craft-theme`, no egui) |
| Primitives | Surface, state layer, focus ring, divider, stack, interactive area, text, icon, handle |
| Controls | Buttons, fields, sliders, checkboxes, selects, swatches, tooltips |
| Composites | Controls with labels and text: field rows, property rows, headers, lists, tabs, toasts, menus |
| Patterns | Whole regions of a window: panel, dialog, inspector, dock area, options bar, status bar, title bar |
| Shells | The window skeleton: title bar, rails, body, bars; presets for kinds of app |

Supporting parts:

| Part | Crate | Holds |
| --- | --- | --- |
| Store | `craft-theme-store` | Theme files, machine-local library, suite `selection.json`, lock and revision protocol, watcher, client |
| Bridge | `craft_ui::theme` | Tokens to egui styles; stock egui widgets themed too |
| Behaviour | `craft_ui::behavior`, `craft_ui::edit` | Focus, keys, typing and IME, list navigation; the edit lifecycle |
| Tools | `craft-ui-gallery`, `craft-design-lint`, `craft-plate-test` | Gallery app, lint for app code, print test scenes |

### Crate tiers

| Tier | Crates | Rule |
| --- | --- | --- |
| 0. Engines and services | `craft-theme`, `craft-i18n`, `craft-settings`, `craft-cms`, `craft-pdf`, `craft-media`, `craft-platform`, `craft-host`, ... | No egui, except `ecolor` for colour conversion |
| 1. Kernel | `craft-ui` | Tokens bridge, primitives, controls, generic composites, patterns, shells, dialog framework, docking, canvas plumbing, preferences framework, story and testing. Nothing that names a document concept. Never depends on a family crate |
| 2. Families | `craft-color`, `craft-draw`, `craft-layers`, `craft-timeline`, ... | Core at the crate root, egui UI in `ui` behind the `ui` feature. A family crate may be UI-only. No family depends on another family's `ui`; a widget two families share moves down to `craft-ui` |
| 3. Facade | `craft` | Re-exports every crate and forwards features |

A family is a set of features a developer looks for together. The brush cursor, brush settings and brush presets live in one crate, not three.

### The `craft` facade

An app has one dependency, one pin and one import root.

```toml
[dependencies]
craft = { git = "https://github.com/storytold/craft-libs", rev = "...", features = [
    "ui", "color", "layers", "theme-store", "platform-job", "platform-clipboard",
] }
```

```rust
use craft::prelude::*;
use craft::color::ui::ColorPickerDialog;
use craft::layers::ui::LayersPanel;
use craft::theme_store::ui::AppearancePage;
```

- Every public UI type is re-exported at `<crate>::ui`; every core type at the crate root. The facade gives `craft::color::ui::ColorPickerDialog` and `craft::color::Palette`.
- A feature can move between crates without touching any app, as long as `craft::<family>` stays.
- `craft::prelude` holds `AppInfo`, `Host`, `Section`, `Tokens`, `EditEvent`, `DialogStack` and `Outcome`. There are no other preludes.
- `craft::story::all()` and `craft::words::all()` collect the stories and word tables of every enabled crate.

### Feature names

| Feature | Meaning |
| --- | --- |
| `ui` | The family's egui side, on `craft-ui`. The only UI feature name in every crate |
| `screen-testing` | Stories and snapshot tests; implies `ui` |
| `egui` | Reserved for "convert my types to egui's, without `craft-ui`" where that is the whole feature (the model is `craft-theme`'s `ecolor`) |
| `serde`, `collab`, per-service features | Keep their names. `craft-platform` has one feature per service (`job`, `clipboard`, ...) |

The facade's `ui` turns on `?/ui` for every enabled family. Native-only modules are gated with `#[cfg(not(target_arch = "wasm32"))]`. A crate that cannot build for wasm says so in its first doc line, and the facade gates it.

### The host convention: services in, edits out

`craft-host` is a small egui-free crate. The app builds a `Host` once and installs it on the egui context. Every library feature reads it with `Host::of(ui)`. No feature takes an i18n or settings parameter.

```rust
fn setup(ctx: &egui::Context, app: &App) {
    Host::builder(app.info())
        .i18n(app.i18n.clone())
        .settings(app.settings.clone())
        .commands(app.keymap.clone())
        .jobs(Arc::new(craft::host::ThreadJobs))
        .install(ctx);
}
```

| Rule | Detail |
| --- | --- |
| Services in | Identity, language, settings sections, commands and jobs today; files and a clock are planned. A service is a trait; the app or a library crate implements it |
| Document data in | Each feature defines one small trait (`TimelineModel`, `ColorTarget`, `LayerTree`) that the app implements on its document view. Features never own document state |
| Edits out | Features return edit events (`Begin`, `Update`, `Commit`, `Cancel`, `Set`). The app applies them to its own undo stack |
| Dialogs | A feature dialog is a `Dialog` value pushed on the kernel's `DialogStack`. Its result is an `Outcome<T>` the app reads on the next frame. No feature opens its own window |
| Settings keys | Each feature exports a `serde` struct with a `const KEY`. The app embeds it in its settings file through `craft-settings` |
| Words | Library words are rows in `craft-i18n`'s shared catalog. Features look them up through the host |
| Commands | A feature exports its command catalog. The app registers it with `craft-keymap`. Automation dispatches through the host's commands |
| Files | Features ask the host for pick, read and write. Native crates implement the service; apps on the web implement it over the browser |

What an app writes: one dependency, one `Host`, one trait implementation per document feature, and the code that applies edits. The sketch below shows the target API; `ColorPickerDialog` lands in step 4 and the typed `take` in step 2.

```rust
fn frame(ui: &mut egui::Ui, doc: &mut Document, dialogs: &mut DialogStack) {
    if ui.button("Fill colour").clicked() {
        dialogs.push(ColorPickerDialog::new(doc.fill));
    }
    if let Some(Outcome::Ok(color)) = dialogs.take::<ColorPickerDialog>() {
        doc.apply(Edit::set(FILL, color));
    }
    LayersPanel::new(&mut doc.layers).show(ui);
}
```

### One egui revision

`craft-libs` pins one revision of the egui fork. Each app's `xtask` checks that its lockfile uses the same revision and fails with a clear message if it does not. Two revisions in one build would give two incompatible `Tokens` and `Ui` types.

## Crate and module layout

The layout below is the proposed final shape. Crates are alphabetical. Modules are alphabetical inside each crate. The State column says whether a module exists in the repository today or is planned, and in which migration step (see [Migration strategy](#migration-strategy)). A module marked "exists" may gain features in later steps; the note says so.

Conventions:

- Crates are `craft-<family>`, one word where possible. No `-ui` or `-core` suffix.
- Modules are nouns, one level below the crate root. `ui` is the only module that nests a second level (`craft_pages::ui::table`).
- Types carry their full name: `ColorPickerDialog`, `LayersPanel`, `TimelineRuler`, never `color::PickerDialog`.
- In a family crate, a module `ui::x` is the egui side and a bare `x` is core.

Twenty crates are new relative to upstream. Ten exist on the `design-system` branch: `craft`, `craft-color`, `craft-design-lint`, `craft-draw`, `craft-host`, `craft-imaging`, `craft-theme`, `craft-theme-store`, `craft-ui-gallery`, `craft-ui-macros`. Ten are planned: `craft-audio`, `craft-automation`, `craft-browse`, `craft-code`, `craft-gpu`, `craft-layers`, `craft-pages`, `craft-review`, `craft-table`, `craft-timeline`.

### craft

The facade. New on the branch (step 1).

| Module | Holds | State |
| --- | --- | --- |
| `egui_fork` | Lockfile check: the egui revision matches the library's | exists |
| `prelude` | `AppInfo`, `Host`, `Section`, `Tokens`, `EditEvent`, `DialogStack`, `Outcome` | exists |
| `story`, `words` | Stories and word tables of every enabled crate | exists |
| crate re-exports | One module per crate, features forwarded | exists |

### craft-ai-core

Shared data model for AI connections, roles, requests and jobs. No egui.

| Module | Holds | State |
| --- | --- | --- |
| `capability` | Modalities and what can be done to data | exists |
| `catalog`, `model`, `picks` | Models, what each can do, which to suggest for a job | exists |
| `connection`, `consent`, `endpoint`, `secret` | Where work is sent, what leaves the machine and whether the user agreed, secrets that cannot print | exists |
| `conformance`, `mock` | Checks every provider must pass; a scripted provider for tests | exists |
| `draft`, `store` | The "add a connection" form model; the shared versioned store | exists |
| `error`, `ids`, `job`, `history` | Error type, validated ids, job progress, request history | exists |
| `models` | Downloadable model manager | planned, step 4 |
| `payload`, `provider`, `request`, `role`, `spec` | The provider trait, requests and payloads, roles, declarative routes | exists |
| `ui::models`, `ui::settings` | Model download UI; AI settings page | planned, step 4 |

### craft-atomic

Crash-safe atomic file writing. One module at the root; exists.

### craft-audio

Audio device, meter ballistics and peaks, plus mixer UI. New crate.

| Module | Holds | State |
| --- | --- | --- |
| `db` | dB value text | planned, step 8 |
| `meter` | Meter ballistics | planned, step 8 |
| `output` | Output stream and device setup | planned, step 8 |
| `peaks` | Waveform peaks | planned, step 8 |
| `ui::controls` | Fader and pan knob | planned, step 8 |
| `ui::meter` | Level meter | planned, step 8 |
| `ui::mixer` | Channel strip mixer | planned, step 8 |
| `ui::panner` | Surround panner | planned, step 8 |
| `ui::piano_roll` | Piano-roll note editor | planned, step 8 |
| `ui::response` | Frequency response plot and editor | planned, step 8 |
| `ui::waveform` | Waveform view | planned, step 8 |

### craft-automation

Control channel, screenshot queue, profiler and recorded actions for tests and scripting. Native only. New crate.

| Module | Holds | State |
| --- | --- | --- |
| `actions`, `ui::actions` | Recorded actions and the actions list | planned, step 6 |
| `control` | Automation control channel | planned, step 3 |
| `profiler` | Frame profiler | planned, step 3 |
| `screenshot` | Screenshot request queue | planned, step 3 |
| `trace` | Frame timing trace | planned, step 3 |

### craft-browse

Thumbnails, media bin, tile grid, places and in-app file browsing. New crate.

| Module | Holds | State |
| --- | --- | --- |
| `browser`, `ui::browser` | In-app file browser | planned, step 4 |
| `places`, `ui::places` | Folder places tree | planned, step 6 |
| `thumbs` | Thumbnail loader and cache | planned, step 6 |
| `ui::bin` | Media bin browser | planned, step 6 |
| `ui::compare` | Image compare tile | planned, step 6 |
| `ui::grid` | Thumbnail tile grid | planned, step 6 |
| `ui::relink` | Linked files and relink dialog | planned, step 6 |

### craft-cms

Colour management: ICC v2 and v4 profiles, transforms, rendering intents. No egui.

| Module | Holds | State |
| --- | --- | --- |
| `builtin`, `synth` | Built-in and synthetic CMYK profiles | exists |
| `clut`, `curve`, `lut3d`, `lutfile` | Lookup tables, tone curves, display LUTs, creative LUT files | exists |
| `convert` | CMYK models and gamut conversions the colour dialogs need | planned, step 4 |
| `gamut` | Gamut checks against a proof profile | exists |
| `math` | Matrices, XYZ and Lab, chromatic adaptation | exists |
| `pipeline`, `transform` | Evaluation pipelines; transforms with intents and black point compensation | exists |
| `profile`, `write` | ICC profile model and reader; v4.3 writer | exists |

### craft-code

Code editor field and console. UI-only family. New crate.

| Module | Holds | State |
| --- | --- | --- |
| `ui::console` | Script console panel | planned, step 8 |
| `ui::editor` | Syntax-coloured code field (syntax colours as a `data.code.*` token family) | planned, step 8 |

### craft-color

Colour for apps: picker, wells, swatch tables, ramps, hex field. New on the branch.

| Module | Holds | State |
| --- | --- | --- |
| `display` | Display transform on `craft-cms` | planned, step 4 |
| `gray` | Grey level of a colour | exists |
| `harmony` | Colour harmony and saved themes | planned, step 4 |
| `hex` | Hex text for colours | exists |
| `mode` | What a colour area and its strip show | exists |
| `palette` | Palette data | planned, step 4 |
| `ui::area`, `ui::strip` | 2D colour area and its strip | exists |
| `ui::choice` | Colour choice: named colours plus Custom | planned, step 4 |
| `ui::disc`, `ui::wheel`, `ui::ramp` | Colour disc, wheel and ramp | exists |
| `ui::droppers`, `ui::eyedropper`, `ui::samplers`, `ui::screen_picker` | Dialog droppers, eyedropper tool, sample points, screen picker | planned, step 4 |
| `ui::harmony` | Harmony wheel | planned, step 4 |
| `ui::hex`, `ui::readout` | Hex field and channel readout | exists |
| `ui::lut_picker` | LUT picker | planned, step 4 |
| `ui::paint_chip`, `ui::paints` | Paint chip and paint list | planned, step 4 |
| `ui::pair` | Foreground and background swatch pair | exists |
| `ui::palette` | Swatch palette | exists |
| `ui::panel` | Colour panel modes and menus | planned, step 4 |
| `ui::picker`, `ui::popover` | The colour picker and its popover | exists |
| `ui::picker_dialog` | `ColorPickerDialog`: the complete picker dialog | planned, step 4 |
| `ui::profile_prompts` | Profile mismatch prompts | planned, step 4 |
| `ui::swatches` | Swatch panel | planned, step 4 |
| `web` | Web-safe colours | exists |

### craft-crw

Clean-room decoder for one camera raw format. `tables` holds frozen entropy-code data. Exists; no change.

### craft-design-lint

Lint for app UI code: the clippy template that blocks stock egui widgets and raw colours, and the `syn` checker for literal sizes. New on the branch.

| Module | Holds | State |
| --- | --- | --- |
| `baseline` | Baseline files for existing findings | exists |
| `clippy` | Check that an app's `clippy.toml` carries the template | exists |
| `config` | Lint configuration | exists |
| `markers` | `// design: allow(reason)` markers | exists |
| `report` | Findings output | exists |
| `rules` | Rule implementations | exists |

### craft-dialogs

Native file and message dialogs that never block the event loop.

| Module | Holds | State |
| --- | --- | --- |
| `filter` | File-type filters matching any letter case | exists |
| `macos` | macOS presentation | exists |
| `native` | Native dialogs on a worker thread | exists |
| `save` | Save and Save As flow with guards | planned, step 5 |
| `service` | File dialog service behind the host's files | planned, step 4 |
| `ui::file_list`, `ui::path_field` | File list picker; path field with Browse | planned, step 4 |

### craft-docpath

Decides whether a stored path or URL refers to a local file. Exists.

| Module | Holds | State |
| --- | --- | --- |
| `links` | Linked files, for relink | planned, step 6 |
| `names` | Safe file names | planned, step 5 |

### craft-draw

Vector and shape tools, pen, gradient, crop and free transform. UI-only family. New on the branch (step 1).

| Module | Holds | State |
| --- | --- | --- |
| `ui::artboard` | Artboard tool | planned, step 7 |
| `ui::brush`, `ui::brush_presets`, `ui::brush_settings` | Brush cursor and resize; brush preset cards; brush settings panel | planned, step 7 |
| `ui::connector` | Connector curves and handles | planned, step 7 |
| `ui::corner` | Corner radius tool | planned, step 7 |
| `ui::crop` | Crop frame and crop tool | exists; tool host wiring planned, step 7 |
| `ui::gradient` | Gradient swatch, editor, palette and on-canvas gradient line | exists |
| `ui::measure` | Measure overlay | planned, step 7 |
| `ui::pattern`, `ui::picture_fill` | Pattern fill picker; picture fill options | planned, step 7 |
| `ui::pen` | Pen draft and bezier paths | exists; Curvature mode, hidden anchors and hit testing planned, step 7 |
| `ui::shape`, `ui::shape_drag` | Shape properties panel; shape drag tool | planned, step 7 |
| `ui::slice` | Slice tool | planned, step 7 |
| `ui::stroke` | Stroke options editor | planned, step 7 |
| `ui::symmetry` | Paint symmetry | planned, step 7 |
| `ui::transform` | Free transform box and transform fields | exists; tool-host wiring planned, step 7 |

### craft-font-catalog

Installed fonts: discovery on every platform, family and style lookup, shared loading. Exists.

| Module | Holds | State |
| --- | --- | --- |
| `catalog` | Every installed face, found by any of its names | exists |
| `env`, `fontconfig`, `registry` | Font folders, fontconfig folders, registry-listed faces | exists |
| `scan` | Face records from font files | exists |
| `suitcase` | Classic suitcase font files | exists |
| `system` | The platform's font service | exists |

### craft-font-manager

Search and filters over font indexes, installed fonts by family, and the font manager panel.

| Module | Holds | State |
| --- | --- | --- |
| `axes` | Variable font axes | planned, step 4 |
| `filter` | Search by family, category, language, script | exists |
| `glyphs` | Glyphs panel | planned, step 4 |
| `index` | The font indexes | exists |
| `installed` | Installed fonts grouped by family | exists |
| `missing` | Document fonts and missing-fonts dialog | planned, step 4 |
| `panel`, `ui` | The egui `FontsPanel` and its story | exists |
| `picker` | Font picker: families, filters, previews, stars, recents, similar fonts (feature `font-picker`) | planned, step 4 |
| `store` | What the panel needs from the font store | exists |

### craft-font-store

The font store: indexes, exact lookup, verified installs into the suite's font folder. Exists.

| Module | Holds | State |
| --- | --- | --- |
| `http` | The one HTTPS client | exists |
| `index`, `source`, `sources` | Font indexes, one source's file and URLs, built-in sources | exists |
| `pin` | The index commit and hash this build trusts | exists |
| `store` | Store folder, consent, installs, manifest, removal | exists |
| `verify` | Checks a file against its index entry | exists |

### craft-fonts-build

Build-script support that embeds optional faces from an app's `build.rs`. Exists.

| Module | Holds | State |
| --- | --- | --- |
| `cmap`, `coverage` | Which characters a face maps | exists |
| `embed`, `manifest` | Face selection and the generated table | exists |
| `sha256` | Hash for served file names | exists |
| `subset` | Cutting a face to the characters the catalogs use | exists |

### craft-gpu

GPU device, diagnostics and memory budget, plus the GPU canvas surface. New crate; native.

| Module | Holds | State |
| --- | --- | --- |
| `budget` | Memory budget and probes | planned, step 8 |
| `canvas` | GPU canvas display surface | planned, step 8 |
| `device` | Device setup and diagnostics | planned, step 8 |
| `health` | Device loss, fallback and upload guard | planned, step 8 |
| `vector` | Vector canvas renderer | planned, step 8 |

### craft-host

The `Host` bundle and service traits. Egui-free (egui only under `ui`, for `install`). New on the branch (step 1).

| Module | Holds | State |
| --- | --- | --- |
| root | `Host`, `HostBuilder`, `Host::of`, `install` | exists |
| `services` | `AppInfo`, `SectionStore`, `Section`, `Commands`, `Jobs`, `ThreadJobs`, `InlineJobs` and defaults | exists |
| `files`, `clock` | File and clock services | planned, steps 3 and 4 |

### craft-i18n

Translations: embedded catalogs, lookup with fallback, plurals, numbers for a region, language names, locale, search folding, collation. No egui.

| Module | Holds | State |
| --- | --- | --- |
| `catalog` | Reading a catalog (the shared `common.tsv` holds library words) | exists |
| `collate` | Locale sorting for lists people see | exists |
| `fold` | Case- and accent-insensitive search matching | exists |
| `format` | Numbers, percentages and counts for a region | exists; dates and relative times planned, step 5 |
| `number` | Reading numbers people type | exists |
| `region`, `scripts` | Formatting region; scripts, direction and Han forms of a language | exists |
| `size` | Sizes and measures with a unit | exists |

### craft-imaging

Raster image editing. New on the branch (step 1). Core at the root, widgets in `ui`.

| Module | Holds | State |
| --- | --- | --- |
| `layer_style` | Layer style blending model | planned, step 7 |
| `preview` | Live adjustment preview with damage tracking | planned, step 7 |
| `selection` | Selection model | planned, step 7 |
| `trace` | Mask outline tracing | planned, step 7 |
| `ui::adjust` | Levels editor and the colour adjustment editors | exists (Levels); the other editors planned, step 7 |
| `ui::blend_keys` | Blend-mode key picker | planned, step 7 |
| `ui::blur_pins` | Blur pins on the canvas | planned, step 7 |
| `ui::channels` | Channels panel | planned, step 6 |
| `ui::fill` | Fill-from-surroundings workspace | planned, step 7 |
| `ui::filter_gallery` | Filter gallery workspace | planned, step 7 |
| `ui::layer_style` | Layer style page and effect list | planned, step 7 |
| `ui::liquify` | Push-and-warp (liquify) workspace | planned, step 7 |
| `ui::mask_view` | Mask and selection overlay views | planned, step 7 |
| `ui::scopes` | Scope panel: histogram, vectorscope, clipping | planned, step 6 |
| `ui::select`, `ui::select_mask`, `ui::selection` | Outline selection tools; select-and-mask workspace; ants, matte and quick-mask overlay | planned, step 7 |
| `ui::transform_preview` | Live transform preview | planned, step 7 |
| `ui::vanishing_point`, `ui::wide_angle` | Perspective plane and wide-angle correction workspaces | planned, step 7 |
| `ui::warp` | Warp grid; mesh, pin and perspective warp | exists (grid); the modes planned, step 7 |

### craft-jpeg

Clean-room decoder for 12-bit and arithmetic-coded JPEG. Exists.

### craft-keymap

Chords, keymaps, conflicts, overrides and shortcut-set import; the Keyboard Shortcuts dialog under `ui`.

| Module | Holds | State |
| --- | --- | --- |
| `chord` | Keys, modifiers, chords and text forms | exists |
| `command`, `commands` | Commands with parameters; command words for the host | exists |
| `editor` | Shortcut editor model with undo, recorder, conflicts | exists |
| `egui` | Key conversion and dispatch | exists |
| `import` | Import of another application's shortcut set | exists |
| `keymap` | Catalog with defaults, overrides, dispatch | exists |
| `layout` | Chord meaning on the active keyboard layout | exists |
| `profiles` | Named override sets | exists |
| `reserved` | Chords the OS takes first | exists |
| `scope` | Where a binding works | exists |
| `ui` | Shortcuts dialog | exists; tabs, set row and conflict flow planned, step 3 |
| `ui::import_offer` | First-launch import offer | planned, step 3 |
| `ui::router` | Focus gate and key owners | planned, step 3 |

### craft-layers

Layers panel and its parts. UI-only family. New crate.

| Module | Holds | State |
| --- | --- | --- |
| `ui::filter` | Layer filter bar | planned, step 6 |
| `ui::options` | Layers panel options dialog | planned, step 6 |
| `ui::panel` | `LayersPanel` over a `LayerTree` trait | planned, step 6 |
| `ui::rows` | Blend, opacity and lock rows | planned, step 6 |
| `ui::thumbs` | Layer and mask thumbnails | planned, step 6 |

### craft-media

Bit-level I/O and clean-room MP4/MOV and Matroska/WebM demuxers and muxers. Exists.

| Module | Holds | State |
| --- | --- | --- |
| `bitstream` | Bit reading and writing | exists |
| `isobmff` | MP4 and MOV (ISO base media) | exists |
| `matroska` | MKV and WebM | exists |

### craft-menu

Menus for macOS, Windows, Linux and the web, with shortcuts from `craft-keymap`.

| Module | Holds | State |
| --- | --- | --- |
| `bar` | The menu bar drawn with egui | exists |
| `context` | Panel and context menus from command entries | planned, step 3 |
| `driver` | Events from the platform menu in, menu bars out | exists |
| `help`, `language` | Help menu kit; language menu | planned, step 3 |
| `keymap` | Shortcuts and checked states from the keymap | exists |
| `keytips` | Alt keytips | planned, step 8 |
| `layout` | Platform layouts of an app's menus | exists |
| `mac` | macOS menu bar | exists |
| `model` | Menu tree and flat rows | exists |
| `palette` | Menu commands as command-palette entries | exists |
| `recent` | Recent files, over the host's files | planned, step 3 |
| `ribbon` | Ribbon with backstage, gallery, customize dialog and quick access toolbar | planned, step 8 |

### craft-pages

Page geometry, setup, ranges, numbering and paged-document UI. New crate.

| Module | Holds | State |
| --- | --- | --- |
| `layout` | Paged layout: rectangles, columns, facing pages | planned, step 6 |
| `numbering` | Page numbering styles | planned, step 6 |
| `range` | Page range parsing | planned, step 6 |
| `setup` | Page setup model | planned, step 6 |
| `show` | Slideshow playback | planned, step 8 |
| `ui::access` | Accessibility bridge for a canvas of pages | planned, step 6 |
| `ui::column_bar` | Column width bar | planned, step 6 |
| `ui::flatten_preview` | Transparency flattener options and preview | planned, step 4 |
| `ui::grid` | Page grid overlay | planned, step 6 |
| `ui::header_footer` | Header and footer zones | planned, step 6 |
| `ui::list` | Page and artboard list | planned, step 6 |
| `ui::margins` | Margin handles on rulers | planned, step 6 |
| `ui::marks` | Text frame marks overlay | planned, step 6 |
| `ui::preview` | Page preview overlay | planned, step 6 |
| `ui::range` | Page range field | planned, step 6 |
| `ui::reading` | Reading view | planned, step 6 |
| `ui::setup` | Page setup dialog | planned, step 6 |
| `ui::show` | Presenter window, transitions, timer, pen and laser | planned, step 8 |
| `ui::sync_views` | Synchronised side-by-side views | planned, step 6 |
| `ui::table` | Document tables: edges, properties, size picker, cell selection | planned, step 6 |
| `ui::text_selection` | Text selection layer over rendered pages | planned, step 6 |
| `ui::thumbnails` | Page thumbnail strip | planned, step 6 |
| `ui::view` | Paged document view | planned, step 6 |

### craft-pdf

Shared PDF reading, rendering and optional writing. Exists.

| Module | Holds | State |
| --- | --- | --- |
| `link` | Which link targets may become clickable links | exists |
| `raster` | Bounded RGBA8 raster output | exists |
| `read` | Bounded opening and interpreter guards | exists |
| `write` | PDF authoring | exists |

### craft-plate-test

Print-colour test scenes and PDF structure checks. Exists (`gs`, `judge`, `reference`, `report`, `scene`, `structure`, `tiff`).

### craft-platform

Small platform lifecycle services, one cargo feature per service. No craft-ui.

| Module | Holds | State |
| --- | --- | --- |
| `appearance` | Desktop colour, contrast and motion preferences | exists; reduced-motion setting planned, step 4 |
| `autosave`, `ui::recovery` | Autosave and crash recovery | planned, step 4 |
| `browser` | Browser downloads | exists |
| `clipboard` | Clipboard with synthetic input and size limits | planned, step 5 |
| `clock` | Injectable clock | planned, step 3 |
| `default_app` | Default app registration | planned, step 5 |
| `dialog_outcome`, `dialogs` | Dialog results with evidence; picker creation on the UI thread | exists |
| `job` | Background jobs and preview jobs | planned, step 4 |
| `logs` | Crash and command logs | planned, step 3 |
| `open` | OS open requests, drops, open and reveal | planned, step 4 |
| `pacing` | Frame pacing and work budgeting | planned, step 3 |
| `pen` | Pen input: pressure, tilt, eraser end | planned, step 3 |
| `process` | Bounded output capture for one-shot programs | exists |
| `screen` | Screen colour sampling | planned, step 4 |
| `session` | Reopen last session | planned, step 3 |
| `temp` | Private temp files | planned, step 5 |
| `usage` | Resource usage readout | planned, step 3 |
| `watch` | File change watcher | planned, step 5 |

### craft-prepress

Ink model, plates from a PDF, coverage, ink limits, composite preview. Exists.

| Module | Holds | State |
| --- | --- | --- |
| `cms`, `composite` | Process colour conversion; composite from plates | exists |
| `coverage`, `ink`, `plates` | Ink coverage, ink model, page separation | exists |
| `flatten` | Display list for the transparency flattener | exists |

### craft-print

System PDF print spooler (Unix and Windows) and the print dialog.

| Module | Holds | State |
| --- | --- | --- |
| `browser` | Browser print | planned, step 4 |
| `progress`, `sheets`, `ticket`, `windows` | Worker events, sheets of a print-ready PDF, print ticket, Windows spooler | exists |
| `spool` | Spooler front end for the print dialog | planned, step 4 |
| `ui` | Printer selection and job feedback | exists; print dialog planned, step 4 |
| `ui::layout` | Print layout sheet | planned, step 4 |

### craft-quit

Quits an app the platform's way.

| Module | Holds | State |
| --- | --- | --- |
| `close` | Routes an egui window close through the platform quit | exists; the unsaved-changes guard planned, step 2 |
| `macos`, `menu` | `terminate:` on macOS; the app menu with a Quit that asks first | exists |

### craft-review

Comments, annotations, signatures, versions and revisions; collaboration behind `collab`. New crate.

| Module | Holds | State |
| --- | --- | --- |
| `annotate`, `ui::annotate` | Page annotation tool | planned, step 6 |
| `collab` (feature) | Presence, voting, chat, join by code, with `ui::chat`, `ui::presence`, `ui::vote` | planned, step 6 |
| `comments`, `ui::comments`, `ui::thread` | Comments panel and threads | planned, step 6 |
| `revisions`, `ui::revisions` | Revision compare list | planned, step 6 |
| `signature`, `ui::signature` | Signature pad | planned, step 6 |
| `versions`, `ui::versions` | Version history panel | planned, step 6 |

### craft-settings

Shared settings storage: platform folders, `config.toml`, atomic saves, recovery, migration. No egui.

| Module | Holds | State |
| --- | --- | --- |
| `document` | TOML documents, per-section parsing, comment-preserving merges | exists |
| `error` | Reports and errors | exists |
| `migrate` | Schema migrations and import of older files | exists |
| `paths` | Choosing the configuration folder | exists |
| `profiles` | Settings profiles | planned, step 4 |
| `recovery` | Keeping damaged and previous files | exists |
| `remember` | `Remembered<T>`: last-used options for dialogs, panels and tools | planned, step 2 |
| `store` | In-memory store | exists; multi-instance merge planned, step 4 |

### craft-table

Spreadsheet grid and related UI. UI-only family. New crate.

| Module | Holds | State |
| --- | --- | --- |
| `ui::chart` | Chart painting | planned, step 6 |
| `ui::conditions` | Condition builder | planned, step 6 |
| `ui::error_tag` | Error-checking tag | planned, step 6 |
| `ui::fields` | Field list pane | planned, step 6 |
| `ui::filter` | Filter buttons | planned, step 6 |
| `ui::grid` | Spreadsheet cell grid | planned, step 6 |
| `ui::outline` | Row and column outline | planned, step 6 |
| `ui::refpick` | Cell reference picking | planned, step 6 |
| `ui::slicer` | Slicer filter chips | planned, step 6 |
| `ui::stats` | Selection statistics | planned, step 6 |

### craft-text

Text processing below the composer, and text UI.

| Module | Holds | State |
| --- | --- | --- |
| `bidi` | Bidirectional text (UAX #9) | exists; logical-end elision planned, step 4 |
| `find` | Find and replace | planned, step 6 |
| `hyphen` | Hyphenation | exists |
| `linebreak` | Knuth-Plass line breaking | exists |
| `lists` | Bullet paragraph layout | planned, step 6 |
| `quotes` | Smart quotes | planned, step 4 |
| `segment` | Line-break opportunities | exists |
| `spell` | Spelling check | planned, step 4 |
| `styles` | Named style list | planned, step 6 |
| `ui::character`, `ui::style_panel` | Character format controls; text style panel | planned, step 4 |
| `ui::columns`, `ui::hyperlink`, `ui::notes`, `ui::paragraph` | Columns dialog, hyperlink dialog, rich-text notes, paragraph format dialog | planned, step 6 |
| `ui::editor` | Canvas text editor | planned, step 4 |
| `ui::find`, `ui::styles` | Find bar; named styles panel | planned, step 6 |
| `ui::ruler`, `ui::tabs_dialog` | Text ruler with tab stops; tab stops dialog | planned, step 6 |
| `ui::spelling` | Spelling dialog | planned, step 4 |

### craft-theme

The theme model: colour, scale and token primitives, semantic and component tokens, contrast rules, the theme file format, built-in themes, and the `craft-theme check` command. No egui. New on the branch.

| Module | Holds | State |
| --- | --- | --- |
| `app_sizes` | An app's size overrides (`sizes.json`) | exists |
| `builtins` | The ten built-in themes | exists |
| `color`, `color_model` | sRGB, OKLab and OKLCH with gamut mapping; colour models a picker edits in; hex helpers | exists |
| `components` | Component tokens, one struct per family | exists |
| `contrast` | WCAG contrast and repair | exists |
| `derive`, `ramp` | Seeds to ramps to tokens; 12-step ramps | exists |
| `dump`, `explain`, `report` | Token text dump, why each colour has its value, contrast sweep | exists |
| `format` | Theme file schema version 1: parse, validate, write | exists |
| `resolve` | Theme and appearance to concrete tokens | exists |
| `scale` | Space, radius, stroke, elevation, motion, opacity | exists |
| `selection` | The user's pick: theme per mode, mode, contrast, motion, size, text size | exists |
| `sizes` | Size sets | exists |
| `theme`, `tokens`, `values` | What a theme supplies; semantic tokens; tunable numbers | exists |

### craft-theme-store

The suite's theme files, machine-local library and shared selection, with a lock and revision protocol for concurrent apps. New on the branch.

| Module | Holds | State |
| --- | --- | --- |
| `backend`, `dir`, `disk` | Where the files live; atomic-rename disk store | exists |
| `client`, `watch` | App side of the selection protocol; folder watcher | exists |
| `copy` | Import as copy | exists |
| `selection`, `store` | `selection.json` and the lock and revision protocol; the theme library | exists |
| `testing` | Fixtures for adapter tests | exists |
| `ui::appearance` | `AppearancePage`: the whole appearance settings page | planned, step 4 |

### craft-timeline

Time, clips, keyframes, markers, frame cache, playback clock, and timeline UI. New crate.

| Module | Holds | State |
| --- | --- | --- |
| `cache` | Frame cache with prefetch | planned, step 6 |
| `clips` | Clip editing model | planned, step 6 |
| `clock` | Playback clock and follow modes | planned, step 6 |
| `frames` | Playback frame server | planned, step 6 |
| `markers` | Markers | planned, step 6 |
| `rates` | Media format presets | planned, step 6 |
| `timecode` | Timecode and durations | planned, step 6 |
| `ui::clips` | Clip editing UI | planned, step 6 |
| `ui::graph` | Keyframe graph editor | planned, step 6 |
| `ui::keyframes` | Keyframe lane | planned, step 6 |
| `ui::markers` | Marker UI | planned, step 6 |
| `ui::overview` | Overview strip | planned, step 6 |
| `ui::panel` | Timeline panel with track headers | planned, step 6 |
| `ui::preview` | Video frame painter and preview panel | planned, step 6 |
| `ui::render_queue` | Render queue list | planned, step 6 |
| `ui::ruler` | Time ruler | planned, step 6 |
| `ui::timecode` | Timecode fields and readout | planned, step 6 |
| `ui::transport` | Transport bar | planned, step 6 |

### craft-ui

The kernel. Roughly 100 features after the family crates take theirs.

| Module | Holds | State |
| --- | --- | --- |
| `behavior` | Shared keyboard, focus, selection, typing and reorder rules | exists; scrub and pointer wrap (step 3), input quirks (step 4), drag feedback (step 5), move gesture (step 7) planned |
| `buttons` | Icon buttons with app-supplied colours | exists, deprecated (kept until the last app moves) |
| `canvas` | Overlays and the canvas foundation: guides, rulers, marquee, readout, grid, view, scrollbars, hit testing | exists; cursor, navigation, snapping, raster display, pasteboard, split views, tool host (step 3), navigator and textures (step 6), hover, selection frame, compare (step 7) planned |
| `capture` | Screenshot completion slots | exists |
| `combo` | Combo boxes and drop-downs | exists, deprecated |
| `composites` | Field rows, form grid, group box, property rows, headers, curve editor, histogram, empty state, notice, toast | exists; notice centre planned, step 3 |
| `conformance` | An app's real appearance setup against the reference | exists |
| `controls` | Button, checkbox, radio, switch, fields, sliders, select, combo box, swatch, badge, tooltip, thumb row | exists; point picker (step 2), number expressions (step 3), long tail of controls (step 5) planned |
| `dialog` | Frame, footer, alert, prompt, full-window dialog, preview, local history, size, id-only stack | exists; typed stack, fields, parameter and command dialogs, size dialogs, prompts, unsaved guard, properties and report (step 2); new document, export, file list, recovery, progress (step 4); guides, About, password, list manager (step 5) planned |
| `direction` | Right-to-left interface direction | exists |
| `docking` | Panel docking, floating groups, iconic column, workspaces | exists; dock queries and commands, arrange, workspaces, multi-window host planned, step 3 |
| `edit` | The edit lifecycle | exists; history panel and snapshots planned, step 6 |
| `focus` | Window focus as pointer gestures read it | exists; region focus navigation planned, step 3 |
| `fonts` | The UI font stack every app installs | exists; on-demand per-script registration planned, step 4 |
| `icons` | Icons by meaning, rasterised per size, tinted when drawn | exists; tool icon set (step 3), painting helpers (step 5) planned |
| `ime` | Input methods for text typed onto canvases | exists |
| `inspect` | Debug recorder for layout boxes and the AccessKit tree | exists |
| `interface_scale` | Device pixels per point | exists |
| `layout` | Pane geometry and resize handles | exists |
| `list` | Rows for panels that list things; tree; drag; rename | exists; sortable table, metadata table, event log (step 5), findings (step 4) planned |
| `localized` | Region-aware number widgets; the word tables | exists; length units planned, step 3 |
| `menu` | Menu, context menu, menu bar, press menu | exists; keep-open toggle rows planned, step 5 |
| `motion` | Animated caret and selection outline; reduced motion | planned, steps 4 and 5 |
| `panes` | Task panes and rail | exists; reorderable sidebar sections planned, step 6 |
| `patterns` | Panel, inspector, command palette, options bar, status bar, rail, toolbar | exists; options cells, status bar readout, toolbar drawer, coach marks (step 3), align bars (step 5) planned |
| `popover`, `popup_list` | Anchored popups; the list popup under a field | exists |
| `preferences` | Preferences window model, search, body | exists; settings-document pages and cache page planned, step 4 |
| `presets` | Preset browser: model, view, menu, toolbar | exists; library model, named-preset menu, pickers planned, step 6 |
| `primitives` | Surface, state layer, focus ring, divider, stack, interactive, text, icon, handle | exists; painter helpers planned, step 5 |
| `shell` | `AppShell`, `CanvasShell`, `ScreenMode` | exists; start screen planned, step 3; office-style preset planned, step 8 |
| `story` | Stories and the registry macro | exists |
| `tab_strip`, `tabs` | Document and panel tab strips; tab fitting | exists; drag between document tabs planned, step 3 |
| `testing` | Accessibility, style dump, screens, symbols | exists; headless render harness, i18n audit planned, step 3 |
| `text` | Shaped text in a reading area | exists; elided text and path labels planned, step 5 |
| `theme` | Tokens bridge to egui styles | exists |
| `title_bar` | App-drawn title bar, caption buttons, resize zones | exists |
| `viewport` | Windows in their own OS viewport | exists |

### craft-ui-gallery

The gallery app (desktop and web). Not published. Depends on every `ui` crate. New on the branch.

| Module | Holds | State |
| --- | --- | --- |
| `appearance` | The appearance bar and More popover | exists |
| `browser`, `inspector`, `usage` | Token browser, inspector overlay, used-by | exists |
| `capture` | Contact-sheet capture | exists |
| `faces` | Face specimens | exists |
| `fields`, `layout`, `gallery` | Gallery chrome and workspaces | exists |
| `persist` | Settings kept between runs | exists |
| `preview` | Mock app preview | exists |
| `probe`, `problems` | Probing and reporting layout problems | exists |

### craft-ui-macros

Derive macros for story arguments. New on the branch.

### craft-update

Release checking, downloading, installing and self-updating for the app suite and the launcher.

| Module | Holds | State |
| --- | --- | --- |
| `app`, `suite`, `version`, `pins` | Apps, releases and versions | exists |
| `desktop_entry`, `detect`, `distro`, `folders`, `install`, `manifest`, `msi_*`, `packages`, `paths`, `platform`, `portable`, `process`, `queue`, `status` | Installing, detecting and removing apps | exists |
| `dialog` | Startup update prompt | exists; moves onto the dialog stack, with a startup modal queue, planned step 2 |
| `http`, `proxy`, `release`, `source`, `time` | Network access and release lookup | exists |
| `protocol` | Vocabulary jobs and results report in | exists |
| `self_update`, `updater` | Launcher self-update; the update flow of an egui app | exists |

### craft-window

Decides who draws the window frame, makes an app-drawn strip act as the title bar, and keeps the window list, placement and restore.

| Module | Holds | State |
| --- | --- | --- |
| `desktop` | Desktop and backend detection | exists; visible work area planned, step 3 |
| `layout` | Window button layout per desktop | exists |
| `list`, `menu` | Open windows; the Window menu | exists |
| `mac` | macOS traffic lights, measured | exists |
| `place` | Cascaded placement kept on screen | exists |
| `policy` | Frame policy | exists |
| `restore` | Window restore | exists |
| `title_bar` | An app strip as the title bar | exists |
| `viewports` | Immediate viewports per window | exists |

## Appearance

| Setting | Options | Changes | Stored |
| --- | --- | --- | --- |
| Theme | 10 built-ins or a JSON file; chosen per mode (`theme_light`, `theme_dark`) | colours; shape for Adwaita, Breeze, Classic, Soft | suite |
| Mode | light, dark, system | palette | suite |
| Contrast | standard, high, system (default) | high: 7:1 text, 4.5:1 control cues | suite |
| Size | Compact, Comfortable | heights, padding, gaps, type, corners | suite |
| Text size | 0.9x, 1x, 1.15x, 1.3x | type only | suite |
| Reduced motion | full, reduced, system (default) | animations to zero | suite |
| Interface font | Suite face, System (OS UI face) | UI face | app |
| Direction | Language (default), LTR, RTL | mirroring; sliders, number fields, key chords unmirrored | app |
| UI scaling | Auto, or 75-300 % (steps of 5 from 75 to 125, then 150, 175, 200, 250, 300) | whole-interface zoom; sizes that do not fit the screen are hidden | app |

Storage:

- **Per suite**: theme, mode, contrast, size, text size, motion in the shared `selection.json`. Contrast, motion and text size always follow the suite.
- **Per app**: a default (theme pair, mode, size; creative apps ship Dark and Compact, office-style apps may ship Light) and an optional override in the app's preferences ("Use a different ... in this app"). Interface font, direction and UI scaling are app settings.

UI scaling rules:

- The list leaves out sizes that would make the window smaller than the app's minimum on the current screen. The size in use always stays listed.
- Every app has a "Reset UI Scaling" command.
- Pointers an app draws (rotate and resize arrows) grow with the interface scale.

Built-in themes (10; 17 mode variants). Suite default: ArtCraft.

| Theme | Modes | Look |
| --- | --- | --- |
| ArtCraft | light, dark | mid-grey panels; suite default |
| Pro | light, dark | neutral greys |
| Pro Light Gray | light | light grey panels |
| Graphite | light, dark | near-black fields in dark |
| Studio | light, dark | PhotoCraft Studio colours |
| Solarized Dark | dark | muted teal-tinted dark |
| Adwaita | light, dark | rounded, light grey; radii 6 / 9 / 12 |
| Breeze | light, dark | flat, 5 px corners everywhere |
| Soft | light, dark | low contrast (declared); corners rounder by +1 / +2 / +2 |
| Classic | light | classic square, bevelled look, 1 pt bevel, all radii 0 |

A two-mode theme is one theme with a variant per mode. A mode can carry a partial `seeds` override, so each mode derives its own colours instead of freezing them as pins. Picking a two-variant theme sets both modes. A theme file can name an icon pack that replaces the base icons it covers; missing icons fall back to the base set.

The gallery is also the base of a theme editor for people who make themes. Theme editing is designed for them, not only for developers.

## Colour

1. Seeds: neutral, accent, good, warn, bad, info
2. Ramps: 12 steps per seed per mode, OKLCH, fixed lightness curve
3. Semantic tokens (92 in the eight groups below), mapped to ramp steps by meaning
4. Pins: any token by hand
5. Anchoring: unpinned background greys keep their distance from the theme's panel colour, so they follow a pinned panel. Text greys stay on the fixed curve
6. Contrast pass: derived colours moved to WCAG 2.2 AA; pins kept and reported (high contrast: everything adjusted)
7. Component tokens from semantic tokens; each pinnable (about 1,100 resolved tokens per variant)

| Group | Tokens |
| --- | --- |
| Surface | background, canvas, canvas pattern, chrome, panel, raised, sunken, knob, scrim, shadow |
| Text | normal, muted, faint, disabled, on accent, on strong, on selected, on status |
| Line | divider, border, control edge, knob edge, mark, focus |
| Accent | solid, hover, pressed, strong (+ hover, pressed), soft, text |
| Status | good, warn, bad, info x fg, bg, border, solid |
| State | hover, pressed, selected, selected hover, disabled fill |
| Canvas overlay | stroke, halo, handle, marquee, guide, grid |
| Data | 8 series; histogram R, G, B, luma |

Accent fills that carry text (menu highlight, primary buttons, solid accent badges) use the strong pair. The strong accent is the theme's accent moved in OKLCH lightness only as far as white text needs for 4.5:1. Its hover and pressed variants move away from the text by the ramp's hover and pressed steps.

| Contrast pair | Standard | High |
| --- | --- | --- |
| Text on its surface or fill | 4.5:1 | 7:1 |
| Control cues (focus, check and radio outlines, accent marks, overlays) | 3:1 | 4.5:1 |
| Borders, field edges, hover, pressed, dividers | no minimum | no minimum |
| Disabled | exempt | exempt |

Guides, path outlines and box lines are bare lines in a colour the user chooses, so they are exempt from the overlay contrast rule.

## Shape, size and type

Shape and size come from the Size preset, never the theme (except Adwaita, Breeze, Classic and Soft for shape). The presets are data, resolved at run time. Apps can override them (see [How an app tunes a component](#how-an-app-tunes-a-component-without-writing-its-own)).

| pt | Compact | Comfortable |
| --- | --- | --- |
| Control height sm / md | 20 / 24 | 24 / 30 |
| Icon sm / md / lg | 12 / 16 / 20 | 14 / 18 / 22 |
| Gap inline / row / stack / section | 4 / 4 / 6 / 12 | 6 / 6 / 8 / 16 |
| Padding control / panel / dialog | 6 / 8 / 16 | 8 / 12 / 20 |
| Radius control / popup / window / panel | 3 / 4 / 6 / 0 | 6 / 8 / 12 / 0 |
| Buttons, badges, slider tracks | pill | pill |
| Bar title / tab / status / options | 32 / 26 / 24 / 36 | 38 / 30 / 30 / 42 |
| Panel header | 28 | 34 |
| Rail width / button | 40 / 30 | 50 / 36 |

The type scale is under [Design principles](#principles). NumberField defaults to a width of three control heights; apps set other widths per field.

- Docked and floating panels: square. Floating panels add a shadow
- Edge: flat or bevel; only the Surface primitive reads it
- Elevation: shadow levels for popups, dialogs, floating panels, knobs; none under a bevel
- Pill radius: a `radius.pill` token shapes switch and slider tracks and knobs, progress bars and badges. Classic sets it to 0
- Faces: the built-in faces stay until the face comparison is done. The interface font setting offers the OS UI face as an alternative; the mono face stays the suite's
- Icons: Phosphor is the base set; themes may supply an icon pack, and the gallery's active set defaults to Lucide. Strokes 1.25 pt at the small size, 1.5 pt at medium and large. Icons are typed values generated at build time from the SVG folders (four sets, 540 files)

## Components

The kernel's registry holds 165 stories for 79 components. `craft-color`, `craft-draw` and `craft-imaging` register about twenty more components. Each family crate adds its own stories as it lands, and the gallery shows all of them.

Every component has a builder API, semantic props only, gallery stories, AccessKit support, keyboard behaviour, a keyboard-only focus ring, RTL support and reduced-motion support.

| Level | Count in `craft-ui` | Components |
| --- | --- | --- |
| Foundations | 3 | colours, edit lifecycle, stock egui |
| Specimens | 1 | icons |
| Primitives | 18 | Surface, State layer, Focus ring, Divider, Stack, Row, Interactive, Text, Icon, Handle · canvas overlays: canvas, grid, guides, marquee, readout, reference point, rulers, smart guides |
| Controls | 23 | Button, IconButton, ToggleButton, Segmented, Link · Checkbox, Radio, Switch · TextField, SearchField, NumberField · Slider, RangeSlider, AngleDial · Select, ComboBox · Swatch, Badge, Kbd, Spinner, Progress, Tooltip, ThumbRow |
| Composites | 25 | Field, FormGrid, GroupBox, PropertyRow, SliderRow, SectionHeader, SectionLabel, PanelHeader, PanelFooter · Menu, ContextMenu, PressMenu, MenuBar · List, ListRow, TreeRow · Tab, TabStrip · Toast, Notice, EmptyState · CurveEditor, CurvePreview, Histogram · PresetBrowser |
| Patterns | 16 | Panel, Popover, Dialog, FullWindowDialog, PreferencesWindow, CommandPalette, Inspector, DockArea, ViewportWindow, PanelRail, ToolRail, Toolbar, OptionsBar, StatusBar, TitleBar, FlyoutButton |
| Shells | 1 preset | AppShell with the Canvas preset (`CanvasShell`) |

Family components:

| Crate | Components |
| --- | --- |
| `craft-color` | ColorPicker, ColorArea, ColorWheel, ColorDisc, ColorRamp, ColorPopover, HexField, SwatchPair, SwatchPalette |
| `craft-draw` | GradientSwatch, GradientEditor, GradientPalette, gradient line, TransformFields, transform box, bezier paths, crop frame, perspective crop quad, pen draft |
| `craft-imaging` | LevelsEditor, warp grid |

Shells:

- **Canvas** (built): a fixed document canvas, document tabs, tool rail, panel dock. For the photo, vector, design, figure, PDF and CAD apps.
- **Workspace** (planned): the whole body is equal panels in named workspaces. For the film, effects and sound apps.
- **Office-style** (planned, step 8): the ribbon as a top slot, backstage view and task panes.
- Panels live as tabs in an expanded dock column or as icons in a collapsed (iconic) column, never both. A click on an icon opens its group in a flyout beside the column. Dialogs move by dragging the title only. A floating panel group has square corners and the floating shadow.

Absent so far: Workspace and office-style presets, Frame preset, timeline, waveform, scopes.

## Translation and accessibility

Translation:

- Library words (button labels, dialog text, error text) live as rows in the shared `craft-i18n` catalog (`common.tsv`). There are no per-app translations of library words
- The app installs its translator once per language change, through the host
- Lookup: the language's row, then the English row, then the built-in English. English shows when a row is missing
- Labels the host app sets win and are never replaced; a translation missing a placeholder is ignored
- RTL: chrome mirrors; sliders, number fields and key chords never mirror
- `craft::words::all()` lists the word tables of every enabled crate for the catalog extraction

Accessibility:

- AccessKit roles on widgets (button, spin button, slider, text input, combo box, list option, radio, dialog, menu item, tab, tree item)
- High contrast: 7:1 text, 4.5:1 control cues, every colour adjusted
- Keyboard: keyboard-only focus ring. Behaviour follows the suite's own apps, WAI-ARIA where they have no convention. Examples: Tab visits fields only in dialogs and every control elsewhere; a swatch palette and a guide set are one Tab stop with arrow keys inside; a tab strip is one Tab stop; Ctrl+Tab focuses a dialog's title row
- Lists: Cmd or Ctrl+] and [ move the focused row in every reorderable list; F2 renames; Enter runs the row's default action
- Standard contrast follows each theme's design. High contrast is the accessible option and meets the standard

## Tests and gates

| Gate | What it holds |
| --- | --- |
| Token dumps | 68 files: 17 theme-mode variants x {standard, high} x {Compact, Comfortable}, plus one app-sizes file; rewrite with `UPDATE_DUMPS=1` |
| Style dumps | 17 egui style files, one per theme-mode |
| Component dumps | 70 files across `craft-ui`, `craft-color`, `craft-draw` and `craft-imaging` |
| Story registry | 165 kernel stories and their default args |
| Title-bar dumps | 2 files |
| Other tests | visual regression, screens, docking workflows, interface scale, font rendering, web fonts |
| Conformance | an app's real appearance setup against the library's reference, across the case matrix; fails on wrong theme, size or text size, a changed `Style`, own zoom or font override; `allow(field, reason)` for exceptions |
| Theme check | `craft-theme check <file>`: contrast report, adjustments, invalid fields |
| Design lint | `craft-design-lint check` (syn) and `check-clippy` |
| Gallery and screenshots | the gallery covers every `ui` crate; contact sheets are rendered locally and committed once the first app has migrated |

Text dumps of every token and component style are the regression gate.

Design-lint rules: `literal-size/*`, `raw-colour/const`, `style-mutation`, `tooltip/stock`, `clippy/inner-allow`. Exemption marker `// design: allow(reason)`; empty, invalid and unused markers are findings. Baselines and `--deny-unused` are supported. Exit 0 clean, 1 findings, 2 error. Each app runs the lint and the conformance test in its `xtask ci`.

## Migration strategy

### Approach

Everything is built in `craft-libs` first. The rule is "build them all in one spot, use them all in one way". A feature one app has today is built in the library even when no second app needs it yet. The work does not go family by family. It does go step by step, so each step ends in a library the apps can use.

- **PhotoCraft migrates first.** It has the most UI and the most features. Its port branch runs on the library while the library grows.
- **VectorCraft migrates second.** It shares the Canvas shell and most of PhotoCraft's skeleton. If the two ports look alike, the library is general enough.
- **The other apps follow**, each adopting the steps it needs.
- Each app switches in one pass, not gradually. An app's port is a single branch that replaces its own widgets.

### Acceptance for an app port

- Users do not notice significant differences, or see slightly more polish.
- Where an app draws one kind of component several ways, the port uses one. That consistency is part of the polish.
- A screenshot comparison of the app's upstream build against its port covers each screen. Differences a user would notice need a decision.
- A gap an app finds is library work, never an app workaround. The library fix comes first; the port then deletes its own code.

### Steps

| Step | Delivers | Depends on | Status |
| --- | --- | --- | --- |
| 0 | PhotoCraft adopts the types the library already has; gaps it finds become library work | none | done |
| 1 | Foundations: `ui` feature names, `craft-host`, module moves into family crates, the `craft` facade, gallery and word registry over every crate, one egui revision | 0 | done |
| 2 | Dialog framework: typed results, parameter and command dialogs, size dialogs, prompts, unsaved guard, startup modal queue; text and number control gaps | 1 | next |
| 3 | Shell (dock, panel menus, floating frames, workspaces, start screen, focus regions), keymap completion, canvas core, options bar cells, testing tools | 2 | planned |
| 4 | Files, preferences, text and fonts, colour, previews and jobs | 2, 3 | planned |
| 5 | Common controls, system services, value formatting, guides and About, align bars | 2 | planned |
| 6 | 6a: layers, presets, history, navigator, browsing. 6b: pages, timeline, review, tables | 2, 3, 4 (6a); 2, 3, 5 (6b) | planned |
| 7 | Canvas tools and the image-editing workspaces | 3, 4, 6a | planned |
| 8 | Office-style shell, audio, scripts and code, slideshow, GPU | 2, 3; 6b timeline for audio | planned |

Size: about 185 library tasks in all, plus about 28 PhotoCraft tasks and about 20 VectorCraft tasks. Step 3 is about 30 library tasks, step 4 about 35, step 6 about 44.

### Critical path

```
2 --> 3 --> 4 --> 6a --> 7 --> PhotoCraft full port --> VectorCraft full port
       |     |
       |     +--> 5 (off the path: PhotoCraft needs only guides and About from it)
       +--> 6b, 8 (off the path for PhotoCraft)
```

- Step 2 gates every feature that opens a dialog.
- Step 3 gates the canvas tools (7), the colour droppers and text editor (4), and every panel (6) through the dock's panel-menu host.
- Step 4's colour and font pickers gate the stroke, shape and text work in 7.
- Step 6a gates PhotoCraft's panels and the brush presets in 7.
- Steps 5, 6b and 8 run beside the path when capacity is free. Their inputs are other apps' code, so they do not block PhotoCraft.

### What each step delivers

**Step 0: adopt existing types.** PhotoCraft swaps widgets it already shares in kind with the library for the library's. Each difference it finds is a gap row (see [Library gaps from the PhotoCraft port](#library-gaps-from-the-photocraft-port)).

**Step 1: foundations.** No pixel changes. It changes how apps reach the library.

- The UI feature is named `ui` in every crate, plus `screen-testing`.
- `craft-host` carries the services every feature reads.
- Domain code moves out of the kernel into `craft-draw`, `craft-imaging`, `craft-color` and `craft-print`.
- The `craft` facade gives one dependency and one pin.
- The gallery and the word registry cover every `ui` crate.
- Each app's `xtask` checks the egui revision.

**Step 2: dialog framework.** A feature dialog is one value an app pushes and one result it reads.

- Delivers: `DialogStack` with typed `Outcome`, `ParamDialog` from a parameter list, command forms, image and canvas size dialogs, confirmation prompts, `UnsavedChangesGuard`, properties and report dialogs, `PointPicker`, `Remembered<T>`, and `craft-update`'s dialogs on the stack.
- PhotoCraft adopts: the stack for every dialog, parameter dialogs for filters, size dialogs, prompts, the unsaved guard, remembered options.
- PhotoCraft deletes: its dialog host and routing, its parameter-dialog builder, its field map, its size-dialog code and prompt helpers.

**Step 3: shell, testing tools, canvas core, options bar.** The window skeleton and what every canvas needs.

- Delivers: dock host and commands, panel menus, floating frames, Window menu and Arrange, named workspaces, start screen, region focus, shortcuts dialog completion, the keymap catalog as source of truth, zoom and navigation, cursors, snapping, raster display, the on-canvas tool host, options bar cells with number cells and scrubbing, `craft-automation`, the render harness.
- PhotoCraft adopts: the library dock, workspaces, `ScreenMode`, the shortcut catalog and dialog, canvas navigation, the tool host, options bar cells, automation.
- PhotoCraft deletes: its dock, panel-menu popups, floating card frames, shortcuts table, zoom ladder, cursor host, momentary-key and snapping code, options bar cell code, screenshot queue and control channel.
- VectorCraft: its mapping to library components runs during this step, and its gaps feed the specs of steps 4 to 7.

**Step 4: files, preferences, text and fonts, colour, previews.** The shared features every app has.

- Delivers: the file service, New Document, export dialogs, print, file intake; preferences over a settings document with `AppearancePage` and recovery; font picker, canvas text editor, style panels; `ColorPickerDialog`, droppers, samplers, screen picker, swatch panel, palettes, CMS additions; preview panes and job progress.
- PhotoCraft adopts all of it. It deletes its file dialogs, most of its preferences code, its recovery and path code, the family picker, its colour panel and picker bodies, and its job UI.
- VectorCraft adopts steps 1 to 4 on its port branch in one pass.

**Step 5: common controls, system services, value formatting, guides, About.** A small step for PhotoCraft.

- Delivers: sortable tables, chips, date and tree pickers, linked fields, mixed values, split button, breadcrumb, rating; clipboard, file watcher, default-app registration, temp files, Save flow; dates, byte sizes and elision; New Guide and Guide Layout dialogs, About, align bars.
- PhotoCraft adopts guides, About, align bars, linked size fields, clipboard and the Save flow.

**Step 6: layers, presets, history, browsing (6a); pages, timeline, review, tables (6b).** 6a is on PhotoCraft's path. 6b serves other apps and runs beside steps 7 and 8.

- 6a delivers: `LayersPanel` over a `LayerTree` trait, channels, the preset model and menus, a history panel with one-edit steps and snapshots, navigator, scopes, actions, thumbnails and browsing.
- After 6a, all of PhotoCraft's panels are library panels in the library dock. It deletes its layer, history, navigator and histogram panels and its preset model.
- 6b delivers: paged documents, timeline, comments and versions, spreadsheet tables. VectorCraft adopts layers and presets after 6a, and artboard lists after 6b.

**Step 7: canvas tools and image-editing workspaces.** PhotoCraft's last large deletion.

- Delivers: transform, warp, selection and mask tools, vector and shape tools, crop, measure, brush, and the full-window editors: select and mask, liquify, filter gallery, fill from surroundings, perspective plane, wide-angle correction, layer style, adjustments.
- PhotoCraft adopts every tool through the library tool host and the workspaces as library editors. After this step its port runs the lint clean and passes the conformance test and the render matrix.
- VectorCraft adopts the draw family and finishes its port.

**Step 8: office-style shell, audio, scripts, slideshow, GPU.** Mostly other apps' code.

- Delivers: ribbon, backstage view, keytips and the office-style preset; audio engine and mixer UI; code editor and console; slideshow presenter; `craft-gpu`.
- PhotoCraft's only task is to move its GPU display onto `craft-gpu`. Nothing visible changes.

### How each step runs

1. **Spec.** A written spec for the step is reviewed before any code. Open questions are settled in one round.
2. **Parallel tracks.** Implementation splits into independent tasks, one branch each, with narrow checks only.
3. **Integration.** The branches merge into one, and the dumps are regenerated.
4. **One full test run**, then a review of each track.
5. **Fixes**, then a final full run.
6. **PhotoCraft adoption.** The port branch repins to the library and replaces its code with the new features. The screenshot comparison runs at each adoption task.

Each step also adds its families to the library docs: a page per family with the out-of-the-box recipe (the `Host`, the trait the app implements, the builder call, the `Outcome`), plus a migration table of old path to new path.

### Library gaps from the PhotoCraft port

The port has recorded 108 gap rows so far. At least 26 are closed with a merged library change, and 34 are scheduled in a later step. The rest have a track in progress or need no library change. The gaps range from small (a missing option on a slider row) to structural (a floating frame with no title-bar move). They are why the steps above include the shell and common controls before the tools.

## Landing strategy

### Size

Measured on `design-system` against `upstream/main` (c39e435c):

| Measure | Value |
| --- | --- |
| Commits | 1,107 |
| Files changed | 1,547 |
| Lines added | about 332,000 |
| Lines removed | about 5,000 |
| Generated test dumps among the added lines (`*/tests/dumps/*`) | about 105,000 |
| Icon set files | 540 (four sets) |

The branch is mostly additive. A review reads the dumps and icons as data, not code.

### The series

The work lands as a series of upstream pull requests, one at a time, bottom-up. Each PR builds and passes CI on its own. Apps pin `craft-libs` by commit, so a merged PR changes nothing in an app until the app moves its pin.

| Order | PR | Contents |
| --- | --- | --- |
| 1 | egui patches | The egui fork's patches go to the storytold egui fork first; craft-libs main moves its egui pin. Everything else depends on this |
| 2 | Icon sets | Data only |
| 3 | Token model and themes | `craft-theme`, `craft-theme-store`, `craft-color`. New crates; nothing depends on them yet |
| 4 | `craft-ui` core | Theme bridge, primitives, controls, edit lifecycle, behaviour |
| 5 | Composites, patterns, shells | With `craft-draw` and `craft-imaging` |
| 6 | Host, facade and changes to existing crates | `craft-host`, `craft`, and the changes in `craft-menu`, `craft-keymap`, `craft-window` and `craft-platform` |
| 7 | Tooling | The gallery, `craft-design-lint`, conformance |
| 8 | Apps | PhotoCraft, then VectorCraft, one PR each, then the other apps |

The egui patches are seven numbered changes (E22 to E28) on top of the fork's earlier patches. They add:

- the IME composition's pre-edit range on `TextEdit` state
- a public accessibility description for `Response`
- scroll-bar labels
- an AccessKit focus report that leaves egui focus alone
- a pointer press that goes to the widget under the press position
- `DragAndDrop::aborted_by_escape`
- `Popup::swallow_outside_press`, so the press that closes a popup does nothing else

### Breaking changes

About 100 public items in `craft-ui`, `craft-menu`, `craft-keymap`, `craft-window` and `craft-platform` are removed or renamed (106 on the current branch). That count is an upper bound: it counts every removed `pub` item, and items that moved to another crate count too.

The rule: each PR that removes or renames a public item keeps the old name as a deprecated alias or shim until every app has moved past it, so an app's next pin bump still builds. A migration table (old path to new path) goes with each PR. The old APIs that apps use today (the icon-button and combo-box modules, the old rail and number-field types) stay marked `#[deprecated]` until the last app that uses them has migrated.

### Timing

A layer lands once PhotoCraft's port has used it in earnest. Bottom layers land first. Landing early stops drift from upstream. The cost is that a landed layer's API is then shared, and changes to it need the same deprecation care.

### Staying current

Upstream keeps moving. `design-system` merges upstream main daily. After each landed PR, the remaining work rebases onto main. Conflicts that are only lockfile and version pins need no local build; CI checks them.

## Status and open questions

### What exists

| Area | State |
| --- | --- |
| Token model, ten built-in themes, store, bridge | built |
| Primitives, controls, composites, patterns | built (79 components in `craft-ui`) |
| Canvas shell, docking with iconic column and flyouts, title bar | built |
| Dialog frame, prompts, full-window dialogs, preferences window | built; typed stack in step 2 |
| `craft-color` picker parts, `craft-draw` crop, gradient, pen and transform, `craft-imaging` levels and warp grid | built |
| `craft-host`, the `craft` facade, one egui revision check | built (step 1) |
| Gallery with Components and Theme workspaces, Variants, app preview | built |
| Design lint, conformance test, `craft-theme check` | built |
| PhotoCraft on library components | window frame, title bar, menus (native on macOS), start screen, document tabs, status bar, options bar, tool rail, dock, dialog frame, Levels, Curves and Threshold, histograms, gradient editor, layer and guide dialogs, fill and stroke dialogs, Layers panel body |
| Still PhotoCraft's own code, restyled by the theme | most panels, most dialogs, canvas overlays, other adjustment editors |
| macOS | the port builds on macOS; window, title bar and menu bar checks pass |
| Comparison | screenshots of 59 screens, current PhotoCraft beside the port, refreshed after each port batch |

### What is next

1. Step 2 (dialog framework) and step 3 (shell and canvas core), with PhotoCraft's adoption tasks after each.
2. Step 4 (files, preferences, text, colour).
3. Step 6a, then step 7, completing PhotoCraft's port.
4. VectorCraft's port, then the other apps.
5. The upstream PR series, starting with the egui patches once PhotoCraft's port has used the lower layers.

### Questions for the team

| Topic | Question |
| --- | --- |
| Landing order | Is the bottom-up order above right, or should the token model and `craft-ui` core land together? Should the apps' PRs follow each layer, or wait for the whole series? |
| Deprecation window | How long do deprecated aliases stay? Options: until every app has moved past them (the plan above), or a fixed number of releases |
| egui patches | Which of the seven patches go to upstream egui, and which stay in the storytold fork? |
| Facade name | `craft` is the facade crate's name. If it clashes with an app crate, the alternative is `craft-kit` |
| Crate granularity | The layout adds twenty crates to upstream's twenty-six. Is there a family that should merge, or a crate that should split? |
| Review load | Is a series of about 330,000 added lines, mostly generated or additive, reviewable as laid out, or should it split further? |
