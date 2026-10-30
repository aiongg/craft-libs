//! The context menu (DS-44 task 4).

use std::hash::Hash;
use std::rc::Rc;

use egui::{Id, Response, Ui};

use super::event::PresetEvent;
use super::labels::PresetLabels;
use super::model::PresetRow;
use super::view::{MenuRequest, State};
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};

/// An entry the host adds to the context menu; choosing it reports [`PresetEvent::Extra`].
#[derive(Clone, Debug)]
pub struct PresetExtra {
    pub(crate) id: &'static str,
    pub(crate) label: String,
    pub(crate) enabled: bool,
}

impl PresetExtra {
    /// An entry named `label` that reports `id`.
    pub fn new(id: &'static str, label: impl Into<String>) -> Self {
        Self { id, label: label.into(), enabled: true }
    }

    /// Whether the entry can be chosen (default yes).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// A test the host gives for one key: may it be renamed, deleted, moved.
pub(super) type KeyTest<'a, K> = Rc<dyn Fn(&K) -> bool + 'a>;

/// What the menu is built from.
pub(super) struct Spec<'a, K> {
    pub rows: &'a [PresetRow<K>],
    pub labels: &'a PresetLabels,
    pub can_rename: Option<&'a KeyTest<'a, K>>,
    pub can_delete: Option<&'a KeyTest<'a, K>>,
    pub can_move: Option<&'a KeyTest<'a, K>>,
    pub extras: &'a [PresetExtra],
}

#[derive(Clone)]
enum Command<K> {
    Rename,
    Delete,
    MoveTo(K),
    Extra(&'static str),
}

/// The entries for the preset or group at `at`.
fn entries<K: Clone + Eq>(spec: &Spec<'_, K>, at: usize) -> Vec<MenuEntry<Command<K>>> {
    let row = &spec.rows[at];
    let allowed = |test: Option<&KeyTest<'_, K>>| test.is_none_or(|t| t(&row.key));
    let l = spec.labels;
    let group = row.is_group();
    let mut out: Vec<MenuEntry<Command<K>>> = vec![
        MenuItem::new(if group { &l.rename_group } else { &l.rename }, Command::Rename).enabled(allowed(spec.can_rename)).into(),
        MenuItem::new(if group { &l.delete_group } else { &l.delete }, Command::Delete).enabled(allowed(spec.can_delete)).into(),
    ];
    if !group && allowed(spec.can_move) {
        let targets = move_targets(spec, at);
        if !targets.is_empty() {
            let children = targets.into_iter().map(|(key, path)| MenuItem::new(path, Command::MoveTo(key)).into()).collect();
            out.push(MenuItem::submenu(l.move_to.as_str(), children).into());
        }
    }
    if !spec.extras.is_empty() {
        out.push(MenuEntry::Separator);
        out.extend(spec.extras.iter().map(|e| MenuEntry::from(MenuItem::new(e.label.as_str(), Command::Extra(e.id)).enabled(e.enabled))));
    }
    out
}

/// Every group but the one the item at `at` is in, with its path of ancestors.
fn move_targets<K: Clone + Eq>(spec: &Spec<'_, K>, at: usize) -> Vec<(K, String)> {
    let item_depth = spec.rows[at].depth;
    let parent = spec.rows[..at].iter().rposition(|r| r.is_group() && r.depth < item_depth);
    let mut path: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for (i, r) in spec.rows.iter().enumerate().filter(|(_, r)| r.is_group()) {
        path.truncate(r.depth);
        path.push(&r.label);
        if Some(i) != parent {
            out.push((r.key.clone(), path.join(&spec.labels.path_separator)));
        }
    }
    out
}

/// Opens the context menu `request` asked for, keeps it open on its preset or group until it
/// closes, and reports what was chosen. `browser` is the browser's id and `response` its response.
pub(super) fn show<K>(ui: &mut Ui, browser: Id, response: &Response, spec: &Spec<'_, K>, events: &mut Vec<PresetEvent<K>>)
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
{
    let ctx = ui.ctx().clone();
    let memory = browser.with("menu-key");
    let mut state = State::<K>::load(&ctx, browser);
    let request: Option<MenuRequest<K>> = state.menu.take();
    let open = ContextMenu::is_open(&ctx, response);
    let mut key = ctx.data(|d| d.get_temp::<Option<K>>(memory)).flatten();
    let mut at = None;
    if let Some(r) = request {
        key = Some(r.key);
        at = Some(r.pointer.map_or(OpenAt::Rect(r.anchor), OpenAt::Pointer));
    } else if !open {
        key = None;
    }
    let row = key.as_ref().and_then(|k| spec.rows.iter().position(|r| r.key == *k));
    let Some((key, row)) = key.zip(row) else {
        ctx.data_mut(|d| d.insert_temp(memory, None::<K>));
        state.store(&ctx, browser);
        return;
    };
    let entries = entries(spec, row);
    let out = ContextMenu::new(&entries).own_gestures(false).open_at(at).show(ui, response);
    ctx.data_mut(|d| d.insert_temp(memory, (out.open || at.is_some()).then(|| key.clone())));
    match out.chosen {
        Some(Command::Rename) => state.renaming = Some(key),
        Some(Command::Delete) => events.push(PresetEvent::Delete(key)),
        Some(Command::MoveTo(group)) => events.push(PresetEvent::MoveTo { key, group }),
        Some(Command::Extra(id)) => events.push(PresetEvent::Extra { key, id }),
        None => {}
    }
    state.store(&ctx, browser);
}
