//! The parity replay (spec 22.5): runs a story's generic script (Tab walk, then clicks and keys on each actionable
//! node) on the new library and compares the text with the phase-0 capture of the legacy library under `capture/`.
//!
//! The capture format is the one `craft-ui-parity` writes: the story id, its args, the AccessKit tree without
//! bounds, then what the script changed. A difference is a line of text for review, never a failure: node
//! nesting, names and flags are expected to change in places, and each change is accepted or fixed by a reviewer.
//!
//! A family track replays one story with [`replay`], or every captured story that matches a filter with
//! [`replay_matching`]; `CRAFT_PARITY=<filter>` names the filter a trial reads through [`env_filter`].

mod script;
mod tree;

/// A node of a kittest harness and the trait that walks them, for the AccessKit adapter that reads them.
pub(crate) use egui_kittest::Node as KitNode;
pub(crate) use egui_kittest::kittest::NodeT as KitNodeT;
pub(crate) use tree::Line;

use egui_kittest::Harness;
use std::fmt;
use std::path::{Path, PathBuf};

/// What a story's script reads and compares besides the tree: the story's args as `(name, text)` pairs, written the
/// way the capture wrote them (`Text("Blend mode")`, `Bool(false)`).
pub trait ParityState: Clone {
    /// The args in their fixed order.
    fn args(&self) -> Vec<(String, String)>;
}

/// A story without args.
impl ParityState for () {
    fn args(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}

/// The frame length every step runs at, so animations advance by the same amount on every run.
const STEP_DT: f32 = 1.0 / 60.0;

/// The size of the surface a story is shown on, in points.
const SURFACE: [f32; 2] = [640.0, 1200.0];

/// A harness for a story: the runtime installed, the capture's surface size and frame length. `app` shows the story.
// TK-MERGE: TK-a (the fixture builds this harness, with the suite's fonts and the story's theme and scale)
pub fn harness<'a, S: ParityState + 'a>(state: S, app: impl FnMut(&mut egui::Ui, &mut S) + 'a) -> Harness<'a, S> {
    Harness::builder()
        .allow_missing_glyphs()
        .with_size(egui::Vec2::from(SURFACE))
        .with_step_dt(STEP_DT)
        .with_accessibility_check(false)
        .with_context_setup(|ctx| {
            let installed = crate::install(ctx, crate::testing::install_spec());
            debug_assert!(installed.is_ok(), "the context takes the runtime once");
        })
        .build_ui_state(app, state)
}

/// The story as text: its id, args and tree at rest, then the script's steps. The same format the capture has.
pub fn record<S: ParityState>(id: &str, harness: &mut Harness<'_, S>) -> String {
    use std::fmt::Write as _;
    script::settle(harness);
    let mut out = String::new();
    let _ = writeln!(out, "story {id}");
    let _ = writeln!(out, "args");
    out.push_str(&tree::args(harness.state()));
    let _ = writeln!(out, "tree");
    out.push_str(&tree::tree(harness).text());
    script::run(harness, &mut out);
    out
}

/// The directory of the capture: `capture/` at the repository root.
pub fn capture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("capture")
}

/// The capture file of a story id: `<level>/<component>--<story>.txt`.
pub fn capture_file(id: &str) -> PathBuf {
    let mut parts = id.split('/');
    let level = parts.next().unwrap_or(id);
    let rest: Vec<&str> = parts.collect();
    let name = if rest.is_empty() { id.to_owned() } else { rest.join("--") };
    capture_dir().join(level).join(format!("{name}.txt"))
}

/// The ids of the captured stories that contain `filter`, in file order. The id is the capture's first line.
pub fn captured_ids(filter: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut levels: Vec<PathBuf> =
        std::fs::read_dir(capture_dir()).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    levels.sort();
    for level in levels {
        let mut files: Vec<PathBuf> = std::fs::read_dir(level)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "txt"))
            .collect();
        files.sort();
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            if let Some(id) = text.lines().next().and_then(|line| line.strip_prefix("story ")).filter(|id| id.contains(filter)) {
                ids.push(id.to_owned());
            }
        }
    }
    ids
}

/// The filter in `CRAFT_PARITY`, when set and not empty.
pub fn env_filter() -> Option<String> {
    std::env::var("CRAFT_PARITY").ok().filter(|filter| !filter.is_empty())
}

/// The result of replaying one story against its capture.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Replay {
    /// The legacy story id.
    pub id: String,
    /// There is no capture file for the id.
    pub missing: bool,
    /// Lines only in the capture (`- `) and only in the replay (`+ `), in document order.
    pub differences: Vec<String>,
}

impl Replay {
    /// The replay matched the capture line for line.
    pub fn identical(&self) -> bool {
        !self.missing && self.differences.is_empty()
    }
}

impl fmt::Display for Replay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.missing {
            return write!(f, "{}: no capture at {}", self.id, capture_file(&self.id).display());
        }
        if self.differences.is_empty() {
            return write!(f, "{}: identical", self.id);
        }
        write!(f, "{}: {} differing lines", self.id, self.differences.len())?;
        for line in &self.differences {
            write!(f, "\n  {line}")?;
        }
        Ok(())
    }
}

/// Replays the story shown by `harness` as the legacy story `id` and compares it with the capture.
pub fn replay<S: ParityState>(id: &str, harness: &mut Harness<'_, S>) -> Replay {
    let Ok(captured) = std::fs::read_to_string(capture_file(id)) else {
        return Replay { id: id.to_owned(), missing: true, differences: Vec::new() };
    };
    let replayed = record(id, harness);
    Replay { id: id.to_owned(), missing: false, differences: line_diff(&captured, &replayed) }
}

/// Replays every captured story whose id contains `filter` for which `harness_of` returns a harness (it returns
/// `None` for a story the track has not migrated).
pub fn replay_matching<'a, S: ParityState>(filter: &str, mut harness_of: impl FnMut(&str) -> Option<Harness<'a, S>>) -> Vec<Replay> {
    captured_ids(filter).into_iter().filter_map(|id| harness_of(&id).map(|mut harness| replay(&id, &mut harness))).collect()
}

/// The lines only in `old` as `- ` and only in `new` as `+ `, in order, from their longest common subsequence.
/// A pair too large to tabulate is reported as replaced whole.
pub fn line_diff(old: &str, new: &str) -> Vec<String> {
    let (old, new): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..].iter().rev().zip(new[prefix..].iter().rev()).take_while(|(a, b)| a == b).count();
    let (old, new) = (&old[prefix..old.len() - suffix], &new[prefix..new.len() - suffix]);
    let mut out = Vec::new();
    if old.len().saturating_mul(new.len()) > 16_000_000 {
        out.extend(old.iter().map(|l| format!("- {}", l.trim_start())));
        out.extend(new.iter().map(|l| format!("+ {}", l.trim_start())));
        return out;
    }
    let width = new.len() + 1;
    let mut table = vec![0u32; (old.len() + 1) * width];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            table[i * width + j] =
                if old[i] == new[j] { table[(i + 1) * width + j + 1] + 1 } else { table[(i + 1) * width + j].max(table[i * width + j + 1]) };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < old.len() || j < new.len() {
        if i < old.len() && j < new.len() && old[i] == new[j] {
            i += 1;
            j += 1;
        } else if i < old.len() && (j == new.len() || table[(i + 1) * width + j] >= table[i * width + j + 1]) {
            out.push(format!("- {}", old[i].trim_start()));
            i += 1;
        } else {
            out.push(format!("+ {}", new[j].trim_start()));
            j += 1;
        }
    }
    out
}
