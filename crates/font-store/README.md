# craft-font-store

The suite's font store: the Google Fonts index, exact lookup of a missing font, and verified installs into the one folder every app's font catalog reads ([`craft-font-catalog`](../font-catalog/README.md)'s `font_store_dir`).

Fonts come from [google/fonts](https://github.com/google/fonts) at the commit the index records. The index is [aiongg/google-fonts-index](https://github.com/aiongg/google-fonts-index) (format 1.x), designed by @kntrttm for photocraft#1135 and #2453.

## Network access

| Rule | How |
|---|---|
| Off until the user turns it on | `FontStore::network()` is `Network::Off` until `enable_network()`. The setting is `settings.json` in the store folder, so it holds for every app. |
| Only the user turns it on | `FontStore::for_user` is for the app's own interface; call `enable_network()` from a setting or from the user's Install click. `FontStore::for_agent` (MCP, CLI, scripting) gets `Error::NotAllowed`. An agent can install while the user has the network on. |
| No requests at startup | Nothing in the crate makes a request on its own. Only `fetch_index` and the installs do, and they return `Error::NetworkOff` without one while the network is off. |
| One HTTP client | `Client`: ureq with rustls and the webpki roots, HTTPS only, no cookies, `User-Agent: craft-font-store/<version>`. Timeouts: 15 s to connect, 30 s to the response headers, 10 minutes per file. |

Files come from `https://cdn.jsdelivr.net/gh/google/fonts@<commit>/<path>`; when that fails or gives bytes that don't verify, from `https://raw.githubusercontent.com/google/fonts/<commit>/<path>`.

## The index

| Call | What it does |
|---|---|
| `Index::bundled()` (feature `bundled-index`) | The pinned index compiled in: about 500 KB gzipped, no request. |
| `FontStore::cached_index(&PINNED_INDEX)` | The index the last `fetch_index` saved in the store folder, when its sha256 is the pin's. |
| `FontStore::fetch_index(&PINNED_INDEX, &cancel)` | Downloads the pinned index from jsDelivr (raw GitHub as fallback), checks its sha256 and saves it. Needs the network on. |
| `Index::from_bytes(bytes)` | Any copy: JSON, or gzipped JSON with the `gzip` feature. |

`PINNED_INDEX` names a commit of the index repository and the index's sha256; an index with other bytes is refused. Reading checks every entry: the version (a major version other than 1 is refused; unknown fields are ignored), HTTPS base URLs, paths of the form `<ofl|apache|ufl>/<folder>/<file>` with safe names, sha256s, sizes and weights.

## Exact matches only

| Call | Matches |
|---|---|
| `index.family(name)` | The whole family name, ignoring case. |
| `index.find_face(family, style)` | A style name (`Bold Italic`, `SemiBold`), a CSS weight (`700`) or a PostScript style (`BoldItalic`). A variable file has every weight its `wght` axis reaches. |
| `index.find_postscript(name)` | The PostScript name, case included. A name that several families share is `Unavailable::Ambiguous`; look it up in the document's family with `Family::face_by_postscript`. |

A family without the style asked for is `Unavailable::NoStyle`: the store never offers a nearer weight or a similar family. Substitution stays a separate choice in each app.

## Installing

```rust
use craft_font_catalog::{Catalog, FontEnv};
use craft_font_store::{Cancel, FontStore, Index};

let store = FontStore::for_user(&FontEnv::from_process()).unwrap().with_catalog(Catalog::global().clone());
let index = Index::bundled().unwrap();
let face = index.find_face("Lato", "Bold Italic")?;
store.enable_network()?; // the user clicked Install
let cancel = Cancel::new(); // cancel.cancel() from the UI thread stops the download
store.install_face(&index, face, &cancel, &mut |p| println!("{} of {} bytes", p.done, p.total))?;
```

`install_family` installs every file of a family; `install_face` one file and the licence. Each file is:

1. downloaded (at most its index size), with progress and cancellation;
2. checked: its size, its sha256, then that every face in it parses and has outlines (`verify_font`);
3. saved under `.staging/` as `<name>.part`, which no font scan picks up.

When every file has passed, the store takes `.lock`, moves the files into `<family folder>/` and writes `manifest.json` (written atomically through `craft-atomic`). If a move or the manifest fails, the files moved so far are taken back out and replaced files are restored. A failed or cancelled install leaves nothing behind. Files already installed with the same sha256 aren't downloaded again.

The licence file (`OFL.txt`, `LICENSE.txt`, `UFL.txt`) goes beside the fonts. The index has no sha256 for it, so only its size is checked (1 MiB at most).

## The store folder

```text
settings.json        the network setting
manifest.json        what is installed
index.json           the last index fetched
<family folder>/     google/fonts' folder name (lato/): the font files and the licence
.staging/            downloads being installed
.lock                held while the store changes
```

`manifest.json` lists each installed family: its name, folder, licence and licence file, and per file its name, google/fonts path, size, sha256, the index commit it came from, and its PostScript names. A manifest that can't be read is moved aside to `manifest.json.bad` and counts as empty.

| Call | What it does |
|---|---|
| `installed()`, `installed_family(name)`, `is_installed(family, file)` | Read the manifest. |
| `remove_family(name)` | Moves the family folder out at once, updates the manifest, then deletes the files and the licence. |
| `verify()` | Reads every installed file and reports the missing, damaged and unparseable ones. |

## Telling the catalog

The catalog scans the store folder. With `with_catalog(catalog)`, every install and removal ends with `catalog.rescan()`, which reads only new and changed files, so the app sees the fonts at once. Other running apps notice through `Catalog::rescan_if_changed` (when their window is activated) or `Catalog::watch`.

## Web

The index, the lookups, `verify_font` and `IndexPin` build for `wasm32-unknown-unknown`. The store folder and the HTTP client are native only; the web keeps fonts elsewhere.

## Tests

`cargo test -p craft-font-store --all-features`. The store tests run a small HTTP server on 127.0.0.1 that stands in for jsDelivr and raw GitHub; no test uses the network.
