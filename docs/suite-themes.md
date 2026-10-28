# Suite themes

A suite theme is a named set of seeds and pins from which every Craft app derives its colours,
shape and size tokens, so one choice in one app can restyle the whole suite. This page is the
reference for theme authors and for app developers adopting the suite theme.

| Crate | Contents |
|---|---|
| [`craft-theme`](../crates/theme/README.md) | Token model, derivation, contrast rules, theme file format, built-ins, selection and its resolution. No egui, no I/O; builds for wasm. |
| [`craft-theme-store`](../crates/theme-store/README.md) | The suite folder: user themes, `selection.json`, the lock/revision protocol and the change watcher. |
| [`craft_ui::theme`](../crates/ui/README.md#theme-mechanisms) | The bridge to egui. |

## Tokens

Tokens come in three layers. A theme file only states seeds and exceptions; everything else is
derived.

1. **Primitives**: colour, 12-step ramps, fixed scales (space, radius, stroke, opacity,
   duration, easing, elevation).
2. **Semantic tokens**: `surface.*`, `text.*`, `line.*`, `state.*`, `accent.*`, status, overlay,
   data and caption colours, plus shape, size, space, motion and type tokens.
3. **Component tokens**: one struct per component family, such as `button.primary.fill.rest`.
   Only curated fields can be overridden.

`resolve(theme, appearance, app)` builds the tokens for one appearance: mode (light, dark),
contrast (standard, high), size (compact, comfortable), text size (small, default, large and
larger) and reduced motion. Derivation order, later steps winning: seeds, then ramps, then the theme's pins, then
the app's pins (`apps.<id>`), then the theme's high-contrast pins, then the app's
(`apps.<id>.high_contrast`).

## Theme file (`craft-design-theme`, version 1)

```json
{
  "schema": "craft-design-theme", "schema_version": 1,
  "id": "user.navy", "name": "Navy", "version": 1,
  "variants": "both",
  "contrast_intent": "standard",
  "seeds": { "neutral": "#535353", "accent": "#78ADF6",
             "status": { "good": "#85DC6A", "warn": "#E8B046", "bad": "#FF8080", "info": "#78ADF6" } },
  "shape": null,
  "light": { "surface.panel": "#F0F0F0" },
  "dark":  { "surface.panel": "#535353" },
  "high_contrast": { "light": {}, "dark": {} },
  "components": { "button.radius": 4 },
  "apps": { "photocraft": { "light": {}, "dark": { "surface.canvas": "#282828" }, "high_contrast": {}, "components": {} } }
}
```

| Key | Required | Value |
|---|---|---|
| `schema`, `schema_version` | yes | `"craft-design-theme"`, `1` |
| `id` | yes | `[a-z0-9][a-z0-9._-]{1,63}`; `craft.` is reserved for built-ins and refused in the library; Windows device names (`con`, `nul`, `com1` ...) are refused as the first dot-separated part |
| `name` | no | display name; default the id |
| `version` | no | the author's revision, default 1 |
| `variants` | no | `"both"` (default), `"light"` or `"dark"` |
| `contrast_intent` | no | `"standard"` (default) or `"low"`: the theme is meant to be soft, and pickers label it "Low contrast" |
| `seeds` | yes | `neutral`, `accent` and `status.good/warn/bad/info`, all required |
| `shape` | no | `null`, or `{ "radius": { control, popup, window, panel, button }, "edge": "flat" or {"bevel": {"width": 1}} }`; a radius is a number or `"full"`; `radius` needs all five keys when present |
| `light`, `dark` | no | colour pins: dotted token path to `#RRGGBB` or `#RRGGBBAA`; surfaces (`surface.*` except `surface.scrim`) must be opaque |
| `high_contrast` | no | `{ "light": {...}, "dark": {...} }`, pins used only in high contrast |
| `components` | no | non-colour component overrides: a length (number or `"full"`), an edge, or `{ "level": 0-3 }` |
| `apps` | no | per-app `{ light, dark, high_contrast, components }`, applied over the theme's values for that app; its `high_contrast` pins apply after the theme's |

- JSON, at most 256 KiB. `schema_version` is the format; `version` is the author's revision.
- Paths are flat dotted strings (`"button.radius"`), never nested objects.
- A pin map for a mode the theme lacks is an error when it is not empty.
- `write` refuses a theme that would not parse back (`ThemeDef::validate`: pins for a missing
  mode, overrides of the wrong kind, translucent surfaces, negative or non-finite lengths).
- Unknown keys outside the pin maps are allowed and kept byte for byte when the store rewrites
  the file. An unknown path inside a pin map is an error.
- A newer `schema_version` is refused and the file left alone. A file with another `schema`
  (the earlier `craft-theme` format) is skipped by the library and counted, not reported.
- `craft-theme check <file>... [--json]` reports every error with its path (`dark."surface.panel":
  not a colour`) and every contrast warning per appearance. Exit code 1 for errors, 0 otherwise.

### Built-in themes

`craft.artcraft`, `craft.graphite`, `craft.studio`, `craft.adwaita`, `craft.breeze` (both
variants); `craft.pro-light`, `craft.pro-light-gray`, `craft.classic`, `craft.soft-light`
(light only); `craft.pro-dark`, `craft.solarized-dark`, `craft.soft-dark` (dark only). The two
soft themes have `contrast_intent: low`. `craft.app-default` is not a theme: it stands for the
app's own palette, and settings lists it beside the others. High contrast is a setting, not a
theme. `SUITE_DEFAULT` is `craft.artcraft`.

## Contrast rules

Users must be able to tell text and state indicators apart; nothing else is a rule.

| Pair | Standard | High |
|---|---|---|
| `text.normal`, `text.muted`, `text.faint` on surfaces, `state.hover` and `state.pressed` | 4.5 | 7 |
| `text.on_selected` on `state.selected` and `state.selected_hover` | 4.5 | 7 |
| `accent.text`, status text, `text.on_accent`, `text.on_status`, button text on its fill | 4.5 | 7 |
| `line.focus` on control backgrounds; `accent.solid` on `surface.panel` (checked and selected marks); `overlay.stroke` and `overlay.marquee` on their halo | 3 | 4.5 |

Hover, pressed, dividers, control edges and disabled text have no rule and take their derived
or pinned values in every contrast. A translucent fill is measured over `surface.panel`.
`text.on_selected` defaults to `text.normal`'s ramp step, so a neutral selection looks like the
rest of the text and a coloured one (a navy or blue selection) gets its own readable text.

| | Standard contrast | High contrast |
|---|---|---|
| Unpinned colour that misses | moved to the minimum | moved to the minimum |
| Pinned colour that misses | kept, reported as a warning | fixed, pins included |
| Pinned text on solid fills that misses | kept; the unpinned fills under it move | fixed, pins included |
| State fill under text that cannot reach 7 | n/a | the fill moves away from the text, as far as needed |

In standard contrast a component pair (`button.secondary.text.rest` on its fill) is reported only
when one of its colours is a component pin; a failing semantic colour is reported once, as the
semantic pair.

A theme with `contrast_intent: standard` should resolve with no warnings; built-ins do in both
contrasts, except the soft themes in standard contrast.

## Selection

The suite has one selection, in `selection.json` of the suite folder. Each field is set once a
user picks it in some app; unset fields come from each app's own defaults, so one app's default
never reaches the others.

```json
{ "schema_version": 2, "revision": 17, "library_revision": 4,
  "suite": { "theme": { "light": "craft.artcraft", "dark": "craft.graphite" },
             "mode": "system", "contrast": "system", "size": "compact", "text_size": "default" },
  "written_by": "pdfcraft 0.15.0" }
```

| Field | Values |
|---|---|
| `suite.theme.light`, `suite.theme.dark` | a theme id per mode; each slot optional |
| `suite.mode` | `light`, `dark`, `system` |
| `suite.contrast` | `standard`, `high`, `system` |
| `suite.size` | `compact`, `comfortable` |
| `suite.text_size` | `small`, `default`, `large`, `larger` |

- Picking a two-variant theme sets both slots; a single-variant theme sets only its own mode's
  slot.
- Version 1 files (`suite.theme` a string, `mode` only) are read by putting the theme in both
  slots; the next write is version 2. A `schema_version` of 3 or above is refused and never
  changed. Keys the app doesn't know are kept on every write.
- No `selection.json` means nobody has chosen anything. An app writes it only when the user
  picks, never on first start.

### In an app

- `SuiteChoice::with_defaults(&AppDefaults)` fills the unset fields from the app's
  `AppDefaults { theme: ThemePair, mode, size }`.
- Each app keeps an `AppChoice { theme, size }` in its own preferences for "use a different
  theme / size in this app"; `apply` lays it over the suite's selection. It has no `Default`:
  the app states what a fresh install shows.
- `resolve_selection(&selection, system, fallback_mode, &defaults, app, lookup)`, with `lookup` a
  `&mut dyn FnMut(&ThemeId) -> Option<ThemeDef>` (`&mut |id| store.theme(id)`), returns
  `Shown { theme, appearance, resolved, notices }`. `System` mode or contrast uses what the OS
  reports (`SystemAppearance`); with no answer, mode uses `fallback_mode` (notice
  `SystemSilent`) and contrast is standard.
- An unknown theme id falls back to the app default's slot, then to `SUITE_DEFAULT`, with an
  `UnknownTheme` notice. A single-variant theme in the other mode's slot shows its own variant
  (notice `OnlyVariant`).

## Suite folder and store

`SuiteDir::resolve(app_isolated)` picks the folder in this order:

1. `CRAFT_SUITE_DIR`. Set but empty: in memory, nothing on disk is read or written.
2. In memory when the app runs without a disk store (`app_isolated`: constructors, tests,
   portable installs).
3. `ArtCraft/themes` under the local config folder: `~/.config` on Linux and the BSDs,
   `~/Library/Application Support` on macOS, `%LOCALAPPDATA%` on Windows.

Web builds are always in memory, with the same operations.

```text
ArtCraft/themes/selection.json                     suite selection and revisions
ArtCraft/themes/selection.lock                     lock file, never deleted
ArtCraft/themes/library/<id>.json                  one user theme per file
ArtCraft/themes/selection.broken-<unix-ts>.json    a damaged selection, moved aside on the next write
```

Reading never creates anything; the folder appears with the first write.

### Writes

Reads take no lock; every write replaces the file by atomic rename. A write takes the exclusive
`selection.lock` (retries for 2 s, then `Busy`), reads `selection.json`, merges, writes with
`revision` one above the file's, and checks the file again just before the rename.

A damaged `selection.json` (or one over 64 KiB) is moved to `selection.broken-<unix-ts>.json`
before the next write. A file from a newer build gives `TooNew` and stays read-only.

`Store::update(base, change)` takes the state the user's change was made on and a
`SelectionChange` with the fields that changed (`theme_light`, `theme_dark`, `mode`, `contrast`,
`size`, `text_size`). Per field:

| Field | Disk | Result |
|---|---|---|
| not changed | anything | disk's value |
| changed | not chosen | mine |
| changed | equal to mine | disk's value |
| changed | another value, equal to base's | mine |
| changed | another value, not base's | conflict: another app chose it after the caller read |

A conflict returns `Conflict { disk, mine }` and writes nothing; the app offers "Apply mine".
A change naming a library theme that no longer exists is `NotFound`.

Library operations take the same lock and bump `library_revision`:

- **Import** validates the file and writes its original bytes to `library/<id>.json`, never
  replacing a file (`IdExists`); "import as copy" picks the first free `<id>-2`, `<id>-3` ...
- **Delete** refuses built-ins and a theme in either suite slot (`InUse`).
- **Export** returns a library theme's stored bytes, or a built-in written as a theme file.

Ids a user couldn't create (`craft.` prefix, device names) are `NotFound` and never reach the
file system. A failed write is undone and the error returned.

### Watching

Apps use `SuiteWatch::open(dir, options, wake)`, which owns the store on a worker thread and
publishes an immutable `Snapshot` (`accepted`, `index`, `active`, `app_theme`, `problems`,
`results`, `generation`). The worker checks the stamp of `selection.json` and of the themes in
use every 500 ms, compares bytes every 10 s, and lists the library only when `library_revision`
changes or on `Op::Rescan`. Each change publishes a snapshot and calls `wake` once.

Each frame the app compares `generation()` (one atomic load) with its cache key and rebuilds its
styles only when it changed. Changes go through `request(Op)`: `Update`, `Import`, `Delete`,
`Export`, `Rescan` and `AppTheme`; the result appears in `snapshot().result(id)`. `SuiteWatch::memory()`
runs without a thread, applying each request before it returns.

The lock file and revision protocol follow
[pdfcraft#1090](https://github.com/storytold/pdfcraft/pull/1090).
