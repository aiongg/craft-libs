# Craft design system: overview

As of 30 Oct 2026

## Try it

The component gallery runs from the `design-system` branch of this repository. One build holds every theme, and they switch at run time. Rust 1.95+.

```
git switch design-system
cargo run --release -p craft-ui-gallery
```

- Updates: `git pull`, rebuild
- Candidate fonts (optional): check out the craft-fonts repository at branch `design-system-fonts`, set `CRAFT_FONTS_DIR=<absolute path>` before `cargo run`
- Web: `cd crates/ui-gallery && trunk serve --release true index.html` (needs trunk + wasm32 target)
- Gallery contents: Overview (one tile per component of a level), a tab per component with all its stories, Variants (the selected story once per theme, mode, contrast or size), the appearance bar and its More popover

## Per app vs shared

Each app keeps the layout, placement, conventions and density of the industry tool it is modelled on. What is shared is the code for the building blocks and how they behave.

| Per app | Shared |
| --- | --- |
| Layout and placement follow each app's industry original | Components (buttons, fields, sliders, swatches, colour wheel, bezier handles, dialogs, menus) |
| Conventions (menu structure, shortcuts, tool behaviour) | Family patterns (dock and tool rail for creative apps, ribbon for office apps) |
| Default density (Compact for dense editors, Comfortable for roomier apps such as PDFCraft) | Behaviour (focus and Tab order, Esc cancels, one undo step per drag, number-field scrubbing) |
| Default theme and mode (dark for creative apps, light for office apps) | Themes, high contrast, translation, screen-reader support |

### How an app tunes a component without writing its own

1. **Density per app**: Compact or Comfortable.
2. **Component tokens**: named sizes and colours per component. One value changes it app-wide.
3. **Options on the component**: builder options for structural differences. A new need gets a new option, not a new component.
4. **Theme overrides per app**: a theme can change values for one app (for example SoundCraft's red playhead).
5. **One-off app components**: a last resort, built from shared pieces.

Proposed next (not built): an app layer, a small file each app ships that changes named tokens in every theme, with colours given as references to theme roles, so themes, dark mode and high contrast keep working.

## Architecture

| Layer | Crate | Holds |
| --- | --- | --- |
| Tokens | `craft-theme` (no egui) | scales, OKLCH ramps, seeds, derivation, contrast, Size presets, component tokens, theme JSON, 10 built-ins, selection types, `craft-theme check` |
| Store | `craft-theme-store` | theme files, machine-local library, suite `selection.json`, lock and revision protocol, watcher, client |
| Bridge | `craft_ui::theme` | tokens to egui styles; stock egui widgets themed too |
| Behaviour | `craft_ui::behavior`, `craft_ui::edit` | focus, keys, typing + IME, list navigation; one undo step per drag or typing run, Esc restores |
| Components | `craft_ui` | primitives, controls, composites, patterns, shells |

Layer order: tokens, primitives, controls, composites, patterns, shells.

Other crates in the design system:

| Crate | Holds |
| --- | --- |
| `craft-ui` (also) | docking, dialogs, menus, preferences, presets, lists, tab strip, panes, title bar, canvas overlays, fonts, icons, direction, interface scale, `localized` words, `conformance`, `testing`, `story` |
| `craft-ui-gallery` | gallery app (desktop and web) |
| `craft-ui-macros`, `craft-menu`, `craft-keymap`, `craft-window`, `craft-dialogs` | story-arg macros, menu layouts, chords and keymaps, window frame and title strip, native dialogs |
| `craft-design-lint` | `syn` lint plus a clippy.toml template for app UI code |
| `craft-i18n` | embedded catalogs, lookup with fallback, plurals, numbers, language names, locale, search folding, collation |

- Apps ask for meaning (raised surface, primary button, muted text), never a colour or pixel size
- Text dumps of every token and component style are the regression gate

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
- **Per app**: a default (theme pair, mode, size; creative apps ship Dark and Compact, office apps may ship Light) and an optional override in the app's preferences ("Use a different ... in this app"). Interface font, direction and UI scaling are app settings.

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

## Colour, shape, size and type

Colour:

1. Seeds: neutral, accent, good, warn, bad, info
2. Ramps: 12 steps per seed per mode, OKLCH, fixed lightness curve
3. Semantic tokens (92 in the eight families below), mapped to ramp steps by meaning
4. Pins: any token by hand
5. Anchoring: unpinned background greys follow the theme's panel colour
6. Contrast pass: derived colours moved to WCAG 2.2 AA; pins kept and reported (high contrast: everything adjusted)
7. Component tokens from semantic tokens; each pinnable (about 1090 resolved tokens per variant)

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

Accent fills that carry text (menu highlight, primary buttons) use the strong pair.

| Contrast pair | Standard | High |
| --- | --- | --- |
| Text on its surface or fill | 4.5:1 | 7:1 |
| Control cues (focus, check/radio outlines, accent marks, overlays) | 3:1 | 4.5:1 |
| Borders, field edges, hover, pressed, dividers | design target | design target |
| Disabled | exempt | exempt |

Shape and size come from the Size preset, never the theme (except Adwaita, Breeze, Classic, Soft).

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

| Text role (size / line, pt) | Compact | Comfortable |
| --- | --- | --- |
| caption | 10.5 / 14 | 11.5 / 16 |
| label | 11.5 / 16 | 12.5 / 18 |
| body, body strong | 12 / 16 | 13 / 18 |
| title | 13 / 18 | 14 / 20 |
| heading | 15 / 20 | 17 / 24 |
| mono | 11.5 / 16 | 12.5 / 18 |

- Docked and floating panels: square (floating panels add a shadow)
- Edge: flat or bevel; only the Surface primitive reads it
- Elevation: shadow levels for popups, dialogs, floating panels, knobs; none under a bevel
- Faces: the UI face and mono pick is pending. Candidates: Inter, IBM Plex Sans, Source Sans 3 · mono: IBM Plex Mono, Source Code Pro, JetBrains Mono
- Icons: Phosphor is the base set; themes may supply an icon pack. The gallery still defaults to the Lucide candidate. Strokes 1.25 pt sm, 1.5 pt md/lg

## Components

- 195 stories for 90 components in the gallery registry
- Every component: builder API, semantic props only, gallery stories, AccessKit, keyboard behaviour, keyboard-only focus ring, RTL, reduced motion

| Level | Count | Components |
| --- | --- | --- |
| Foundations | 3 | colours, edit, stock egui |
| Specimens | 1 | icons |
| Primitives | 24 | Surface, State layer, Focus ring, Divider, Stack, Row, Interactive, Text, Icon, Handle · canvas overlays: bezier paths, canvas, crop frame, gradient line, grid, guides, marquee, pen draft, reference point, readout, rulers, smart guides, transform box, warp grid |
| Controls | 23 | Button, IconButton, ToggleButton, Segmented, Link · Checkbox, Radio, Switch · TextField, SearchField, NumberField · Slider, RangeSlider, AngleDial · Select, ComboBox · Swatch, Badge, Kbd, Spinner, Progress, Tooltip, ThumbRow |
| Composites | 25 | Field, FormGrid, GroupBox, PropertyRow, SliderRow, SectionHeader, SectionLabel, PanelHeader, PanelFooter · Menu · List, ListRow · Tab, TabStrip · Toast, Notice, EmptyState · Color (picker, area, wheel), Gradient, CurveEditor, Histogram, LevelsEditor · PresetBrowser · TransformFields |
| Patterns | 14 | Panel, Popover, Dialog, Preferences, CommandPalette, Inspector, DockArea, FullWindow, Rail, Toolbar, OptionsBar, StatusBar, TitleBar, Viewport |
| Shells | 1 preset | AppShell with the Canvas preset (`CanvasShell`) |

- Menus: Menu, ContextMenu, MenuBar, PressMenu
- Still absent: Office and Workspace shell presets, Frame preset, timeline, waveform, scopes

## Translation and accessibility

Translation:

- Library words (button labels, dialog text, error text) live as rows in the shared `craft-i18n` catalog (`common.tsv`)
- 41 word tables, about 120 words (approximate; exact count comes from `localized::all()`)
- The app installs its translator once per language change
- Lookup: the language's row, then the English row, then the built-in English. English shows when a row is missing
- Labels the host app sets win and are never replaced; a translation missing a placeholder is ignored
- RTL: chrome mirrors; sliders, number fields and key chords never mirror

Accessibility:

- AccessKit roles on widgets (button, spin button, slider, text input, combo box, list option, radio, dialog, menu item, tab, tree item)
- High contrast: 7:1 text, 4.5:1 control cues, every colour adjusted
- Keyboard: keyboard-only focus ring; behaviour follows the suite's own apps, WAI-ARIA where unspecified; Ctrl+Tab moves a dialog
- Accessibility is a consideration, not a hard rule

## Tests and gates

| Gate | What it holds |
| --- | --- |
| Token dumps | 68 files: 17 theme-mode variants x {standard, high} x {Compact, Comfortable}; rewrite with `UPDATE_DUMPS=1` |
| Style dumps | 17 egui style files, one per theme-mode |
| Component dumps | 70 files |
| Story registry | 195 stories and their default args |
| Title-bar dumps | 2 files |
| Other tests | visual regression, screens, docking workflows, interface scale, font rendering, web fonts |
| Conformance | an app's real appearance setup against the library's reference, across the case matrix; fails on wrong theme, size or text size, a changed `Style`, own zoom or font override; `allow(field, reason)` for exceptions |
| Theme check | `craft-theme check <file>`: contrast report, adjustments, invalid fields |
| Design lint | `craft-design-lint check` (syn) and `check-clippy` |

Design-lint rules: `literal-size/*`, `raw-colour/const`, `style-mutation`, `tooltip/stock`, `clippy/inner-allow`. Exemption marker `// design: allow(reason)`; empty, invalid and unused markers are findings. Baselines and `--deny-unused` supported. Exit 0 clean, 1 findings, 2 error.

## Status

The PhotoCraft port uses the library daily. 14 of its 50 port tasks are merged, and each gap the port finds goes back into the library within hours.

| Area | State |
| --- | --- |
| PhotoCraft on library components | window frame, title bar, menus (native on macOS), start screen, document tabs, status bar, options bar, tool rail, dock, dialog frame, Levels/Curves/Threshold, histograms, Gradient Editor, layer and guide dialogs, fill and stroke dialogs, Layers panel body |
| Still PhotoCraft's own code, restyled by the theme | most panels (Channels, History, Properties, Colour, Swatches, Brushes, Type), most dialogs (New Document, Export, Preferences, Layer Style, filters, Camera Raw), canvas overlays, other adjustment editors |
| macOS | the port builds on macOS; window, title bar and menu bar checks pass |
| Comparison | screenshots of 59 screens, current PhotoCraft beside the port, refreshed after each port batch |

Next: the rest of the PhotoCraft port, then VectorCraft as the second app, then the other apps; upstream PRs once the first app is done.
