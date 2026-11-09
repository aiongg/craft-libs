//! PhotoCraft parity in the keymap.

use crate::chord::{Chord, Key, Platform};
use crate::keymap::{Binding, CommandDef, CommandKind, Keymap, Lookup};
use crate::scope::Active;
use crate::{Scope, Scopes};

fn tool(id: &str, group: &str, key: &str) -> CommandDef {
    CommandDef::new(id, id, Scope::app()).with_kind(CommandKind::Tool { group: Some(group.to_owned()) }).with_default(Binding::parse(key).unwrap())
}

#[test]
fn tools_share_keys_when_allowed() {
    let catalog = || vec![tool("tool.move", "a", "V"), tool("tool.artboard", "b", "V")];
    let km = Keymap::new(catalog(), Scopes::new(), Platform::Windows);
    assert!(!km.shares_tool_keys());
    assert_eq!(km.conflicts().len(), 1);
    let v = Chord::plain(Key::V);
    assert!(matches!(km.lookup(&v, false, &Active::NONE), Lookup::Run(_)));

    let km = Keymap::new(catalog(), Scopes::new(), Platform::Windows).with_shared_tool_keys(true);
    assert!(km.shares_tool_keys());
    assert!(km.conflicts().is_empty());
    let Lookup::Cycle(cycle) = km.lookup(&v, false, &Active::NONE) else { panic!("V should cycle") };
    assert_eq!(cycle.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["tool.move", "tool.artboard"]);
}
