//! `LinkedFields` link rules.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `follow_ratio`: 100×50 with W 200 gives 200×100; the factor is lowered so no value leaves the range; an old zero scales nothing.
#[test]
#[ignore = "skeleton"]
fn a_ratio_link_scales_the_others_within_the_range() {
    todo!()
}

// `follow_equal` sets every value to the new one.
#[test]
#[ignore = "skeleton"]
fn an_equal_link_copies_the_value() {
    todo!()
}

// In a harness: turning the link on in `Equal` reports one `Set` with every value equal to the first.
#[test]
#[ignore = "skeleton"]
fn linking_equal_fields_copies_the_first() {
    todo!()
}

// In a harness: a scrub of W with the ratio link reports Begin, Updates and one Commit, each with both values;
// typing "100" digit by digit into W of 500×400 ends at 100×80 (the starting ratio, not one drifted per digit).
#[test]
#[ignore = "skeleton"]
fn a_linked_scrub_moves_both_fields_in_one_gesture() {
    todo!()
}
