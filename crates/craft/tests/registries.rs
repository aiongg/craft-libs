#![cfg(feature = "ui")]
//! The crates register their UI contract entries with craft-ui's registry.

/// The Keyboard Shortcuts dialog lives in craft-menu and registers with the UI contract's registry; the check fails
/// when the dialog's registration is gone.
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
