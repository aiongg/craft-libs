//! The Select Text tool in a `ToolHost` over a mock page text.

// A press at glyph 3 and a drag to glyph 10 select Run { anchor: 3, head: 10 }, changed() is true and text_of gives the text; Esc during the drag cancels it.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn drag_selects_in_reading_order() {
    todo!()
}

// Alt-drag over a two-column mock page selects only the left column's glyphs, one line per row in its text.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn alt_drag_selects_a_column() {
    todo!()
}

// ToolPointer clicks 2, 3 and 4 select the word, the line and the page.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn multi_clicks_select_word_line_and_page() {
    todo!()
}
