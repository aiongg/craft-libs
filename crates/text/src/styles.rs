//! Named styles (N3.04): the rules every style list shares, whatever the styles hold (character,
//! paragraph, table, cell, object, colour or effect styles). Folders come from the name ("Brand/
//! Blue/500" is "500" in folder "Brand/Blue"), a new style takes the next free "Style N", and a
//! list groups styles by folder in order of first appearance.
//!
//! The style attributes, which styles are in use and the commands stay in the app; the list
//! panels are [`crate::ui::style_panel`].

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

/// The separator between a style's folders and its name.
pub const FOLDER_SEPARATOR: char = '/';

/// The folder part of a style name, trimmed: "Brand/Blue" for "Brand/Blue/500", "" for a name
/// without a separator.
// figcraft: crates/ui-egui/src/widgets/style_picker.rs:326 `folder`.
pub fn style_folder(name: &str) -> &str {
    todo!()
}

/// The last part of a style name, trimmed: "500" for "Brand/Blue/500".
// figcraft: crates/ui-egui/src/widgets/style_picker.rs:331 `leaf`.
pub fn style_leaf(name: &str) -> &str {
    todo!()
}

/// `"{base} {n}"` with the smallest n ≥ 1 that no name in `taken` has ("Paragraph Style 3").
/// `base` is the app's translated word ("Character Style", "Paragraph Style", "Style").
// figcraft: crates/ui-egui/src/widgets/style_picker.rs:336 `next_name`.
pub fn next_style_name<'a>(taken: impl IntoIterator<Item = &'a str> + Clone, base: &str) -> String {
    todo!()
}

/// `items` grouped by the folder of `name(item)`: folders in order of first appearance, the
/// top-level styles (folder "") first, each group in the items' order.
// figcraft: crates/ui-egui/src/widgets/style_picker.rs:341 `by_folder`.
pub fn group_by_folder<'a, T>(items: impl IntoIterator<Item = &'a T>, name: impl Fn(&T) -> &str) -> Vec<(String, Vec<&'a T>)>
where
    T: 'a,
{
    todo!()
}

/// The styles a drag from the list carries: every highlighted style in list order when the
/// grabbed row is one of them, else the grabbed style alone (VectorCraft's rows dragged onto New
/// or Delete).
// vectorcraft: crates/ui-egui/src/panels/text_styles.rs:121 `dragged`.
pub fn dragged_styles<K: Clone + PartialEq>(highlighted: &[K], listed: &[K], grabbed: &K) -> Vec<K> {
    todo!()
}
