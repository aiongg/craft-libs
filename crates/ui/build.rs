//! Generates `craft_ui::icons::{Icon, IconSet}` from `icons/icons.toml`, embedding each set's files.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use toml::{Table, Value};

struct Set {
    id: String,
    title: String,
    licence: String,
    stroke_width: Option<f64>,
    /// The base set: every icon has a file in it, and the other sets fall back to it.
    base: bool,
}

struct Entry {
    name: String,
    directional: bool,
    /// One file stem per set, in set order; `None` where a non-base set has no file.
    files: Vec<Option<String>>,
}

fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let dir = Path::new(&root).join("icons");
    println!("cargo::rerun-if-changed=icons");
    let text = std::fs::read_to_string(dir.join("icons.toml")).expect("read icons/icons.toml");
    let manifest: Table = text.parse().unwrap_or_else(|e| panic!("icons/icons.toml: {e}"));
    let sets = sets(&manifest);
    let icons = icons(&manifest, &sets);
    check_files(&dir, &sets, &icons);
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    std::fs::write(Path::new(&out).join("icons.rs"), generate(&root, &sets, &icons)).expect("write icons.rs");
}

fn str_field<'a>(table: &'a Table, key: &str, what: &str) -> &'a str {
    table.get(key).and_then(Value::as_str).unwrap_or_else(|| panic!("icons/icons.toml: {what} has no string `{key}`"))
}

fn sets(manifest: &Table) -> Vec<Set> {
    let list = manifest.get("set").and_then(Value::as_array).expect("icons/icons.toml: no [[set]] tables");
    let sets: Vec<Set> = list
        .iter()
        .map(|value| {
            let table = value.as_table().expect("icons/icons.toml: [[set]] is not a table");
            let id = str_field(table, "id", "a [[set]]").to_owned();
            assert!(is_slug(&id, '_'), "icons/icons.toml: set id `{id}` is not lower-case ascii");
            let what = format!("set `{id}`");
            let stroke_width = match str_field(table, "kind", &what) {
                "stroke" => Some(
                    table.get("stroke_width").and_then(Value::as_float).unwrap_or_else(|| panic!("icons/icons.toml: {what} has no `stroke_width`")),
                ),
                "fill" => None,
                other => panic!("icons/icons.toml: {what} has kind `{other}`, not `stroke` or `fill`"),
            };
            Set {
                title: str_field(table, "title", &what).to_owned(),
                licence: str_field(table, "licence", &what).to_owned(),
                stroke_width,
                base: table.get("base").and_then(Value::as_bool).unwrap_or(false),
                id,
            }
        })
        .collect();
    assert!(!sets.is_empty(), "icons/icons.toml: no sets");
    assert_eq!(sets.iter().filter(|s| s.base).count(), 1, "icons/icons.toml: exactly one [[set]] must have `base = true`");
    sets
}

fn icons(manifest: &Table, sets: &[Set]) -> Vec<Entry> {
    let list = manifest.get("icon").and_then(Value::as_array).expect("icons/icons.toml: no [[icon]] tables");
    let mut seen = BTreeSet::new();
    list.iter()
        .map(|value| {
            let table = value.as_table().expect("icons/icons.toml: [[icon]] is not a table");
            let name = str_field(table, "name", "an [[icon]]").to_owned();
            assert!(is_slug(&name, '_'), "icons/icons.toml: icon name `{name}` is not lower-case ascii with underscores");
            assert!(seen.insert(name.clone()), "icons/icons.toml: icon `{name}` is listed twice");
            let directional = table
                .get("directional")
                .and_then(Value::as_bool)
                .unwrap_or_else(|| panic!("icons/icons.toml: icon `{name}` has no `directional = true|false`"));
            let files = sets
                .iter()
                .map(|set| {
                    let file = table.get(&set.id).and_then(Value::as_str);
                    assert!(file.is_some() || !set.base, "icon `{name}` has no file in the base set `{}`", set.id);
                    if let Some(file) = file {
                        assert!(is_slug(file, '-'), "icon `{name}`: file `{file}` in set `{}` is not a plain file stem", set.id);
                    }
                    file.map(str::to_owned)
                })
                .collect();
            Entry { name, directional, files }
        })
        .collect()
}

/// Every listed file exists, and every set folder holds only listed files and its licence.
fn check_files(dir: &Path, sets: &[Set], icons: &[Entry]) {
    for (index, set) in sets.iter().enumerate() {
        let folder = dir.join(&set.id);
        for icon in icons {
            if let Some(file) = &icon.files[index] {
                let path = folder.join(format!("{file}.svg"));
                assert!(path.is_file(), "icon `{}` is missing from set `{}`: no file {}", icon.name, set.id, path.display());
            }
        }
        let listed: BTreeSet<String> = icons.iter().filter_map(|icon| icon.files[index].as_ref()).map(|file| format!("{file}.svg")).collect();
        let entries = std::fs::read_dir(&folder).unwrap_or_else(|e| panic!("read {}: {e}", folder.display()));
        for entry in entries {
            let file = entry.expect("read icon folder entry").file_name().to_string_lossy().into_owned();
            assert!(file == "LICENSE" || listed.contains(&file), "set `{}` holds {file}, which icons.toml does not list", set.id);
        }
        assert!(folder.join("LICENSE").is_file(), "set `{}` has no LICENSE file", set.id);
    }
}

fn is_slug(s: &str, separator: char) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase()) && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == separator)
}

fn camel(s: &str) -> String {
    s.split(['_', '-']).map(|word| word[..1].to_ascii_uppercase() + &word[1..]).collect()
}

fn generate(root: &str, sets: &[Set], icons: &[Entry]) -> String {
    let mut out = String::new();
    let n = icons.len();
    let m = sets.len();
    out.push_str("/// An icon of the suite, by meaning. The list and each set's files are in `crates/ui/icons/icons.toml`.\n");
    out.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\npub enum Icon {\n");
    for icon in icons {
        let _ = writeln!(out, "    /// `{}`{}", icon.name, if icon.directional { ", mirrored right to left" } else { "" });
        let _ = writeln!(out, "    {},", camel(&icon.name));
    }
    out.push_str("}\n\nimpl Icon {\n    /// Every icon, in manifest order.\n");
    let _ = writeln!(
        out,
        "    pub const ALL: [Icon; {n}] = [{}];\n",
        icons.iter().map(|i| format!("Icon::{}", camel(&i.name))).collect::<Vec<_>>().join(", ")
    );
    out.push_str("    /// The manifest name, e.g. `chevron_right`.\n    pub const fn name(self) -> &'static str {\n        match self {\n");
    for icon in icons {
        let _ = writeln!(out, "            Icon::{} => {:?},", camel(&icon.name), icon.name);
    }
    out.push_str("        }\n    }\n\n    /// Whether the icon points along the reading direction, so it mirrors right to left.\n");
    let directional: Vec<String> = icons.iter().filter(|i| i.directional).map(|i| format!("Icon::{}", camel(&i.name))).collect();
    let body = if directional.is_empty() { "false".to_owned() } else { format!("matches!(self, {})", directional.join(" | ")) };
    let _ = writeln!(out, "    pub const fn directional(self) -> bool {{\n        {body}\n    }}\n");
    out.push_str(
        "    /// The icon's SVG document in `set`: the set's own file, or the base set's when the set has none.\n    pub const fn svg(self, set: IconSet) -> &'static [u8] {\n        match set {\n",
    );
    let base = sets.iter().find(|s| s.base).expect("a base set");
    for set in sets {
        if set.base {
            let _ = writeln!(out, "            IconSet::{} => {}[self as usize],", camel(&set.id), set.id.to_ascii_uppercase());
        } else {
            let _ = writeln!(
                out,
                "            IconSet::{} => match {}[self as usize] {{\n                Some(bytes) => bytes,\n                None => {}[self as usize],\n            }},",
                camel(&set.id),
                set.id.to_ascii_uppercase(),
                base.id.to_ascii_uppercase()
            );
        }
    }
    out.push_str("        }\n    }\n\n");
    out.push_str("    /// Whether `set` draws the icon from its own file; `false` means [`Icon::svg`] returns the base set's.\n    pub const fn drawn_in(self, set: IconSet) -> bool {\n        match set {\n");
    for set in sets {
        if set.base {
            let _ = writeln!(out, "            IconSet::{} => true,", camel(&set.id));
        } else {
            let _ = writeln!(out, "            IconSet::{} => {}[self as usize].is_some(),", camel(&set.id), set.id.to_ascii_uppercase());
        }
    }
    out.push_str("        }\n    }\n}\n\n");

    out.push_str("/// An icon set. The base set draws every icon; the others are packs that draw the icons they have and fall back to the base set for the rest.\n");
    out.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\npub enum IconSet {\n");
    for set in sets {
        let _ = writeln!(out, "    /// {}, {}.\n    {},", set.title, set.licence, camel(&set.id));
    }
    out.push_str("}\n\nimpl IconSet {\n    /// Every set, in manifest order.\n");
    let _ = writeln!(
        out,
        "    pub const ALL: [IconSet; {m}] = [{}];\n",
        sets.iter().map(|s| format!("IconSet::{}", camel(&s.id))).collect::<Vec<_>>().join(", ")
    );
    let _ = writeln!(
        out,
        "    /// The base set: it draws every icon, and the other sets fall back to it.\n    pub const BASE: IconSet = IconSet::{};\n",
        camel(&base.id)
    );
    set_fn(&mut out, sets, "The folder name under `crates/ui/icons/`.", "id", "&'static str", |s| format!("{:?}", s.id));
    set_fn(&mut out, sets, "The set's name for display.", "title", "&'static str", |s| format!("{:?}", s.title));
    set_fn(&mut out, sets, "The SPDX licence of the set's files.", "licence", "&'static str", |s| format!("{:?}", s.licence));
    set_fn(&mut out, sets, "The set's own stroke width in view-box units; `None` for a set drawn with fills.", "stroke_width", "Option<f32>", |s| {
        s.stroke_width.map_or("None".to_owned(), |w| format!("Some({w:?})"))
    });
    out.push_str("}\n");

    for (index, set) in sets.iter().enumerate() {
        if set.base {
            let _ = writeln!(out, "\nconst {}: [&[u8]; {n}] = [", set.id.to_ascii_uppercase());
        } else {
            let _ = writeln!(out, "\nconst {}: [Option<&[u8]>; {n}] = [", set.id.to_ascii_uppercase());
        }
        for icon in icons {
            match (&icon.files[index], set.base) {
                (Some(file), true) => {
                    let _ = writeln!(out, "    include_bytes!({:?}),", format!("{root}/icons/{}/{file}.svg", set.id));
                }
                (Some(file), false) => {
                    let _ = writeln!(out, "    Some(include_bytes!({:?})),", format!("{root}/icons/{}/{file}.svg", set.id));
                }
                (None, _) => out.push_str("    None,\n"),
            }
        }
        out.push_str("];\n");
    }
    out
}

/// A `const fn` of `IconSet` with one match arm per set.
fn set_fn(out: &mut String, sets: &[Set], doc: &str, func: &str, ty: &str, value: fn(&Set) -> String) {
    let _ = writeln!(out, "    /// {doc}\n    pub const fn {func}(self) -> {ty} {{\n        match self {{");
    for set in sets {
        let _ = writeln!(out, "            IconSet::{} => {},", camel(&set.id), value(set));
    }
    out.push_str("        }\n    }\n\n");
}
