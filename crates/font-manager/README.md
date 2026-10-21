# craft-font-manager

Craft Font Manager as a library: every font the suite's apps can use, and the Google Fonts
collection to add to them. ArtCraft Launcher shows it as its Fonts tab; the standalone app
([aiongg/craft-font-manager](https://github.com/aiongg/craft-font-manager)) shows it as its window.

| Part | Feature | What it does |
|---|---|---|
| `filter` | always | Search over the index: family names ignoring case and accents, category, language (English name, autonym or id) and script |
| `index` | always | The pinned Google Fonts index compiled into [`craft-font-store`](../font-store/README.md) (`bundled`), and its categories |
| `installed` | always | The installed fonts [`craft-font-catalog`](../font-catalog/README.md) sees, grouped by family and source |
| `store` | always | `Installer`, what the interface needs from the font store: the download consent, install, remove, and downloads for a preview. `SuiteStore` is craft-font-store's store, `FakeStore` a stand-in for tests, `NoStore` one for when the store folder is unknown |
| `FontsPanel` | `egui` | The whole interface as one egui panel |

## The panel

- **Installed**: system and font store fonts by family, with where each comes from, its styles and
  a sample line drawn in the face itself. Font store families can be removed; system fonts are
  removed with the system's own tools.
- **Catalogue**: the Google Fonts collection. Search by name, filter by category, language and
  script, and set the sample text and size. Installed families show their styles at once; for the
  others, **Preview** downloads the family for the session without installing it.
- **Install and remove** with progress, through the `Installer` on a worker thread.

`FontsPanel` fills the `Ui` it is given and has no window, menu, theme or interface fonts of its
own. The faces it draws samples in are egui font families of its own, added a few per frame and
taken out again past 64, without touching the host's fonts.

```rust
let catalog = Arc::clone(Catalog::global());
catalog.scan_in_background();
let installer: Arc<dyn Installer> = match SuiteStore::for_user(Arc::clone(&catalog)) {
    Some(store) => Arc::new(store),
    None => Arc::new(NoStore),
};
let options = Options { catalog, installer, index: index::bundled().map_err(|e| e.to_string()), runner: Runner::Thread };
let mut panel = FontsPanel::new(options, Settings::default());
// each frame
panel.ui(ui);
```

| Call | Use |
|---|---|
| `settings()`, `settings_mut()` | What the host saves: the sample text and size (`serde`) |
| `downloads_allowed()`, `set_downloads_allowed(on)` | The download consent, for a host's settings page. It is the font store's own setting, shared by every Craft app |
| `set_language(tag)` | The interface language, the host's own; English when its catalog isn't embedded |
| `install_interface_fonts(ctx, craft_faces)` | For a host without interface fonts of its own (the standalone app): craft-ui's font stack for the panel's language, craft-fonts' faces the app embeds with craft-fonts-build, and the interface direction |

## Network use

Nothing is downloaded until the user allows it. Downloads are off by default; the first Install or
Preview asks first. Files come from google/fonts at the index's pinned commit through jsDelivr (raw
GitHub as the fallback), and each is checked against the index's size and sha256 before use.
Previews stay in memory and are never installed.

## Strings

Every interface string goes through craft-i18n with the English text as the key. The interface is
English until the panel's catalog rows exist in craft-i18n.

## Testing

```sh
cargo test -p craft-font-manager --all-features
CRAFT_FONT_MANAGER_SNAPSHOTS=<dir> cargo test -p craft-font-manager --features egui --test flows screenshot -- --test-threads=1
```

`tests/flows.rs` drives the panel with `egui_kittest` against a small index, a font folder made per
test and `FakeStore`. They install egui's Hack and Ubuntu Light faces into temporary folders.
