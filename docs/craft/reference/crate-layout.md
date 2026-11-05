# Crate and module layout

Maintainer reference. The concepts are in [3. Architecture](../3-architecture.md). Crates are alphabetical, and modules are alphabetical inside each crate. The State column says whether a module exists in the repository or is planned, and in which migration step (see [Migration and landing](../migration-and-landing.md)). A module marked "exists" may gain features in later steps; the note says so.

Conventions:

- Crates are `craft-<family>`, one word where possible. No `-ui` or `-core` suffix.
- Modules are nouns, one level below the crate root. `ui` is the only module that nests a second level (`craft_pages::ui::table`).
- Types carry their full name (`LayersPanel`, not `layers::Panel`), so they read the same wherever they are imported.
- In a family crate, a module `ui::x` is the egui side and a bare `x` is core.

Twenty crates are new relative to upstream. Thirteen exist on the `design-system` branch: `craft`, `craft-automation`, `craft-browse`, `craft-color`, `craft-design-lint`, `craft-draw`, `craft-host`, `craft-imaging`, `craft-layers`, `craft-theme`, `craft-theme-store`, `craft-ui-gallery`, `craft-ui-macros`. Seven are planned: `craft-audio`, `craft-code`, `craft-gpu`, `craft-pages`, `craft-review`, `craft-table`, `craft-timeline`.

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

Control channel, screenshot queue, profiler and recorded actions for tests and scripting. Native only. New on the branch.

| Module | Holds | State |
| --- | --- | --- |
| `actions`, `ui::actions` | Recorded actions and the actions list | exists |
| `control` | Automation control channel | exists |
| `profiler` | Frame profiler | exists |
| `screenshot` | Screenshot request queue | exists |
| `trace` | Frame timing trace | exists |

### craft-browse

Thumbnails, media bin, tile grid, places and in-app file browsing. New on the branch.

| Module | Holds | State |
| --- | --- | --- |
| `browser`, `ui::browser` | In-app file browser | planned, step 4 |
| `places`, `ui::places` | Folder places tree | exists |
| `thumbs` | Thumbnail loader and cache | exists |
| `ui::bin` | Media bin browser | exists |
| `ui::compare` | Image compare tile | exists |
| `ui::grid` | Thumbnail tile grid | exists |
| `ui::relink` | Linked files and relink dialog | exists |

### craft-cms

Colour management: ICC v2 and v4 profiles, transforms, rendering intents. No egui.

| Module | Holds | State |
| --- | --- | --- |
| `builtin`, `synth` | Built-in and synthetic CMYK profiles | exists |
| `clut`, `curve`, `lut3d`, `lutfile` | Lookup tables, tone curves, display LUTs, creative LUT files | exists |
| `convert` | CMYK models and gamut conversions the colour dialogs need | exists |
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
| `display` | Display transform on `craft-cms` | exists |
| `gray` | Grey level of a colour | exists |
| `harmony` | Colour harmony and saved themes | exists |
| `hex` | Hex text for colours | exists |
| `mode` | What a colour area and its strip show | exists |
| `palette` | Palette data | exists |
| `ui::area`, `ui::strip` | 2D colour area and its strip | exists |
| `ui::choice` | Colour choice: named colours plus Custom | exists |
| `ui::disc`, `ui::wheel`, `ui::ramp` | Colour disc, wheel and ramp | exists |
| `ui::droppers`, `ui::eyedropper`, `ui::samplers`, `ui::screen_picker` | Dialog droppers, eyedropper tool, sample points, screen picker | exists |
| `ui::harmony` | Harmony wheel | exists |
| `ui::hex`, `ui::readout` | Hex field and channel readout | exists |
| `ui::lut_picker` | LUT picker | exists |
| `ui::paint_chip`, `ui::paints` | Paint chip and paint list | planned, step 4 |
| `ui::pair` | Foreground and background swatch pair | exists |
| `ui::palette` | Swatch palette | exists |
| `ui::panel` | Colour panel modes and menus | exists |
| `ui::picker`, `ui::popover` | The colour picker and its popover | exists |
| `ui::picker_dialog` | The complete colour picker dialog | exists |
| `ui::profile_prompts` | Profile mismatch prompts | exists |
| `ui::swatches` | Swatch panel | exists |
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
| `save` | Save and Save As flow with guards | exists |
| `service` | File dialog service behind the host's files | exists |
| `ui::file_list`, `ui::path_field` | File list picker; path field with Browse | exists |

### craft-docpath

Decides whether a stored path or URL refers to a local file. Exists.

| Module | Holds | State |
| --- | --- | --- |
| `links` | Linked files, for relink | exists |
| `names` | Safe file names | exists |

### craft-draw

Vector and shape tools, pen, stroke, gradient, crop, measure, symmetry, brush presets and free transform. UI-only family. New on the branch (step 1).

| Module | Holds | State |
| --- | --- | --- |
| `ui::add_beside`, `ui::artboard` | Add-beside buttons; artboard tool, bar and properties | exists |
| `ui::brush_presets`, `ui::brush_settings` | Brush preset picker, cards and chip; brush settings panel | exists |
| `ui::connector` | Connector tool, overlay and bar | exists |
| `ui::corner` | Corner radius widgets | exists |
| `ui::crop` | Crop frame and crop tool | exists |
| `ui::gradient` | Gradient swatch, editor, palette and on-canvas gradient line | exists |
| `ui::measure` | Ruler tool, bar and box distances | exists |
| `ui::pattern`, `ui::picture_fill` | Pattern fill picker; picture fill options | exists |
| `ui::pen` | Pen draft and bezier paths | exists |
| `ui::shape`, `ui::shape_drag` | Shape properties panel and options bar; shape drag tool and Create Shape dialog | exists |
| `ui::slice` | Slice tool, overlay, bar and dialogs | exists |
| `ui::stroke` | Stroke editor, button and preview | exists |
| `ui::symmetry` | Paint symmetry guides, menu and transform | exists |
| `ui::transform` | Free transform box and transform fields | exists |

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
| `axes` | Variable font axes | exists |
| `filter` | Search by family, category, language, script | exists |
| `glyphs` | Glyphs panel | exists |
| `index` | The font indexes | exists |
| `installed` | Installed fonts grouped by family | exists |
| `missing` | Document fonts and missing-fonts dialog | exists |
| `panel`, `ui` | The egui `FontsPanel` and its story | exists |
| `picker` | Font picker: families, filters, previews, stars, recents, similar fonts (feature `font-picker`) | exists |
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
| `layer_style` | Layer style blending model | exists |
| `selection` | Selection model | exists |
| `trace` | Mask outline tracing | exists |
| `ui::adjust` | Levels editor and the colour adjustment editors | exists |
| `ui::live_preview` | Live preview of an adjustment: keys, lanes, proxy plan |
| `ui::blend_keys` | Blend-mode key picker | exists |
| `ui::blur_pins` | Blur pins on the canvas | exists |
| `ui::channels` | Channels panel | exists |
| `ui::fill` | Fill-from-surroundings workspace | exists |
| `ui::filter_gallery` | Filter gallery workspace | exists |
| `ui::layer_style` | Layer style page and effect list | exists |
| `ui::liquify` | Push-and-warp (liquify) workspace | exists |
| `ui::mask_view` | Mask and selection overlay views | exists |
| `ui::scopes` | Scope panel: histogram, vectorscope, clipping | exists |
| `ui::select`, `ui::select_mask`, `ui::selection` | Outline selection tools; select-and-mask workspace; ants, matte and quick-mask overlay | exists |
| `ui::transform_preview` | Live transform preview | exists |
| `ui::vanishing_point`, `ui::wide_angle` | Perspective plane and wide-angle correction workspaces | exists |
| `ui::warp` | Warp grid; mesh, pin and perspective warp | exists |

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
| `ui::import_offer` | First-launch import offer | exists |
| `ui::router` | Focus gate and key owners | exists |

### craft-layers

Layers panel and its parts. UI-only family. New on the branch.

| Module | Holds | State |
| --- | --- | --- |
| `ui::filter` | Layer filter bar | exists |
| `ui::options` | Layers panel options dialog | exists |
| `ui::panel` | `LayersPanel` over a `LayerTree` trait | exists |
| `ui::rows` | Blend, opacity and lock rows | exists |
| `ui::thumbs` | Layer and mask thumbnails | exists |

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
| `context` | Panel and context menus from command entries | exists |
| `driver` | Events from the platform menu in, menu bars out | exists |
| `help`, `language` | Help menu kit; language menu | exists |
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
| `clipboard` | Clipboard with synthetic input and size limits | exists |
| `clock` | Injectable clock | exists |
| `default_app` | Default app registration | exists |
| `dialog_outcome`, `dialogs` | Dialog results with evidence; picker creation on the UI thread | exists |
| `job` | Background jobs and preview jobs | planned, step 4 |
| `logs` | Crash and command logs | exists |
| `open` | OS open requests, drops, open and reveal | exists |
| `pacing` | Frame pacing and work budgeting | exists |
| `pen` | Pen input: pressure, tilt, eraser end | planned, step 3 |
| `process` | Bounded output capture for one-shot programs | exists |
| `screen` | Screen colour sampling | exists |
| `session` | Reopen last session | exists |
| `temp` | Private temp files | exists |
| `usage` | Resource usage readout | exists |
| `watch` | File change watcher | exists |

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
| `profiles` | Settings profiles | exists |
| `recovery` | Keeping damaged and previous files | exists |
| `remember` | `Remembered<T>`: last-used options for dialogs, panels and tools | exists |
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
| `quotes` | Smart quotes | exists |
| `segment` | Line-break opportunities | exists |
| `spell` | Spelling check | exists |
| `styles` | Named style list | planned, step 6 |
| `ui::character`, `ui::style_panel` | Character format controls; text style panel | exists |
| `ui::columns`, `ui::hyperlink`, `ui::notes`, `ui::paragraph` | Columns dialog, hyperlink dialog, rich-text notes, paragraph format dialog | planned, step 6 |
| `ui::editor` | Canvas text editor | exists |
| `ui::find`, `ui::styles` | Find bar; named styles panel | planned, step 6 |
| `ui::ruler`, `ui::tabs_dialog` | Text ruler with tab stops; tab stops dialog | planned, step 6 |
| `ui::spelling` | Spelling dialog | exists |

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
| `motion` | Animated caret and selection outline; reduced motion | exists |
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
