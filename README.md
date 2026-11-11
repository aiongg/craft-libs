Craft App Libraries
===================

<a href="https://sonarcloud.io/project/overview?id=storytold_craft-libs"><img alt="Scanned by SonarCloud" src="https://img.shields.io/badge/Scanned_by-SonarCloud-4b9fd5?style=flat-square&logo=sonarqubecloud&logoColor=white"></a>

This repo contains libraries used by the various Craft Apps. It's a Rust monorepo workspace, and the libraries can be included by any Craft Apps that need them.

In the future this may grow to include non-Rust library code.

If any libraries grow beyond Craft App usage, eg. RAW format support, they may be promoted out and into their own libraries for the broader ecosystem to take advantage of.


## Layout

A Cargo workspace (edition 2024, resolver 3). Each library is a crate under `crates/<name>`, named
`craft-<name>`, with `version`, `edition`, `license` and `repository` inherited from the workspace
(`license.workspace = true`, and so on) and `[lints] workspace = true`. Register each crate in
`[workspace.dependencies]` in the root `Cargo.toml`.

Apps depend on a crate by git:

```toml
craft-<name> = { git = "https://github.com/storytold/craft-libs", rev = "<commit>" }
```

## Libraries

- [`craft-platform`](crates/platform/README.md): optional native dialog lifecycle, Linux/BSD desktop appearance observation and browser download ownership; application policy stays in the apps.

- [`craft-ai-core`](crates/ai-core/README.md): the data model and contract for AI in the Craft Apps (connections, models, tools, requests, jobs, consent, history, route spec). Pure, no I/O, wasm-safe. Contract in [docs/ai-shared-layer.md](docs/ai-shared-layer.md).
- [`craft-pdf`](crates/pdf/README.md): independently optional PDF parsing, interpretation, rasterization and guarded authoring. Apps retain layout, fonts, editing and export policy; writer-only consumers do not enable the shared reader.

- [`craft-automation`](crates/automation/README.md): drive a running app from tests, agents and the CLI: control channel, screenshot queue, frame profiler and trace (native only); recorded actions, their playback and the Actions panel (every target).
- [`craft-atomic`](crates/atomic/README.md): Crash-safe atomic file writing and replacement with same-directory staging, directory fsync, Apple `F_FULLFSYNC` fallback, transient lock backoff, and fault injection for the Craft suite.
- [`craft-browse`](crates/browse/README.md): browsing: standard folders, volumes and lazy folder
  listings, linked-file statuses kept current, file thumbnails decoded by the app in jobs, and
  (`ui` feature) the tile grid, media bin, places tree, compare tile, links list and relink dialog.
- [`craft-cms`](crates/cms/README.md): pure-Rust ICC v2/v4 colour management: profile parsing
  and writing, transforms, rendering intents, black point compensation, soft proofing, gamut
  checks, 3D LUTs and built-in profiles (sRGB, Display P3, synthetic coated CMYK and others).
- [`craft-color`](crates/color/README.md): colour picker, wells, swatch tables, ramps and the hex
  field; the Color Picker dialog, droppers, eyedropper, samplers, screen picker, Color and Swatches
  panels (`ui` feature); swatch files, the display transform and harmonies.
- [`craft-crw`](crates/crw/README.md): clean-room Canon CRW (CIFF) raw decoder, from LightCraft:
  the heap container, the entropy coding derived by black-box analysis (frozen code tables), sensor
  black and white levels and as-shot white balance. The complete derivation record (plan, sources
  log, scripts, verification) is in `crates/crw/clean-room/`. Colour stays in each app.
- [`craft-dialogs`](crates/dialogs/README.md): native file and message dialogs that never block
  the event loop: queued requests shown at the end of the frame and answered on a later frame,
  free-standing Open and Folder panels and Save sheets on macOS, case-insensitive filters. What
  to do with the answer stays in each app.
- [`craft-doc`](crates/doc/README.md): reads PhotoCraft `.pcraft`, VectorCraft `.vectorcraft`
  and EffectCraft `.ecproj` documents without the apps: the kind and version, the previews and
  PDF they carry, and their pages and layers. Rendering and editing stay in each app.
- [`craft-docpath`](crates/docpath/src/lib.rs): decides whether a path or `file:` URL stored in a
  document may be opened. Paths stored in documents are followed only when they are local or on the
  document's own network share; other network paths, device paths and non-file URLs are refused.
  `links` keeps a document's linked files: their status, the identity check, folder remaps and
  folder searches.
- [`craft-draw`](crates/draw/README.md): vector and shape tools; the gradient editor and
  on-canvas gradient overlay, the pen and path overlays, the crop frame and perspective crop, and the
  free transform box and fields (`ui` feature).
- [`craft-ecproj-format`](crates/ecproj-format/README.md): the envelope of EffectCraft `.ecproj`
  project files: `savedBy`, the schema version and its check, and the file object with the
  project and its saved view as type parameters. The project model stays in EffectCraft.
- [`craft-imaging`](crates/imaging/README.md): raster image editing; the scope counting (every
  target) and, with the `ui` feature, the warp grid, the levels editor, the Channels panel and the
  scope panel with its clipping overlay.
- [`craft-font-catalog`](crates/font-catalog/README.md): the installed fonts on every platform
  (font folders, fontconfig and Flatpak host folders, CoreText, DirectWrite, the font store),
  lookup by family, style, localized, legacy, full and PostScript names, family loads shared
  between threads, and rescans when fonts are installed or removed.
- [`craft-font-manager`](crates/font-manager/README.md): Craft Font Manager as a library: search
  and filters over the Google Fonts index, installed fonts by family, the font store interface,
  and (`ui` feature) `FontsPanel`, the whole interface as one panel for the launcher's Fonts tab
  and the standalone app.
- [`craft-font-store`](crates/font-store/README.md): the suite's font store: the pinned Google
  Fonts and Noto indexes plus indexes the user adds (never curated), exact lookup by family and
  style or PostScript name, and installs from each source's URLs (jsDelivr, then raw GitHub)
  checked by size, sha256 and parsing, with the licence, a manifest, and removal. Off until the
  user turns downloading on; agents can't.
- [`craft-fonts-build`](crates/fonts-build/README.md): embeds the optional craft-fonts faces
  from an app's `build.rs` (`CRAFT_FONTS_DIR`, `CRAFT_FONTS_REQUIRED`, script filters) as a table
  of `EmbeddedFace`s. No dependencies; the app picks the faces.
- [`craft-i18n`](crates/i18n/README.md): translations over the embedded craft-i18n catalogs:
  lookup with fallback (`pt-br` → `pt` → English), plurals, language names, the system locale and
  a pseudo locale. The catalogs stay a fully optional build input; apps keep `tl!` and their
  language setting.
- [`craft-jpeg`](crates/jpeg/README.md): clean-room decoder for 12-bit and arithmetic-coded
  JPEG (T.81 SOF0/SOF1 at 12 bits, SOF9, SOF10), from PhotoCraft, and a frame probe for routing.
  Metadata stays in each app.
- [`craft-keymap`](crates/keymap/README.md): keyboard bindings: chords and their text forms,
  scopes, conflicts, user overrides stored through craft-settings and import of other
  applications' shortcut sets. Command catalogs, file readers and the shortcut editor UI stay
  in each app.
- [`craft-layers`](crates/layers/README.md): the Layers panel over the app's `LayerTree`: rows
  with thumbnails and masks, blend, opacity, lock and fill rows, the filter bar, Layers Panel
  Options, drag and drop, and events the app applies with its own commands and undo. UI-only.
- [`craft-audio`](crates/audio/README.md): audio file I/O and codecs (WAV, AIFF, FLAC), streaming
  and seeking, and packet decoding for any demuxer. See its README for formats and features.
- [`craft-media`](crates/media/README.md): bit-level I/O (`bitstream`), clean-room MP4/MOV
  (`isobmff`) and Matroska/WebM (`matroska`) demuxers and muxers, an Ogg demuxer (`ogg`), from
  FilmCraft, and an ISO BMFF box reader. Containers only; codecs are in `craft-audio`, and
  video codecs will go in a separate crate.
- [`craft-menu`](crates/menu/README.md): an app's menus as a macOS menu bar: the Mac layout
  (app menu, Window, Help, system-key clashes), a platform backend trait, and key equivalents
  fed back to egui as the input egui-winit would have made. Commands and their enabled state
  stay in each app.
- [`craft-pages`](crates/pages/README.md): pages for the document apps: page layout and fit modes,
  paper sizes and page setup, page ranges and numbering; with `ui`, the page view, thumbnails, Page
  Setup, margin handles, header and footer zones, text selection, reading view, page list and
  document tables. The document, its layout and rendering stay in each app.
- [`craft-pcraft-format`](crates/pcraft-format/README.md): the envelope of PhotoCraft's `.pcraft`
  bundle: `Manifest<D>` with the document as a type parameter, `TileRef`, the entry paths, the
  manifest decode limits and the migration runner. The document model stays in PhotoCraft.
- [`craft-print`](crates/print/README.md): PDF submission to CUPS and Windows, printer discovery,
  duplex/copies/colour and driver preferences. PDF generation stays in each app.
- [`craft-proof`](crates/proof/README.md): proofing per language (spelling, suggestions, grammar,
  hyphenation, a Hunspell reader, user words, Language packs). The word lists and patterns are
  craft-i18n's `language-data/`, embedded at build time from `CRAFT_I18N_DIR`: English and the
  carry-over locales by default, the set `CRAFT_PROOF_EMBED` names otherwise (`en` for English
  only); other languages come from installed Language packs.

- [`craft-quit`](crates/quit/src/lib.rs): native macOS quit, egui close handling and an optional
  app menu. Unsaved-document decisions and save prompts stay in each app.

- [`craft-review`](crates/review/README.md): review: the comments panel, comment pins, thread
  popover and composer, Acrobat's annotation tool, the signature pad, version history and the
  revisions panel, presence, voting, cursor chat and join over the app's own transport (`ui`
  feature).
- [`craft-settings`](crates/settings/README.md): `config.toml` settings storage for every app: platform folders, portable mode and overrides, atomic saves while running, backup, damaged-file recovery and schema migrations, plus remembered dialog options and preset folders (also on wasm32). Policy in [docs/settings-storage.md](docs/settings-storage.md).
- [`craft-table`](crates/table/README.md): spreadsheet widgets: the cell grid with Excel's
  keys, outline bands, the PivotTable field list, the condition builder, chart painting, slicers,
  reference picking, selection statistics, error tags and filter buttons (`ui` feature).
- [`craft-text`](crates/text/README.md): text processing below the composer. Its `hyphen` module finds
  hyphenation points, with a bundled US English dictionary and patterns and optional hypher patterns
  for 47 more languages (one cargo feature each). With `ui`, the canvas text editor, Character and
  Paragraph controls, text style panels and Check Spelling dialog, and the paged-text controls
  (ruler, Tabs, Paragraph, Columns and Hyperlink dialogs, notes editor, find); `spell` and `find`
  (regex) are features.
- [`craft-timeline`](crates/timeline/README.md): timelines: exact media time, frame rates,
  timecode, the playback clock, a frame cache and frame server (egui-free); the timeline panel,
  ruler, clips, keyframe lanes, graph editor, transport, timecode fields, preview, overview and
  render queue (`ui` feature).
- [`craft-vectorcraft-format`](crates/vectorcraft-format/README.md): the envelope of VectorCraft's
  `.vectorcraft` file: its members, versions, embedded images and the data after the JSON, with the
  document as a type parameter. The document and its migrations stay in VectorCraft.
- [`craft-web-api`](crates/web-api/README.md): the page command API every Craft web build offers its page — one
  `window.<app>` shape (`request`, `execute`, `commands`, `docInspect`, `uiInspect`, `screenshot`) over the app's
  JSON control channel.
- [`craft-web-host`](crates/web-host/README.md): the host messages between a Craft app's web build and the page that hosts it (`?host=parent` or `data-host="page"`): one contract and one bridge for every app.

## Craft framework

The shared UI layer of the Craft apps: the design system, the UI library on egui, the host convention and the
feature families above. Assets, localization and commands stay in each app. Requires Rust 1.95 or later. Design and
architecture: [docs/craft-framework.md](docs/craft-framework.md).

- [`craft`](crates/craft/README.md): the facade: one dependency, one pin and one import root for an app, with a
  feature per crate.
- `craft-ui`: the UI library's facade, which re-exports its six crates under one root (`craft_ui::…`):
  - `craft-ui-contract`: runtime, style, fonts, primitives, the component contract, layout and the test kit;
  - `craft-ui-widgets`: the components (buttons, choices, fields, lists, menus, pickers, tabs, tables, bars);
  - `craft-ui-panels`: sections, dialogs, docking and rails;
  - `craft-ui-canvas`: the canvas, its tools and controls, and the full-window dialogs;
  - `craft-ui-patterns`: ready-made dialogs, presets, preferences and dock areas;
  - `craft-ui-shell`: the app shell, the title bar and the eframe entry point.
- `craft-ui-macros`: the derives and macros of the library (`Component`, `Props`, `ComponentStyle`, `Pattern`,
  `CommandSet`, words, stories and trials); use them through `craft-ui`.
- `craft-behavior`: the behaviour cores with no egui: press lifecycle, value drags, navigation, item layouts, edit
  events and the dock model.
- [`craft-theme`](crates/theme/README.md) and [`craft-theme-store`](crates/theme-store/README.md): the token model,
  the built-in themes and the suite's theme selection ([docs/suite-themes.md](docs/suite-themes.md)).
- [`craft-host`](crates/host/README.md): the services an app installs once and every feature reads.
- `craft-win32`: the Win32 calls that need `unsafe`, behind a safe API; the one crate that allows `unsafe`.
- Tools, not shipped in apps: `craft-ui-gallery` (the story viewer), `story-render` (one component's stories to
  PNG files), [`craft-design-lint`](crates/design-lint/README.md), `craft-ui-bench`, `craft-ui-test-kit`.

Framework docs:

- [Craft framework](docs/craft-framework.md): the design, installation, architecture, design system, foundations,
  components, accessibility, tests, and the migration and landing plan.
- [Development workflow](docs/development.md): the `cl` commands, the one fixed feature set, what not to run
  locally, CI coverage, the profiles, the job pool and what a new worktree's first build costs.
- [Writing a component](docs/authoring.md), the [component index](docs/components-index.md), the
  [contract API](docs/contract-api.md) and the [glossary](docs/glossary.md).
- [Docking](docs/ui-docking.md): the dock model, `DockArea`, `SplitArea` and `PaneArea`.
- [Dialogs](docs/ui-dialogs.md): `Dialog`, `DialogHost`, the dialog stack and the standard prompts.

## License and credits

craft-libs is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Copyright (c) 2026 ArtCraft Team and the craft-libs contributors. Required notices are in [NOTICE](NOTICE).

Bundled fonts, icons, images and other assets keep their own open licenses; each one is listed
with its author, source and license in [ATTRIBUTION.md](ATTRIBUTION.md).
