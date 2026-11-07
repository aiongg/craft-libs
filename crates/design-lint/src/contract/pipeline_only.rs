//! `behavior/pipeline-only`: layers 5 to 7 do not import the behaviours the pipeline runs (spec 20.4).

use crate::report::Finding;

use super::Cx;
use super::sites::Kind;

pub const ID: &str = "behavior/pipeline-only";

/// Behaviours reached through the pipeline only: the press lifecycle, `Gesture`, the focus-scope builders and `TypingRun`.
/// The press types that are plain data every layer names (`PointerButton`, `PointerSample`, `PressStart`) are not listed.
const BEHAVIOURS: [&str; 11] = [
    "PressMachine",
    "PressState",
    "PressPhase",
    "PressStep",
    "PressCancel",
    "PassInput",
    "count_clicks",
    "Gesture",
    "FocusScope",
    "FocusScopeBuilder",
    "TypingRun",
];

/// The module that holds the press lifecycle.
const PRESS_MODULE: [&str; 2] = ["craft_behavior", "press"];

pub(super) fn check(cx: &Cx<'_>, out: &mut Vec<Finding>) {
    if !matches!(cx.scope.layer(cx.path), Some(5..=7)) {
        return;
    }
    for site in &cx.sites {
        let Kind::Path { path, in_use: true, .. } = &site.kind else { continue };
        let name = path.iter().find(|s| BEHAVIOURS.contains(&s.as_str())).cloned().or_else(|| (path == &PRESS_MODULE).then(|| "press".to_owned()));
        if let Some(name) = name {
            cx.flag(
                out,
                ID,
                ID,
                site,
                format!("`use {}` imports `{name}`, a behaviour the pipeline runs; components reach it through the contract", path.join("::")),
            );
        }
    }
}
