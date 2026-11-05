# Migration and landing

How apps move onto the Craft framework, and how the work lands in `craft-libs`. This is a procedure document. The design is in [1](1-overview.md) to [8](8-tests-and-gates.md).

## Contents

1. [Migration strategy](#1-migration-strategy)
2. [Acceptance checklist](#2-acceptance-checklist)
3. [Steps](#3-steps)
4. [Critical path](#4-critical-path)
5. [What each step delivers](#5-what-each-step-delivers)
6. [How each step runs](#6-how-each-step-runs)
7. [Landing strategy](#7-landing-strategy)
8. [Breaking changes](#8-breaking-changes)
9. [Status and open questions](#9-status-and-open-questions)

## 1. Migration strategy

Everything is built in `craft-libs` first: build each feature once, in one place, and use it one way. A feature one app has today is built in the library even when no second app needs it yet. Work goes step by step, so each step ends in a library the apps can use. It does not go family by family.

- **PhotoCraft migrates first.** It has the most UI and features. Its port branch runs on the library while the library grows.
- **VectorCraft migrates second.** It shares the Canvas shell and most of PhotoCraft's skeleton. If the two ports look alike, the library is general enough.
- **Other apps follow**, each adopting the steps it needs.
- **One branch per port.** The port adopts each step as it lands and deletes the code the step replaces. The app's main branch switches once, when the port merges.
- **A gap is library work.** A gap an app finds is fixed in the library first. The port then deletes its own code. No app workaround.
- **Review in small PRs.** Keep hand-written architecture changes in small, independently buildable PRs. Treat generated dumps and icon assets as reviewable data.

## 2. Acceptance checklist

Every app-port step passes all of these before it counts as done:

- [ ] Tests are green.
- [ ] Every story of the step's components is complete ([7.1](7-accessibility-and-translation.md#71-required-for-every-component)).
- [ ] Screenshot comparison of the app's upstream build against the port covers each screen, and every difference a user would notice has a recorded decision. Users see no significant difference, or slightly more polish.
- [ ] Design lint is clean.
- [ ] The conformance test is clean.
- [ ] The app's duplicate implementation is removed. Where the app drew one kind of component several ways, the port uses one.

## 3. Steps

| Step | Delivers | Depends on | Status |
| --- | --- | --- | --- |
| 0 | PhotoCraft adopts the types the library already has; gaps become library work | none | done |
| 1 | Foundations: `ui` feature names, `craft-host`, module moves into family crates, the `craft` facade, gallery and word registry over every crate, one egui revision | 0 | done |
| 2 | Dialog framework: typed results, parameter and command dialogs, size dialogs, prompts, unsaved guard, startup modal queue; text and number control gaps | 1 | built |
| 3 | Shell (dock, panel menus, floating frames, workspaces, start screen, focus regions), keymap completion, canvas core, options bar cells, testing tools | 2 | built |
| 4 | Files, preferences, text and fonts, colour, previews and jobs | 2, 3 | built |
| 5 | Common controls, system services, value formatting, guides and About, align bars | 2 | built |
| 6 | 6a: layers, presets, history, navigator, browsing. 6b: pages, timeline, review, tables | 2, 3, 4 (6a); 2, 3, 5 (6b) | 6a built; 6b planned |
| 7 | Canvas tools and the image-editing workspaces | 3, 4, 6a | in progress |
| 8 | Office-style shell, audio, scripts and code, slideshow, GPU | 2, 3; 6b timeline for audio | planned |

Size: about 185 library tasks, plus about 28 PhotoCraft tasks and about 20 VectorCraft tasks. Step 3 is about 30 library tasks, step 4 about 35, step 6 about 44.

## 4. Critical path

```text
2 --> 3 --> 4 --> 6a --> 7 --> PhotoCraft full port --> VectorCraft full port
       |     |
       |     +--> 5 (off the path: PhotoCraft needs only guides and About)
       +--> 6b, 8 (off the path for PhotoCraft)
```

- Step 2 gates every feature that opens a dialog.
- Step 3 gates the canvas tools (7), the colour droppers and text editor (4), and every panel (6) through the dock's panel-menu host.
- Step 4's colour and font pickers gate the stroke, shape and text work in 7.
- Step 6a gates PhotoCraft's panels and the brush presets in 7.
- Steps 5, 6b and 8 run beside the path when capacity is free. Their inputs are other apps' code.

## 5. What each step delivers

**Step 0: adopt existing types.** PhotoCraft swaps widgets it shares in kind with the library for the library's. Each difference becomes a gap row. The port has recorded 108 gap rows. At least 26 are closed with a merged library change and 34 are scheduled in a later step. They range from a missing option on a slider row to a floating frame with no title-bar move, which is why the shell and common controls come before the tools.

**Step 1: foundations.** No pixel changes.

- The UI feature is `ui` in every crate, plus `screen-testing`.
- `craft-host` carries the services every feature reads.
- Domain code moves out of the kernel into `craft-draw`, `craft-imaging`, `craft-color` and `craft-print`.
- The `craft` facade gives one dependency and one pin.
- The gallery and word registry cover every `ui` crate.
- Each app's `xtask` checks the egui revision.

**Step 2: dialog framework.** A feature dialog is one value an app pushes and one result it reads.

- Delivers: `DialogHost` with typed `DialogResult`, `ParamDialog`, command forms, image and canvas size dialogs, confirmation prompts, `UnsavedChangesGuard`, properties and report dialogs, `PointPicker`, `Remembered<T>`, `craft-update`'s dialogs on the stack.
- PhotoCraft adopts: the stack for every dialog, parameter dialogs for filters, size dialogs, prompts, the unsaved guard, remembered options.
- PhotoCraft deletes: its dialog host and routing, parameter-dialog builder, field map, size-dialog code and prompt helpers.

**Step 3: shell, testing tools, canvas core, options bar.**

- Delivers: dock host and commands, panel menus, floating frames, Window menu and Arrange, named workspaces, start screen, region focus, shortcuts dialog, the keymap catalog as source of truth, zoom and navigation, cursors, snapping, raster display, the on-canvas tool host, options bar cells with scrubbing, `craft-automation`, the render harness.
- PhotoCraft adopts: the library dock, workspaces, `ScreenMode`, the shortcut catalog and dialog, canvas navigation, the tool host, options bar cells, automation.
- PhotoCraft deletes: its dock, panel-menu popups, floating card frames, shortcuts table, zoom ladder, cursor host, momentary-key and snapping code, options bar cell code, screenshot queue and control channel.
- VectorCraft maps to library components during this step. Its gaps feed the specs of steps 4 to 7.

**Step 4: files, preferences, text and fonts, colour, previews.**

- Delivers: the file service, New Document, export dialogs, print, file intake; preferences over a settings document with `AppearancePage` and recovery; font picker, canvas text editor, style panels; colour picker dialog, droppers, samplers, screen picker, swatch panel, palettes, CMS additions; preview panes and job progress.
- PhotoCraft deletes: its file dialogs, most preferences code, recovery and path code, family picker, colour panel and picker bodies, job UI.
- VectorCraft adopts steps 1 to 4 on its port branch in one pass.

**Step 5: common controls, system services, value formatting, guides, About.** A small step for PhotoCraft.

- Delivers: sortable tables, chips, date and tree pickers, linked fields, mixed values, split button, breadcrumb, rating; clipboard, file watcher, default-app registration, temp files, Save flow; dates, byte sizes and elision; New Guide and Guide Layout dialogs, About, align bars.
- PhotoCraft adopts guides, About, align bars, linked size fields, clipboard and the Save flow.

**Step 6: layers, presets, history, browsing (6a); pages, timeline, review, tables (6b).** 6a is on PhotoCraft's path. 6b serves other apps.

- 6a delivers: `LayersPanel` over a `LayerTree` trait, channels, the preset model and menus, a history panel with one-edit steps and snapshots, navigator, scopes, actions, thumbnails, browsing. PhotoCraft's panels then all run in the library dock; it deletes its layer, history, navigator and histogram panels and its preset model.
- 6b delivers: paged documents, timeline, comments and versions, spreadsheet tables. VectorCraft adopts layers and presets after 6a, and artboard lists after 6b.

**Step 7: canvas tools and image-editing workspaces.** PhotoCraft's last large deletion.

- Delivers: transform, warp, selection and mask tools, vector and shape tools, crop, measure, brush, and the full-window editors (select and mask, liquify, filter gallery, fill from surroundings, perspective plane, wide-angle correction, layer style, adjustments).
- PhotoCraft adopts every tool through the library tool host. Its port then runs the lint clean and passes conformance and the render matrix.
- VectorCraft adopts the draw family and finishes its port.

**Step 8: office-style shell, audio, scripts, slideshow, GPU.** Mostly other apps' code.

- Delivers: ribbon, backstage view, keytips and the office-style preset; audio engine and mixer UI; code editor and console; slideshow presenter; `craft-gpu`.
- PhotoCraft only moves its GPU display onto `craft-gpu`. Nothing visible changes.

## 6. How each step runs

1. **Spec.** A written spec is reviewed before code. Open questions are settled in one round.
2. **Parallel tracks.** Independent tasks, one branch each, narrow checks only.
3. **Integration.** Branches merge into one and the dumps are regenerated.
4. **One full test run**, then a review of each track.
5. **Fixes**, then a final full run.
6. **PhotoCraft adoption.** The port branch repins to the library and replaces its code. The screenshot comparison runs at each adoption task.

Each step also adds its families to the library docs: a page per family with the out-of-the-box recipe (the `Host`, the trait the app implements, the builder call, the `Outcome`) and a migration table from old path to new path.

## 7. Landing strategy

Measured on `design-system` against `upstream/main` (c39e435c):

| Measure | Value |
| --- | --- |
| Commits | 1,107 |
| Files changed | 1,547 |
| Lines added | about 332,000 |
| Lines removed | about 5,000 |
| Generated test dumps among the added lines (`*/tests/dumps/*`) | about 105,000 |
| Icon set files | 540 (four sets) |

The work is mostly additive. A review reads dumps and icons as data.

It lands as a series of upstream pull requests, one at a time, bottom-up. Each PR builds and passes CI alone. Apps pin `craft-libs` by commit, so a merged PR changes nothing in an app until the app moves its pin.

| Order | PR | Contents |
| --- | --- | --- |
| 1 | egui patches | The patches go to the storytold egui fork; craft-libs main moves its egui pin. Everything else depends on this |
| 2 | Icon sets | Data only |
| 3 | Token model and themes | `craft-theme`, `craft-theme-store`, `craft-color`. New crates; nothing depends on them yet |
| 4 | `craft-ui` core | Theme bridge, primitives, controls, edit lifecycle, behaviour |
| 5 | Composites, patterns, shells | With `craft-draw` and `craft-imaging` |
| 6 | Host, facade and changes to existing crates | `craft-host`, `craft`, and changes in `craft-menu`, `craft-keymap`, `craft-window` and `craft-platform` |
| 7 | Tooling | The gallery, `craft-design-lint`, conformance |
| 8 | Apps | PhotoCraft, then VectorCraft, one PR each, then the other apps |

The egui patches are seven numbered changes (E22 to E28) on top of the fork's earlier patches:

- the IME composition's pre-edit range on `TextEdit` state
- a public accessibility description for `Response`
- scroll-bar labels
- an AccessKit focus report that leaves egui focus alone
- a pointer press that goes to the widget under the press position
- `DragAndDrop::aborted_by_escape`
- `Popup::swallow_outside_press`, so the press that closes a popup does nothing else

**Timing.** A layer lands once PhotoCraft's port has used it in earnest. Bottom layers land first. Landing early stops drift from upstream. A landed layer's API is shared, and changes to it need the same deprecation care.

**Staying current.** `design-system` merges upstream main daily. After each landed PR, the remaining work rebases onto main. A merge whose conflicts are only lockfile and version pins needs no local build; CI checks it.

## 8. Breaking changes

About 100 public items in `craft-ui`, `craft-menu`, `craft-keymap`, `craft-window` and `craft-platform` are removed or renamed (106 on the current branch). The count is an upper bound: it counts every removed `pub` item, including items that moved to another crate.

Each PR that removes or renames a public item keeps the old name as a deprecated alias or shim until every app has moved past it, so an app's next pin bump still builds. A migration table from old path to new path goes with each PR. The old APIs apps use today (the icon-button and combo-box modules, the old rail and number-field types) stay `#[deprecated]` until the last app that uses them has migrated.

## 9. Status and open questions

### What exists

| Area | State |
| --- | --- |
| Token model, ten built-in themes, store, bridge | built |
| Primitives, controls, composites, patterns | built |
| Canvas shell, docking with iconic column and flyouts, title bar | built |
| Dialog frame, prompts, full-window dialogs, preferences window, typed dialog stack | built |
| `craft-color`, `craft-draw`, `craft-imaging`, `craft-layers`, `craft-browse`, `craft-automation` | built (parts) |
| `craft-host`, the `craft` facade, one egui revision check | built |
| Gallery with Components and Theme workspaces, Variants, app preview | built |
| Design lint, conformance test, `craft-theme check` | built |
| PhotoCraft on library components | window frame, title bar, menus (native on macOS), start screen, document tabs, status bar, options bar, tool rail, dock, dialog frame, Levels, Curves and Threshold, histograms, gradient editor, layer and guide dialogs, fill and stroke dialogs, Layers panel body |
| Still PhotoCraft's own code, restyled by the theme | most panels, most dialogs, canvas overlays, other adjustment editors |
| macOS | The port builds on macOS; window, title bar and menu bar checks pass |
| Comparison | Screenshots of 59 screens, current PhotoCraft beside the port, refreshed after each port batch |

### What is next

1. Finish step 7, completing PhotoCraft's port.
2. VectorCraft's port, then the other apps.
3. The upstream PR series, starting with the egui patches.

### Questions for the team

| Topic | Question |
| --- | --- |
| Landing order | Is the bottom-up order right, or should the token model and `craft-ui` core land together? Should the apps' PRs follow each layer, or wait for the whole series? |
| Deprecation window | How long do deprecated aliases stay: until every app has moved past them (the plan), or a fixed number of releases? |
| egui patches | Which of the seven patches go to upstream egui, and which stay in the storytold fork? |
| Facade name | `craft` is the facade's name. If it clashes with an app crate, the alternative is `craft-kit` |
| Crate granularity | The layout adds twenty crates to upstream's twenty-six. Is there a family that should merge, or a crate that should split? |
| Review load | Is a series of about 330,000 added lines, mostly generated or additive, reviewable as laid out, or should it split further? |
