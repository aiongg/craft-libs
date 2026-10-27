# craft-theme-store

The Craft suite's theme files, the machine-local theme library and the shared suite selection,
outside rendering. Several apps can change the selection and the library at the same time
without losing a change: every write takes a lock, re-reads the file, merges and bumps a
revision. No egui; the theme contract (ids, roles, built-ins, resolution) is
[`craft-theme`](../theme/README.md). [docs/suite-themes.md](../../docs/suite-themes.md#file-format-and-storage)
describes the file format, the folder and the protocol.

```rust,ignore
use craft_theme::{Mode, Selection, ThemeId};
use craft_theme_store::{SelectionChange, Store, StoreError, StoreOptions, SuiteDir};

// Desktop shells only; app constructors, tests and web builds use SuiteDir::memory().
let mut store = Store::open(&SuiteDir::resolve(false), StoreOptions::new("pdfcraft 0.15.0"));
let _ = store.load_selection(); // a problem keeps the last accepted state

// The user picks a theme. base is the accepted state the UI showed; `shown` is the selection on
// screen (the suite's, or the app's own default), which fills fields the change leaves alone.
let base = store.accepted().clone();
let change = SelectionChange::new(shown).with_theme(ThemeId::parse("craft.graphite")?);
match store.update(&base, change) {
    Ok(accepted) => { /* show accepted.suite */ }
    Err(StoreError::Conflict { disk }) => { /* "Changed in another Craft app", offer "Apply mine" */ }
    Err(e) => { /* back to store.accepted(), show e with Retry */ }
}
```

| Item | What it is |
|---|---|
| `SuiteDir` | `CRAFT_SUITE_DIR` (empty: in memory), in memory for isolated apps, else `ArtCraft/themes` in the local config folder |
| `Store` | `load_selection`, `update`, `import`, `delete`, `export`, `library`, `load_theme`, `theme` (a `resolve` lookup) |
| `SelectionChange` | the fields the user changed (`theme`, `mode`) over the selection the app showed |
| `Accepted` | `suite: Option<Selection>`, `revision`, `library_revision`; `None` until an app writes a suite choice |
| `StoreError` | `Busy`, `Conflict { disk }`, `IdExists`, `InUse`, … ; nothing on disk changed |
| `Problem` | damaged, too new or unreadable files; the store keeps its last accepted state |
| `parse_theme`, `theme_to_json`, `ThemeRecord` | theme file v1; unknown keys stay in the stored bytes |
| `FileStamp`, `library_files`, `selection_stamp`, `theme_stamp`, `parse_counts` | cheap change checks for a watcher |

`Store` methods block on file I/O and on the lock (up to 2 s), so apps call them from a worker
thread. Without a folder the same store runs in memory, and on `wasm32` that is the only mode.

Feature `testing` adds `SAMPLE_THEME_JSON` with its expected colours (`sample_roles`), for each
app's adapter test, and `FailingFs`, a file system that fails one step of a write.

The lock file and revision protocol, the re-check before the rename and the failure handling
follow the theme store in [pdfcraft#1090](https://github.com/storytold/pdfcraft/pull/1090) by
mjshfrvstj-sys.
