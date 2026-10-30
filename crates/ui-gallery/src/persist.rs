//! What the gallery keeps between runs (eframe storage on desktop, local storage on the web).

use std::collections::HashMap;

use craft_ui::docking::Workspace;
use craft_ui::story::{ArgValue, Args};
use serde::{Deserialize, Serialize};

use crate::appearance::Appearance;
use crate::layout::{GalleryWorkspace, PanelId, PreviewView};

pub const KEY: &str = "craft-ui-gallery";

/// The dock layout has a key of its own: a layout this build cannot read (a newer format) then
/// costs only the layout, not the rest of [`Saved`]. The suffix is the layout's version: layouts
/// saved under an earlier version (the token browser in the centre) are not read.
pub const LAYOUT_KEY: &str = "craft-ui-gallery.layout.v2";

/// The Theme workspace's dock layout, kept apart from the Components one so that neither costs
/// the other when it cannot be read.
pub const THEME_LAYOUT_KEY: &str = "craft-ui-gallery.theme-layout.v1";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub appearance: Appearance,
    /// The story selected before the Overview and the component tabs kept their own: read once as
    /// the Overview's selection.
    pub selected: Option<String>,
    /// The component the Overview has selected.
    pub overview_selected: Option<String>,
    /// The slug of the level the Overview shows.
    pub overview_level: Option<String>,
    /// The story id each Overview tile shows, by component name.
    pub overview_story: HashMap<String, String>,
    /// The selected story id of each component tab, by component name.
    pub tab_story: HashMap<String, String>,
    pub filter: String,
    pub workspace: GalleryWorkspace,
    pub preview: PreviewView,
    /// The share of the Tokens panel's height that its list takes.
    pub tokens_split: Option<f32>,
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

/// The saved dock layout, if there is one and it reads.
pub fn load_layout(storage: Option<&dyn eframe::Storage>) -> Option<Workspace<PanelId>> {
    storage.and_then(|s| eframe::get_value(s, LAYOUT_KEY))
}

/// The saved Theme dock layout, if there is one and it reads.
pub fn load_theme_layout(storage: Option<&dyn eframe::Storage>) -> Option<Workspace<PanelId>> {
    storage.and_then(|s| eframe::get_value(s, THEME_LAYOUT_KEY))
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
