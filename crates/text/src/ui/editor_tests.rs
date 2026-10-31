//! The editor over an in-memory host (monospace layout: one char per unit, lines at `\n`) in a
//! harness.

// Typed letters go in at the caret through `replace(.., Typed)`; with a selection they replace it.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn typing_inserts_at_the_caret_and_replaces_the_selection() {
    todo!()
}

// With `smart_quotes(Some(ENGLISH))` a typed `"` after a space is “; a pasted `"` stays straight.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn typed_quotes_follow_the_style_and_pasted_text_keeps_them() {
    todo!()
}

// Preedit "k", then "か": the host text holds "か" at the caret (kind Composing); commit "課" replaces it (Committed).
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn marked_text_is_written_into_the_text_and_replaced() {
    todo!()
}

// A press outside while composing keeps the marked text as typed and returns false.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn click_away_keeps_the_marked_text() {
    todo!()
}

// `text_replaced` after the host undid the marked text clamps the selection and drops the preedit range.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn external_change_drops_the_marked_range() {
    todo!()
}

// ← → by char, ⌘← → by word, ↑ ↓ through `line_step`, Home/End through `line_edge`; Shift extends.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn arrows_words_lines_and_shift_extend() {
    todo!()
}

// ⌫ deletes a char, ⌥⌫ a word, ⌘⌫ to the line start; ⌦ and ⌥⌦ forward.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn backspace_and_delete_by_char_word_and_line() {
    todo!()
}

// Two to five quick clicks select the word, line, paragraph and all; the sixth places a caret.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn click_runs_select_word_line_paragraph_all() {
    todo!()
}

// ⇧⌘B is `Toggle(FauxBold)`; ⌥→ at a caret is `Kern(20.0)`, with a selection `Tracking(20.0)`, ⌘ makes it 100.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn style_keys_reach_the_host() {
    todo!()
}

// Esc sets `commit` by default and `cancel` with `esc_commits(false)`; ⌘↩ commits.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn escape_commits_or_cancels() {
    todo!()
}

// In vertical text ↓ moves to the next char and ← to the next column.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn vertical_text_turns_the_arrows() {
    todo!()
}

// `insert` puts a glyph at the selection with no smart quotes and moves the caret after it; `cut`
// copies the selection to the clipboard output and deletes it.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn menu_insert_copy_and_cut() {
    todo!()
}
