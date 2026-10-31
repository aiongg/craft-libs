//! `craft_docpath::names`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Reserved and control characters become `_`, bidi controls go, trailing dots and spaces go,
// "NUL.txt" becomes "_NUL.txt", a 300-byte name is cut to 200 bytes on a char boundary keeping
// ".pdf", and "..." gives the fallback.
#[test]
#[ignore = "skeleton"]
fn user_text_becomes_a_safe_file_name() {
    todo!()
}

// "Report.PDF" with extension "pdf" is "Report"; "COM1" is "_COM1"; "  " is empty.
#[test]
#[ignore = "skeleton"]
fn stems_drop_the_extension_the_app_adds() {
    todo!()
}

// "a.txt", "a 2.txt" taken: Space gives "a 3.txt", Parenthesised "a (2).txt" when only "a.txt"
// is taken; "photo 2.jpg" counts on to "photo 3.jpg"; a name with no extension numbers its end.
#[test]
#[ignore = "skeleton"]
fn taken_names_are_numbered_before_the_extension() {
    todo!()
}
