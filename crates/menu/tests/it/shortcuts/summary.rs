//! The shortcut summary: HTML and plain text.

use std::cell::RefCell;
use std::rc::Rc;

use craft_keymap::{Bindable, Binding, Keymap, Platform, Scope, Scopes};
use craft_menu::shortcuts::{Outcome, summary};
use craft_ui::testing::{Fixture, trial};
use egui::Ui;

use super::fixture::{Rig, keymap};

fn tricky_keymap() -> Keymap {
    let mut catalog = keymap(Platform::Mac).catalog().to_vec();
    let label = "<b>\"Tom\" & 'Jerry'</b>\nNext";
    catalog.push(
        Bindable::new("edit.tricky", label, Scope::app())
            .with_path(vec!["Edit".into(), "Tricky".into()])
            .with_default(Binding::parse("F6").unwrap_or_else(|e| panic!("{e}"))),
    );
    Keymap::new(catalog, Scopes::new().with_panel("Timeline").with_context("Text"), Platform::Mac)
}

/// Labels are escaped in the HTML and flattened to one line in the text, in both the dialog's Summarize and the
/// functions an app calls for a cheat sheet.
#[trial]
fn summary_escapes_html_and_keeps_text_lines() {
    let mut rig = Rig::new(tricky_keymap(), super::fixture::options(), None);
    rig.click("Summarize…");
    let (html, text) = rig
        .outcomes()
        .into_iter()
        .find_map(|o| if let Outcome::Summary { html, text, .. } = o { Some((html, text)) } else { None })
        .unwrap_or_else(|| panic!("Outcome::Summary"));
    assert!(html.contains("&lt;b&gt;&quot;Tom&quot; &amp; &#39;Jerry&#39;&lt;/b&gt;"), "{html}");
    assert!(!html.contains("<b>"));
    assert!(html.contains("<kbd dir=\"ltr\">⌘S</kbd>"));
    assert!(text.lines().any(|l| l == "  <b>\"Tom\" & 'Jerry'</b> Next\tF6"), "{text}");
    assert!(text.lines().any(|l| l == "  Save\t⌘S"), "{text}");

    let (km, shown) = (tricky_keymap(), Rc::new(RefCell::new(None)));
    let out = shown.clone();
    let mut fixture =
        Fixture::new(move |ui: &mut Ui| *out.borrow_mut() = Some((summary::html(&km, ui, "Shortcuts"), summary::text(&km, ui, "Shortcuts"))));
    fixture.settle();
    let (page, plain) = shown.take().unwrap_or_else(|| panic!("the pass ran"));
    assert!(page.contains("<title>Shortcuts</title>") && plain.starts_with("Shortcuts\n"));
}
