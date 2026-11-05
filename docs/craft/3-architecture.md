# 3. Architecture

## 3.1 Crate tiers

Tiers describe dependency direction. A crate depends only on lower tiers. They are separate from the design-system layers in [4.1](4-design-system.md#41-component-layers).

```text
Tier 3  craft              facade: re-exports and forwards features
          |
Tier 2  families           craft-color, craft-draw, craft-layers, ...
          |
Tier 1  craft-ui           kernel: tokens bridge, components, shells, dialogs, docking
          |
Tier 0  engines, services  craft-theme, craft-i18n, craft-settings, craft-cms, craft-host, ...
```

Rules: tier 0 has no egui except `ecolor`. `craft-ui` names no document concept and never depends on a family. A family keeps core types at the crate root and egui UI in `ui` behind the `ui` feature, and may be UI-only. No family depends on another family's `ui`; a widget two families share moves to `craft-ui`.

## 3.2 The `craft` facade

- Every public UI type is re-exported at `<crate>::ui`; every core type at the crate root: `craft::color::ui::ColorPicker`, `craft::color::Palette`.
- `craft::prelude` holds `AppInfo`, `Host`, `Section`, `Tokens`, `EditEvent`, `DialogStack` and `Outcome`. There are no other preludes.
- `craft::story::all()` and `craft::words::all()` collect the stories and word tables of every enabled crate. A feature can move between crates as long as `craft::<family>` stays.

Feature names: `ui` is the only UI feature name in every crate, and `screen-testing` adds stories and snapshot tests. The full table is in [reference/crate-layout.md](reference/crate-layout.md#facade-features).

Native-only modules use `#[cfg(not(target_arch = "wasm32"))]`. A crate that cannot build for wasm says so in its first doc line, and the facade gates it.

## 3.3 Crate taxonomy

| Kind | Role | Crates |
| --- | --- | --- |
| Facade | One dependency for apps | `craft` |
| Kernel | Shared UI infrastructure | `craft-ui`, `craft-ui-macros` |
| Domain families | Features with core logic and egui UI | `craft-color`, `craft-draw`, `craft-imaging`, `craft-layers`, `craft-browse`, `craft-text`, `craft-font-manager`, `craft-pages`, `craft-review`, `craft-table`, `craft-timeline`; planned: `craft-audio`, `craft-code` |
| Infrastructure services | Egui-free logic that families and apps share | `craft-theme`, `craft-theme-store`, `craft-host`, `craft-i18n`, `craft-settings`, `craft-keymap`, `craft-menu`, `craft-automation`, `craft-update`, `craft-cms`, `craft-ai-core`, `craft-atomic`, `craft-docpath`, `craft-font-catalog`, `craft-font-store`, `craft-pdf`, `craft-prepress` |
| Formats and codecs | Clean-room readers and writers | `craft-crw`, `craft-jpeg`, `craft-media`, `craft-ecproj-format`, `craft-vectorcraft-format` |
| Platform adapters | One operating-system or window-system service each | `craft-platform`, `craft-dialogs`, `craft-window`, `craft-quit`, `craft-print`; next: `craft-gpu` |
| Tooling | Not shipped in apps | `craft-ui-gallery`, `craft-design-lint`, `craft-plate-test`, `craft-fonts-build` |

Services and adapters may carry a `ui` feature (`craft-keymap`, `craft-update`, `craft-dialogs`) but name no document concept. Module tables: [reference/crate-layout.md](reference/crate-layout.md).

## 3.4 The host convention: services in, edits out

`craft-host` is a small egui-free crate. The app builds a `Host` once and installs it on the egui context. Every feature reads it with `Host::of(ui)`. No feature takes an i18n or settings parameter.

```rust
fn setup(ctx: &egui::Context, app: &App) {
    Host::builder(app.info())
        .i18n(app.i18n.clone())
        .settings(app.settings.clone())
        .commands(app.keymap.clone())
        .files(app.files.clone())
        .jobs(Arc::new(craft::host::ThreadJobs))
        .install(ctx);
}
```

| Rule | Detail |
| --- | --- |
| Services in | Identity, language, settings sections, commands, files and jobs. Each is a trait the app or a library crate implements. |
| Document data in | Each feature defines one small trait (`TimelineModel`, `ColorTarget`, `LayerTree`) that the app implements on its document view. Features own no document state |
| Edits out | Features return edit events (`Begin`, `Update`, `Commit`, `Cancel`, `Set`). The app applies them to its own undo stack |
| Dialogs | A feature dialog is a value the app opens on the kernel's `DialogHost`. Its result is a `DialogResult<T>` (`Ok(T)` or `Cancel`) the app takes on a later pass. No feature opens its own window |

The remaining service rules (settings keys, words, commands, files) are in [reference/crate-layout.md](reference/crate-layout.md#host-service-rules). The examples show the shape, not settled APIs.

**Cancel and Escape.** Esc during a gesture ends it with `Cancel { reason: Escape }`. The app restores the value it held at `Begin` and makes no undo step.

```rust
match event {
    EditEvent::Begin { id, .. } => self.mark = Some(doc.undo_mark(id)),
    EditEvent::Update { value, .. } => doc.preview(FILL, value),
    EditEvent::Commit { value, changed, .. } => if changed { doc.apply(Edit::set(FILL, value)) },
    EditEvent::Cancel { .. } => if let Some(m) = self.mark.take() { doc.restore(m) },
    EditEvent::Set { value, .. } => doc.apply(Edit::set(FILL, value)),
}
```

**Interrupted gesture.** Besides Esc, other events end a gesture with `Cancel`; the `CancelReason` says which: `FocusLost`, `Dismissed`, `DocumentSwitched`, `UndoDuringGesture`, `Removed`, `Disabled`, `External`. The handler above covers all of them. After `External`, the control shows the host's value and the outside change stays.

**Error.** A failing service call returns a typed error, and the feature shows it in its own UI without an edit. File calls return `Result<_, FileError>`. A dismissed picker answers `Picked::Cancelled`, not an error.

**Async job.** Work runs on the host's `Jobs`, never on a thread a feature starts. `JobQueue` adds priorities, one job per key, replacement and cancellation. Dropping a key sets the job's cancel flag, and a cancelled job's result is never delivered.

```rust
queue.submit(photo_id, Priority::VISIBLE, move |cancel| decode(&path, cancel));
while let Some(Finished { key, result }) = queue.try_recv() { show(key, result) } // on a later pass
```

## 3.5 One egui revision

`craft-libs` pins one revision of the egui fork. Each app's `xtask` checks that its lockfile uses the same revision and fails with a clear message otherwise. Two revisions in one build would give two incompatible `Tokens` and `Ui` types.
