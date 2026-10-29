//! What the gallery keeps between runs (eframe storage on desktop, local storage on the web).

use std::collections::HashMap;

use craft_ui::docking::Workspace;
use craft_ui::story::{ArgValue, Args};
use serde::{Deserialize, Serialize};

use crate::appearance::Appearance;
use crate::layout::PanelId;

pub const KEY: &str = "craft-ui-gallery";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub appearance: Appearance,
    pub selected: Option<String>,
    pub filter: String,
    pub compare: bool,
    /// The dock: where each panel is. Validated against the default when loaded.
    pub layout: Option<Workspace<PanelId>>,
    /// Theme ids in the compare grid.
    pub compare_themes: Vec<String>,
    /// The compare grid's high-contrast columns.
    pub compare_high: bool,
    /// Edited args by story id.
    pub args: HashMap<String, Vec<SavedArg>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedArg {
    pub name: String,
    pub value: SavedValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SavedValue {
    Bool(bool),
    Choice(usize),
    Text(String),
    Number(f64),
}

impl Saved {
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Saved {
        storage.and_then(|s| eframe::get_value(s, KEY)).unwrap_or_default()
    }
}

/// The saved form of `args`.
pub fn save_args(args: &Args) -> Vec<SavedArg> {
    args.iter()
        .map(|a| SavedArg {
            name: a.name.to_owned(),
            value: match &a.value {
                ArgValue::Bool(b) => SavedValue::Bool(*b),
                ArgValue::Choice { index, .. } => SavedValue::Choice(*index),
                ArgValue::Text(t) => SavedValue::Text(t.clone()),
                ArgValue::Number { value, .. } => SavedValue::Number(*value),
            },
        })
        .collect()
}

/// `defaults` with the saved values applied; entries that no longer match an arg's name or kind
/// are dropped.
pub fn restore_args(mut defaults: Args, saved: &[SavedArg]) -> Args {
    for arg in defaults.iter_mut() {
        let Some(found) = saved.iter().find(|s| s.name == arg.name) else {
            continue;
        };
        match (&mut arg.value, &found.value) {
            (ArgValue::Bool(b), SavedValue::Bool(v)) => *b = *v,
            (ArgValue::Choice { options, index }, SavedValue::Choice(v)) => *index = (*v).min(options.len().saturating_sub(1)),
            (ArgValue::Text(t), SavedValue::Text(v)) => t.clone_from(v),
            (ArgValue::Number { value, min, max }, SavedValue::Number(v)) => *value = v.clamp(*min, *max),
            _ => {}
        }
    }
    defaults
}
