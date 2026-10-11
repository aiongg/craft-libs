# Craft Text

`craft-text` holds the text processing that sits below each app's composer: functions that take
plain text and a language and return what layout needs before it shapes and places glyphs.

| Module | Feature | Provides |
| --- | --- | --- |
| `hyphen` | `hyphen` (default) | Hyphenation points |

Segmentation, bidi and line breaking are planned as further modules beside `hyphen`.

Language features (`spanish`, `german`, …, `all-languages`) add the data for one language to every
module that uses it, and turn on `hyphen`. Build with `default-features = false` to leave out all
modules and data.

## Hyphenation

`craft_text::hyphen` finds hyphenation points. It returns byte offsets into the word: a hyphen
may go before the char that starts at each offset. Line breaking, discretionary hyphens and
exception lists stay in each application.

### English

English is always available with `hyphen` and needs no language feature. Each word is looked up
in this order:

1. the bundled dictionary (`Dictionary::en_us`): the Moby Hyphenator II word list, about 160,000
   words. Simple inflections (`'s`, `-s`, `-es`, `-ed`, `-d`) inherit the breaks of their stem;
2. the bundled Liang patterns (`assets/hyphen/en-us.pat`), trained from the same list by the crate's
   patgen-style trainer;
3. a small syllable heuristic for words outside the pattern alphabet.

Case, apostrophes (`don't`, `’s`), surrounding punctuation and compounds are handled; no break is
placed next to an apostrophe or a non-letter. `Dictionary::en_us()` also serves as an English word
list, for example for spelling checks.

### Other languages

Other languages use the patterns compiled into [hypher](https://crates.io/crates/hypher) 0.1.8.
Each has a cargo feature named as in hypher (`spanish`, `german`, `norwegian`, …); `all-languages`
enables all 47. Patterns for languages whose feature is off are not compiled in.

```rust
use craft_text::hyphen::{Lang, Limits, hyphen_points_in};

fn points(word: &str, language: &str) -> Vec<usize> {
    match Lang::for_language(language) {
        Some(lang) => hyphen_points_in(word, &Limits::default(), lang),
        None => Vec::new(), // unknown language or feature off: don't hyphenate
    }
}
```

`Lang::for_language` accepts English language names as InDesign and IDML write them (`Spanish`,
`German: 2006 Reform`, `$ID/Norwegian: Bokmal`) and locale codes (`es`, `es_ES`, `pt-BR`,
`de_DE_2006`). It returns `None` for unknown languages and for languages not compiled in.
`Lang` is `Copy + Eq + Hash`, so it can key a cache of computed points. Its `Patterns` variant
holds an opaque `PatternLang`; build one with `Lang::for_language`.

### Limits

`Limits` holds the paragraph's settings: minimum word length, letters before the first and after
the last hyphen, and whether capitalized words are hyphenated. Words in all capitals are never
hyphenated. Limit checks are linear in word length.

### Regenerating the English data

`examples/hyphgen.rs` rebuilds `assets/hyphen/en-us.dic` and `assets/hyphen/en-us.pat` from the
Moby list. It needs the `generator` feature, which exposes the dictionary writer (`hyphen::dict`)
and the pattern trainer (`hyphen::patgen`); applications don't need it.

```sh
curl -LO https://www.gutenberg.org/files/3204/files/mhyph.txt
cargo run --release -p craft-text --features generator --example hyphgen -- mhyph.txt crates/text/assets/hyphen --validate
```

## Licenses

The word list is public domain; the trained patterns are CC0-1.0. hypher's patterns come from
hyph-utf8 under their own permissive licenses (LPPL, MPL, MIT, BSD-3); see the repository's
[NOTICE](../../NOTICE) and [ATTRIBUTION.md](../../ATTRIBUTION.md).

## Checks

Some tests depend on language features, so run the suite with them on:

```sh
cargo test -p craft-text --all-features
```
