//! `PresetManager`: a dialog body for managing the presets of several kinds: a kind select over the presets of that
//! kind, with rename and delete, and the host's own action buttons.

use super::event::PresetEvent;
use super::model::PresetRow;
use crate::contract::{Bind, Entry, NoValue, Outcome, Pattern, Props};
use crate::dialog::{DialogId, DialogResult, DialogStack, StackDialog, dialog_id};
use crate::list::RowTest;
use crate::runtime::env::Label;
use crate::runtime::overlay::CloseReason;
use craft_behavior::Value;
use egui::{Response, Ui};
use std::hash::Hash;

/// What a [`PresetManager`] took from the user in a pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct PresetManagerTaken<G, K> {
    /// The kind the user picked in this pass.
    pub kind: Option<G>,
    /// The renames, deletes and picks of the rows, in order.
    pub events: Vec<PresetEvent<K>>,
    /// The indices of the action buttons pressed, in order.
    pub actions: Vec<usize>,
}

/// What a [`PresetManager`] reports.
pub struct PresetManagerOutput<G, K> {
    /// The manager's response.
    pub response: Response,
    /// How the manager closed in this pass; `None` while it stays open.
    pub closed: Option<CloseReason>,
    /// What the user did this pass.
    pub taken: PresetManagerTaken<G, K>,
}

impl<G, K> Outcome for PresetManagerOutput<G, K> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The preset manager: a select of preset `kinds` over the list of the picked kind's `rows`, with Rename and Delete
/// on each row and buttons for the host's own `actions` (Load, Save, Reset).
#[must_use = "show the manager with `show`"]
#[derive(Pattern, Props)]
#[pattern(probe(PresetManager<'static, u32, u32>))]
pub struct PresetManager<'a, G: Value, K: Clone + Eq + Hash + Send + Sync + 'static> {
    title: Label,
    kinds: &'a [Entry<G>],
    kind: Bind<'a, G>,
    rows: &'a [PresetRow<K>],
    /// The host's buttons: a label and a tooltip each.
    #[prop]
    actions: Vec<(Label, Option<Label>)>,
    /// Which presets Rename applies to.
    #[prop(func = "Fn(&K) -> bool")]
    can_rename: Option<RowTest<'a, K>>,
    /// Which presets Delete applies to.
    #[prop(func = "Fn(&K) -> bool")]
    can_delete: Option<RowTest<'a, K>>,
    /// What the list shows when `rows` is empty.
    #[prop(into)]
    empty: Option<Label>,
}

impl<'a, G: Value, K: Clone + Eq + Hash + Send + Sync + 'static> PresetManager<'a, G, K> {
    /// A manager titled `title` over the preset `kinds`, with `kind` the picked one and `rows` its presets.
    pub fn new(title: impl Into<Label>, kinds: &'a [Entry<G>], kind: Bind<'a, G>, rows: &'a [PresetRow<K>]) -> Self {
        let _ = (title.into(), kinds, kind, rows);
        todo!("tp.T34")
    }

    /// Shows the manager.
    pub fn show(self, ui: &mut Ui) -> PresetManagerOutput<G, K> {
        let _ = (ui, &self.title, self.kinds, &self.kind, self.rows);
        todo!("tp.T34")
    }
}

/// A [`PresetManager`] the [`DialogHost`](crate::dialog::DialogHost) owns. The app reads what the user did with
/// [`take`](HostedPresetManager::take).
pub struct HostedPresetManager<G, K> {
    id: DialogId,
    title: Label,
    kinds: Vec<Entry<G>>,
    kind: G,
    rows: Vec<PresetRow<K>>,
    taken: PresetManagerTaken<G, K>,
}

impl<G: Value, K: Clone + Eq + Hash + Send + Sync + 'static> HostedPresetManager<G, K> {
    /// A hosted manager titled `title` over the preset `kinds`, opening on `kind`.
    pub fn new(title: impl Into<Label>, kinds: Vec<Entry<G>>, kind: G) -> Self {
        let _ = (dialog_id("preset-manager"), title.into(), kinds, kind);
        todo!("tp.T34")
    }

    /// Sets the presets of the picked kind.
    pub fn rows(&mut self, rows: Vec<PresetRow<K>>) {
        let _ = rows;
        todo!("tp.T34")
    }

    /// The kind the manager shows.
    pub fn kind(&self) -> &G {
        &self.kind
    }

    /// What the user did since the last call.
    pub fn take(&mut self) -> PresetManagerTaken<G, K> {
        todo!("tp.T34")
    }
}

impl<G: Value + Send + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> StackDialog for HostedPresetManager<G, K> {
    type Result = ();

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ui: &mut Ui, stack: &DialogStack) -> Option<DialogResult<()>> {
        let _ = (ui, stack, &self.title, &self.kinds, &self.rows, &self.taken);
        todo!("tp.T34")
    }

    fn json_fields(&self) -> Option<serde_json::Value> {
        todo!("tp.T34")
    }

    fn set_json_field(&mut self, ctx: &egui::Context, field: &str, value: serde_json::Value) -> Result<(), String> {
        let _ = (ctx, field, value);
        todo!("tp.T34")
    }
}
