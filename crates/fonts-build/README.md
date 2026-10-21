# craft-fonts-build

Embeds the optional [craft-fonts](https://github.com/storytold/craft-fonts) faces from an app's
`build.rs`, and defines `EmbeddedFace`, the plain data type of the generated table. No
dependencies and no egui, so crates below the UI layer (text engines, exporters) can use it.

Apps never list craft-fonts in `Cargo.toml`: Cargo would fetch the font repository for every
build. The build reads a local checkout named by `CRAFT_FONTS_DIR` instead.

| Environment | Result |
|---|---|
| `CRAFT_FONTS_DIR` unset or empty | Empty table. |
| `CRAFT_FONTS_DIR` = a craft-fonts checkout | Every selected face of `fonts/manifest.txt`, embedded with `include_bytes!`. |
| `CRAFT_FONTS_DIR` set, but no manifest or a missing font file | `cargo::warning`, empty table. |
| `CRAFT_FONTS_REQUIRED=1` (exactly `1`; other values are ignored) | Error instead when the directory is unset, unreadable, has no selected face, or has no face for a script passed to `scripts`. |

Give `CRAFT_FONTS_DIR` as an absolute path: Cargo runs build scripts in the package's directory.

## Using it

`Cargo.toml` (same `rev` as the app's other craft-libs crates):

```toml
[dependencies]
craft-fonts-build = { git = "https://github.com/storytold/craft-libs", rev = "…" }

[build-dependencies]
craft-fonts-build = { git = "https://github.com/storytold/craft-libs", rev = "…" }
```

`build.rs` (FilmCraft's `crates/text/build.rs`, in full):

```rust
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let wasm = craft_fonts_build::target_is_wasm();
    let embedded = craft_fonts_build::Embed::new()
        // Web builds have a size budget: embed only the Japanese UI face there.
        .filter(move |face| !wasm || (face.family.as_str(), face.style.as_str()) == ("BIZ UDPGothic", "Regular"))
        .write("craft_fonts.rs");
    if let Err(e) = embedded {
        println!("cargo::error={e}");
    }
}
```

The crate:

```rust
use craft_fonts_build::EmbeddedFace;

/// craft-fonts faces; empty unless built with `CRAFT_FONTS_DIR`.
pub static CRAFT_FONTS: &[EmbeddedFace] = include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));
```

`craft_fonts_build::embed("craft_fonts.rs")` embeds every face. `Embed` narrows the set:

- `scripts(&["Jpan", "Arab", "Hant"])` keeps faces with any of these script tags. A tag matches
  itself and its region forms (`Hant` matches `Hant-HK`). With `CRAFT_FONTS_REQUIRED=1`, each tag
  must match at least one face.
- `filter(|entry| …)` keeps faces by any manifest field (family, style, licence, index).
- `dir(path)` and `required(bool)` override the two environment variables.

`write` returns the embedded `ManifestEntry`s, for build-time checks such as "the UI face is
present". The generated file is a slice expression of `EmbeddedFace { family, style, scripts,
index, bytes }` literals; any struct with those fields and that name in scope works too.

## Serving the faces (web builds)

`Embed::serve(dir, url_prefix)` copies the selected faces into `dir` instead of embedding them,
and writes a table of `ServedFace { family, style, scripts, index, url, size, integrity }`
literals for the app to fetch at run time (craft-ui's `fonts::UiFonts::served_faces` does). Each
font file and its licence keep their path under the checkout's `fonts/` directory, and `url` is
`url_prefix` plus that path: `serve("web/fonts", "fonts/")` copies
`fonts/noto-sans-arabic/NotoSansArabic.ttf` to `web/fonts/noto-sans-arabic/NotoSansArabic.ttf`, with
the URL `fonts/noto-sans-arabic/NotoSansArabic.ttf`. Faces of one collection share a file and a
URL. `integrity` is the manifest's SHA-256 as a Subresource Integrity value. A relative `dir` is
under the package directory. Files already there with the right size are not copied again, a deleted
one is copied again on the next build, and files of faces no longer selected stay.

```rust
let fonts = craft_fonts_build::Embed::new();
let written = if craft_fonts_build::target_is_wasm() {
    fonts.serve("web/fonts", "fonts/").write("craft_fonts.rs")
} else {
    fonts.write("craft_fonts.rs")
};
```

The crate includes the table as `&[ServedFace]` on wasm and `&[EmbeddedFace]` elsewhere. craft-ui's
README shows a whole eframe web app, Trunk copy included.

## The manifest

One face per line, eight fields separated by `" | "`; craft-fonts' `docs/integration.md` defines
them. Lines starting with `#` are comments. A file field ending in `#<n>` names face `n` of a font
collection (`fonts/noto-sans-cjk/NotoSansCJK.ttc#2`); without it the index is 0. Fields after the
eighth are ignored.
