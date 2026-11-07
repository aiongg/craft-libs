//! The component tree: modules as groups, components under them, narrowed by the filter.

use super::catalog::ComponentInfo;
use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::list::OpenSet;
use std::borrow::Cow;
use std::collections::BTreeMap;

/// The module a component without a probe is listed under.
const UNDECLARED: &str = "Other";

/// The key of a tree row.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TreeKey {
    /// A module group.
    Module(&'static str),
    /// A component, by its registered type name.
    Component(&'static str),
}

/// One shown row of the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TreeRow {
    /// The row's key.
    pub key: TreeKey,
    /// The text the row shows.
    pub label: String,
    /// 0 for a module, 1 for a component.
    pub depth: usize,
    /// Whether the row holds components.
    pub group: bool,
    /// Whether a group shows its components.
    pub open: bool,
}

/// The module group a component is listed under.
pub fn module_of(component: &ComponentInfo) -> &'static str {
    if component.module.is_empty() { UNDECLARED } else { component.module }
}

/// The rows of the tree for `filter`. A filter opens every group that holds a match, so the matches show.
pub fn rows(components: &[ComponentInfo], filter: &str, open: &OpenSet<TreeKey>) -> Vec<TreeRow> {
    let filtering = !filter.trim().is_empty();
    let mut modules: BTreeMap<&'static str, Vec<&ComponentInfo>> = BTreeMap::new();
    for component in components.iter().filter(|component| component.matches(filter)) {
        modules.entry(module_of(component)).or_default().push(component);
    }
    let mut rows = Vec::new();
    for (module, members) in modules {
        let key = TreeKey::Module(module);
        let shown = filtering || open.is_open(&key);
        rows.push(TreeRow { key, label: module.to_owned(), depth: 0, group: true, open: shown });
        if shown {
            rows.extend(members.into_iter().map(|component| TreeRow {
                key: TreeKey::Component(component.type_name),
                label: component.short_name(),
                depth: 1,
                group: false,
                open: false,
            }));
        }
    }
    rows
}

/// The groups of `rows`, for an Alt-click that opens or closes every group.
pub fn groups(rows: &[TreeRow]) -> Vec<TreeKey> {
    rows.iter().filter(|row| row.group).map(|row| row.key.clone()).collect()
}

/// `rows` as the display order of the list.
pub struct TreeRows<'a>(&'a [TreeRow]);

impl<'a> TreeRows<'a> {
    /// The display order of `rows`.
    pub fn new(rows: &'a [TreeRow]) -> Self {
        TreeRows(rows)
    }
}

impl DisplayOrder<TreeKey> for TreeRows<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> TreeKey {
        self.0[at].key.clone()
    }

    fn depth(&self, at: usize) -> usize {
        self.0[at].depth
    }

    fn has_children(&self, at: usize) -> bool {
        self.0[at].group
    }

    fn open(&self, at: usize) -> bool {
        self.0[at].open
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.0[at].label.as_str()))
    }
}

impl ItemSource<TreeKey> for TreeRows<'_> {}
