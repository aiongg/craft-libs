#![cfg(feature = "ui")]
//! The facade lists the stories and words of every enabled crate, the kernel first.

#[test]
fn stories_list_the_kernel_first_then_each_enabled_crate() {
    let ids: Vec<&str> = craft::story::all().iter().map(|s| s.id).collect();
    let kernel = craft::ui::story::registry().len();
    assert!(kernel > 0 && ids.len() > kernel);
    assert!(craft::ui::story::registry().iter().map(|s| s.id).eq(ids[..kernel].iter().copied()));
    #[cfg(feature = "font-manager")]
    assert!(ids[kernel..].contains(&"patterns/fonts-panel/panel"));
}

#[test]
fn words_list_the_kernel_tables_first_then_each_enabled_crate() {
    let tables = craft::words::all();
    let kernel = craft::ui::localized::words::tables();
    assert!(tables.len() > kernel.len());
    assert!(tables.iter().zip(kernel).all(|(a, b)| std::ptr::eq(*a, *b)));
    #[cfg(feature = "keymap")]
    assert!(!craft::words::counts().is_empty());
}

/// The Keyboard Shortcuts dialog lives in craft-menu and registers with the UI contract's registry, not the legacy
/// story list above; the check fails when the dialog's registration is gone.
#[test]
#[cfg(all(feature = "menu", feature = "screen-testing"))]
fn the_menu_crate_registers_the_shortcuts_dialog() {
    let registered = craft::menu::testing::registered();
    assert!(
        registered.iter().any(|r| r.pattern_probe().is_some_and(|probe| probe.type_name().contains("ShortcutsDialog"))),
        "no registration of ShortcutsDialog among {} craft-menu registrations",
        registered.len()
    );
}

/// One id never names two English texts, across every crate's tables and count words.
#[test]
fn a_word_id_has_one_english_text_across_the_crates() {
    let counts: Vec<_> = craft::words::counts().into_iter().flatten().map(|c| c.word).collect();
    let mut seen = std::collections::HashMap::new();
    let mut clashes = Vec::new();
    for word in craft::words::all().into_iter().flatten().copied().chain(counts) {
        if let Some(other) = seen.insert(word.key(), word.english)
            && other != word.english
        {
            clashes.push(format!("{}: {other:?} and {:?}", word.key(), word.english));
        }
    }
    assert!(clashes.is_empty(), "{clashes:#?}");
}
