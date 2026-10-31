#![cfg(feature = "ui")]
//! The facade lists the stories and words of every enabled crate, the kernel first.

#[test]
fn stories_list_the_kernel_first_then_each_enabled_crate() {
    let ids: Vec<&str> = craft::story::all().iter().map(|s| s.id).collect();
    let kernel = craft::ui::story::registry().len();
    assert!(kernel > 0 && ids.len() > kernel);
    assert!(craft::ui::story::registry().iter().map(|s| s.id).eq(ids[..kernel].iter().copied()));
    #[cfg(feature = "keymap")]
    assert!(ids[kernel..].contains(&"patterns/keyboard-shortcuts/dialog"));
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
