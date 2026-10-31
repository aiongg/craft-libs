//! `PasswordDialog`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// "abc" is Weak, "abcdef12" Medium, "Abcdefgh1234" Strong; Secret's Debug prints "…".
#[test]
#[ignore = "skeleton"]
fn strength_and_secret_follow_pdfcraft() {
    todo!()
}

// The field has focus on open; Enter answers the typed password with the tag; Esc cancels; the eye
// button switches its name between Show and Hide; an error line shows under the field.
#[test]
#[ignore = "skeleton"]
fn open_prompt_submits_with_enter_and_keeps_focus() {
    todo!()
}

// Set keeps OK disabled until both fields match and aren't empty, and shows the strength line.
#[test]
#[ignore = "skeleton"]
fn set_password_needs_a_matching_second_entry() {
    todo!()
}
