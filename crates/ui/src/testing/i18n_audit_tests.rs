//! Behaviour tests of `testing::i18n_audit`.
use super::*;

fn frame(texts: &[&str]) -> Vec<String> {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        for t in texts {
            ui.label(*t);
        }
    });
    output.textures_delta.clear();
    drawn_texts(&output)
}

// A frame with one label through Words in the pseudo locale and one literal "Save" reports only "Save", with its state.
#[test]
fn literal_text_is_found_and_marked_text_is_not() {
    // `⟦…⟧` is what a Words lookup returns in the pseudo locale.
    let drawn = frame(&["⟦Open⟧", "Save"]);
    assert_eq!(drawn.len(), 2);
    let mut audit = Audit::new();
    audit.record("dialog save", drawn, &Exempt::new());
    assert_eq!(audit.findings(), vec![("Save", vec!["dialog save"])]);
    assert_eq!(audit.report(), "\"Save\"  [dialog save]\n");
}

// Shortcuts, two-capital initials, names, body text and prefixes are allowed; an ordinary word is not.
#[test]
fn exempt_text_is_allowed() {
    let exempt = Exempt::new().names(["Aptos Display"]).body("Chapter One").body("Second line").prefixes(["@", "Style"]);
    for ok in ["Ctrl+Shift+S", "⟦Copy⟧ Ctrl+C", "AB", "Aptos Display", "Chapter One", "Second line", "@someone", "Style 3"] {
        assert!(exempt.allows(ok), "{ok}");
    }
    for bad in ["Save", "Ctrl+Save", "Aptos"] {
        assert!(!exempt.allows(bad), "{bad}");
    }
    assert!(!unmarked("⟦a⟦b⟧c⟧ ok ⟧x").contains('⟧'));
    assert!(!english_words("⟦Open⟧ 12"));
}
