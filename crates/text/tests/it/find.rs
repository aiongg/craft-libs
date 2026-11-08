//! The find bar and the Find and Replace dialog.

use craft_text::find::{FindError, FindMatches, FindOptions, FindPattern};
use craft_ui::testing::trial;

#[trial]
fn a_refused_pattern_is_an_error_the_ui_can_word() {
    let options = FindOptions { regex: true, ..FindOptions::default() };
    assert!(matches!(FindPattern::compile("(open", &options), Err(FindError::Regex(_))));
}

#[trial]
fn matches_step_through_and_wrap() {
    let mut found = FindMatches::new(0);
    found.set_page(0, vec![0..3, 5..8]);
    assert_eq!(found.current(), Some(0));
    found.step(false);
    assert_eq!(found.current(), Some(1));
    found.step(false);
    assert_eq!(found.current(), Some(0), "stepping past the last match wraps to the first");
}
