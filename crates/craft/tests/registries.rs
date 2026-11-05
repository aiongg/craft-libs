#![cfg(feature = "ui")]
//! The facade lists the stories and words of every enabled crate, the kernel first.

#[test]
fn stories_list_the_kernel_first_then_each_enabled_crate() {
    let ids: Vec<&str> = craft::story::all().iter().map(|s| s.id).collect();
    let kernel = craft::ui::story::registry().len();
    assert!(kernel > 0 && ids.len() > kernel);
    assert!(craft::ui::story::registry().iter().map(|s| s.id).eq(ids[..kernel].iter().copied()));
    #[cfg(feature = "keymap")]
    assert!(ids[kernel..].contains(&"patterns/shortcuts-dialog/dialog"));
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
    assert!(tables.iter().any(|t| *t == craft::keymap::ui::WORDS[0]));
    #[cfg(feature = "keymap")]
    assert!(!craft::words::counts().is_empty());
}

#[test]
#[cfg(feature = "platform")]
fn words_list_the_platform_messages() {
    let words: Vec<_> = craft::words::all().into_iter().flatten().copied().collect();
    assert!(words.iter().any(|w| w.id == "open.failed"));
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
