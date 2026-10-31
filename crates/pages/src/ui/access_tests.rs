//! The accessibility bridge.

// Without AccessKit output requested the describer is never called.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn nothing_is_built_without_a_screen_reader() {
    todo!()
}

// A describer with a heading, a two-item list and a 2 × 2 table yields Document › Heading, List › ListItem × 2 with markers, Table › Row › Cell, each line a TextRun with character positions.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn paragraphs_lines_lists_and_tables_form_the_tree() {
    todo!()
}

// 7000 blocks stop at MAX_NODES; the same revision and placement reuse last pass's tree without calling the describer.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn the_tree_is_capped_and_reused() {
    todo!()
}
