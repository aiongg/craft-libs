//! Workspace set behaviour.

use super::{
    Workspace,
    model::{Layout, Node},
    workspaces::{NamedWorkspace, WorkspaceChrome, WorkspaceCommand, WorkspaceError, WorkspaceLabels, WorkspaceSet},
};
use crate::menu::{ItemKind, MenuEntry};

fn named(id: &str, name: &str) -> NamedWorkspace<&'static str> {
    NamedWorkspace {
        id: id.into(),
        name: name.into(),
        layout: Workspace::default(),
        chrome: WorkspaceChrome::default(),
        keeps_shortcuts: false,
        keeps_menus: false,
    }
}

fn set() -> WorkspaceSet<&'static str> {
    WorkspaceSet::new(vec![named("essentials", "Essentials"), named("painting", "Painting")])
}

/// A layout that differs from the default one.
fn edited() -> Workspace<&'static str> {
    Workspace::new(Layout { root: Some(Node::Tabs { panels: vec!["layers"], active: 0 }), floating: Vec::new() })
}

#[test]
fn save_select_delete_workspaces() {
    let mut set = set();
    assert_eq!(set.current().map(|w| w.id.as_str()), Some("essentials"));
    assert_eq!(set.save("  ", Workspace::default(), WorkspaceChrome::default()), Err(WorkspaceError::EmptyName));
    assert_eq!(set.save("painting", Workspace::default(), WorkspaceChrome::default()), Err(WorkspaceError::NameTaken));

    let id = set.save("My Layout", edited(), WorkspaceChrome::default()).unwrap();
    assert_eq!(id, "my-layout");
    assert_eq!(set.current, id);
    assert_eq!(set.delete(&id), Err(WorkspaceError::CurrentCannotBeDeleted));
    assert_eq!(set.delete("nope"), Err(WorkspaceError::UnknownId));
    assert_eq!(set.delete("painting"), Err(WorkspaceError::UnknownId));
    assert_eq!(set.select("nope", Workspace::default()), Err(WorkspaceError::UnknownId));

    set.select("essentials", edited()).unwrap();
    set.delete(&id).unwrap();
    assert!(set.saved.is_empty());
}

#[test]
fn switching_remembers_live_layout_until_reset() {
    let mut set = set();
    let live = edited();
    set.select("painting", live.clone()).unwrap();
    assert_eq!(set.layout(), Some(&Workspace::default()));
    set.select("essentials", Workspace::default()).unwrap();
    assert_eq!(set.layout(), Some(&live));
    assert_eq!(set.reset(), Some(&Workspace::default()));
    assert_eq!(set.layout(), Some(&Workspace::default()));
}

#[test]
fn menu_rows_mark_current_and_lock() {
    let mut set = set();
    let labels = WorkspaceLabels::default();
    let rows = |set: &WorkspaceSet<&'static str>| {
        set.menu_entries(&labels)
            .into_iter()
            .filter_map(|e| match e {
                MenuEntry::Item(item) => Some((item.command().cloned().unwrap(), item.item_kind())),
                MenuEntry::Separator => None,
            })
            .collect::<Vec<_>>()
    };
    let before = rows(&set);
    assert_eq!(before[0], (WorkspaceCommand::Select("essentials".into()), ItemKind::Check(true)));
    assert_eq!(before[1], (WorkspaceCommand::Select("painting".into()), ItemKind::Check(false)));
    assert_eq!(before.last(), Some(&(WorkspaceCommand::LockToggle, ItemKind::Check(false))));

    set.locked = true;
    set.select("painting", Workspace::default()).unwrap();
    let after = rows(&set);
    assert_eq!(after[1].1, ItemKind::Check(true));
    assert_eq!(after.last().map(|r| r.1), Some(ItemKind::Check(true)));
}
