//! PhotoCraft parity in the keymap.

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// Cmd+Enter displays as ⌘↩ with ReturnGlyph::Return and ⌘⏎ with Enter; Ctrl+Enter on Windows either way.
fn return_glyph_on_mac() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// Mods{shift,alt} displays ⇧⌥ on macOS (Apple order) and Shift+Alt on Windows; no modifiers is empty.
fn modifier_only_hints() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// Two tools of different groups on V conflict by default and don't with with_shared_tool_keys(true); V cycles through both.
fn tools_share_keys_when_allowed() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// An override text that doesn't parse comes back from unparsed_texts and the lookup falls back to the default.
fn unparsed_override_texts_are_kept() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// Keymap::remove on a command's first binding leaves its other default bindings bound (gap 84).
fn removing_the_main_binding_keeps_the_alternates() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// With delete equivalence on, a binding on Backspace runs from Delete and the reverse; off, only the exact key.
fn delete_and_backspace_match_each_other() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// On a German layout (ü at BracketLeft) a binding on [ matches the BracketLeft press with position fallback on.
fn brackets_match_at_their_us_position() {
    todo!()
}
