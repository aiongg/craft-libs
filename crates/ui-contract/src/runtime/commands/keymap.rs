//! The keymap the command board is built from: the linked command sets and the app's catalog on a platform, under the
//! user's overrides, with the chords it binds to the edit commands and to the window's keys.

use super::{AppCatalog, WindowShortcut, registry, yields_default};
use craft_behavior::EditCommand;
use craft_keymap::{Chord, CommandDef, CommandRef, Keymap, Overrides, Platform, PressKind, Problem, Scopes, Trigger, TypingRule};
use std::collections::HashMap;
use std::sync::Arc;

/// A keymap with the chords it binds to the edit commands and to the window's keys.
#[derive(Clone)]
pub(crate) struct BuiltKeymap {
    pub(crate) keymap: Arc<Keymap>,
    pub(crate) edit_chords: Arc<[(EditCommand, Chord)]>,
    pub(crate) window_shortcuts: Arc<[(WindowShortcut, Chord)]>,
    /// The text of every chord the keymap shows, written once when the keymap is built.
    pub(crate) shown_text: Arc<HashMap<Chord, Arc<str>>>,
}

/// The keymap of `defs` on `platform` under `overrides`, with the system chords reserved, the app's typing rule when it
/// set one, and hold bindings counting when `holds`.
fn keymap_of(
    defs: Vec<CommandDef>,
    scopes: Scopes,
    platform: Platform,
    overrides: Option<&Arc<Overrides>>,
    typing: Option<TypingRule>,
    holds: bool,
) -> Keymap {
    let mut keymap = Keymap::new(defs, scopes, platform).with_reserved(craft_keymap::reserved::system(platform));
    if let Some(rule) = typing {
        keymap = keymap.with_typing_rule(rule);
    }
    keymap.set_holds(holds);
    if let Some(overrides) = overrides {
        keymap.set_overrides((**overrides).clone());
    }
    keymap
}

#[cfg(feature = "testing")]
thread_local! {
    static CHORD_TEXT_BUILDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many chord texts this thread's keymap builds have written: one per distinct shown chord per build, never one
/// per report.
#[cfg(feature = "testing")]
#[doc(hidden)]
pub fn chord_text_builds() -> u64 {
    CHORD_TEXT_BUILDS.with(std::cell::Cell::get)
}

#[cfg(feature = "testing")]
thread_local! {
    static KEYMAP_BUILDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many keymaps this thread has built: a frame builds one only when the platform, the overrides or the app
/// catalog's generation differ from the last built one's.
#[cfg(feature = "testing")]
#[doc(hidden)]
pub fn keymap_builds() -> u64 {
    KEYMAP_BUILDS.with(std::cell::Cell::get)
}

/// The keymap of `registry` and the app catalog `app` on `platform` under `overrides`, and the chords it binds to the
/// edit commands and to the window's keys. A catalog command whose id the registry has keeps the registry's
/// definition, and the catalog's default bindings join the registry's after them (an app's alternate keys, such as
/// F2/F3/F4 for Cut/Copy/Paste).
///
/// A yielding library command's default chord (a window shortcut's, Quit's: `yields_default`) that another command also
/// binds is that command's: the library command loses it, so the app's binding beats the framework's default. One the
/// overrides name keeps what they give it.
pub(crate) fn build_keymap(
    registry: &registry::Registry,
    app: Option<&AppCatalog>,
    platform: Platform,
    overrides: Option<&Arc<Overrides>>,
    holds: bool,
) -> BuiltKeymap {
    #[cfg(feature = "testing")]
    KEYMAP_BUILDS.with(|builds| builds.set(builds.get() + 1));
    // A held scope and a typing scope are contexts of the keymap: their keys work while they are active, before any
    // panel's, and a typing scope's while typing too.
    let mut scopes = Scopes::new();
    for def in registry.defs() {
        if registry.is_held_scope(def.scope.as_str()) || registry.is_typing_scope(def.scope.as_str()) {
            scopes = scopes.with_context(def.scope.as_str());
        } else if !def.scope.is_app() {
            scopes = scopes.with_panel(def.scope.as_str());
        }
    }
    let app_defs: Vec<CommandDef> =
        app.map(|app| app.defs().iter().filter(|def| registry.get(&def.command.id).is_none()).cloned().collect()).unwrap_or_default();
    // The window-shortcut yield and the edit chords below read the registry's definitions only.
    let joined = |defs: &[CommandDef]| -> Vec<CommandDef> { defs.iter().chain(&app_defs).cloned().collect() };
    let mut defs = registry.defs_for(platform);
    if let Some(app) = app {
        // The app's defaults for a linked set's command replace the set's, before the app's alternate keys join them.
        for def in &mut defs {
            if let Some(bindings) = app.replaced_defaults(&def.command.id) {
                def.defaults = bindings.to_vec();
            }
        }
        for extra in app.defs() {
            if let Some(def) = defs.iter_mut().find(|def| def.command == extra.command) {
                for binding in &extra.defaults {
                    if !def.defaults.contains(binding) {
                        def.defaults.push(binding.clone());
                    }
                }
            }
        }
    }
    let typing = app.and_then(AppCatalog::typing);
    let mut keymap = keymap_of(joined(defs.as_slice()), scopes.clone(), platform, overrides, typing, holds);
    let overridden = |def: &CommandDef| overrides.is_some_and(|overrides| overrides.get(&def.command.id).is_some());
    let yielded: Vec<(CommandRef, Trigger)> = defs
        .iter()
        .filter(|def| yields_default(&def.command) && !overridden(def))
        .flat_map(|def| keymap.conflicts_with(&def.command))
        .filter(|conflict| !yields_default(&conflict.b.0))
        .map(|conflict| (conflict.a.0, conflict.chord))
        .collect();
    if !yielded.is_empty() {
        for def in &mut defs {
            def.defaults
                .retain(|binding| !yielded.iter().any(|(command, chord)| *command == def.command && binding.trigger.effective(platform) == *chord));
        }
        keymap = keymap_of(joined(defs.as_slice()), scopes, platform, overrides, typing, holds);
    }
    // An override whose key names no command binds nothing and leaves that command's defaults in place; a misspelt id in
    // an app's own table otherwise fails without a trace.
    for problem in keymap.problems() {
        if let Problem::UnknownCommand { key } = problem {
            log::warn!("shortcut override `{key}` names no command and binds nothing");
        }
    }
    let window_shortcuts = registry
        .defs()
        .iter()
        .filter_map(|def| WindowShortcut::of(&def.command).map(|key| (key, &def.command)))
        .flat_map(|(key, command)| {
            keymap
                .bindings(command)
                .into_iter()
                .filter(|binding| binding.trigger.press == PressKind::Single)
                .map(move |binding| (key, binding.trigger.chord))
        })
        .collect();
    let edit_chords = registry
        .defs()
        .iter()
        .filter_map(|def| registry.edit_of(&def.command.id).map(|edit| (edit, &def.command)))
        // Delete's keys (Delete, Backspace) stay a focused text part's own deletion, which also deletes one character.
        .filter(|(edit, _)| *edit != EditCommand::Delete)
        .flat_map(|(edit, command)| {
            // Only the platform's own text-editing chords: an app's alternate key (F4 for Paste) is a command key,
            // which the typing rule decides on, as a text field elsewhere on the platform ignores it.
            keymap
                .bindings(command)
                .into_iter()
                .filter(|binding| {
                    binding.trigger.press == PressKind::Single
                        && craft_keymap::text_editing_chords(platform).contains(&binding.trigger.chord.effective(platform))
                })
                .map(move |binding| (edit, binding.trigger.chord))
        })
        .collect();
    let mut shown_text = HashMap::new();
    for def in keymap.catalog() {
        if let Some(chord) = keymap.shown_chord(&def.command) {
            shown_text.entry(chord).or_insert_with(|| {
                #[cfg(feature = "testing")]
                CHORD_TEXT_BUILDS.with(|builds| builds.set(builds.get() + 1));
                Arc::from(chord.display(platform))
            });
        }
    }
    BuiltKeymap { keymap: Arc::new(keymap), edit_chords, window_shortcuts, shown_text: Arc::new(shown_text) }
}

/// The keymap of the command sets linked into the binary on `platform`, the stories' sets too when `stories`: the
/// keymap the app host builds its `CommandBoard` from when the app gives none.
pub fn linked_keymap(stories: bool, platform: Platform) -> Result<Keymap, registry::CommandDefError> {
    let registry = registry::Registry::shared(stories)?;
    Ok((*keymap_for(&registry, None, platform, None, false).keymap).clone())
}

/// Builds or reuses a keymap owned by this exact command registry: one with no catalog, no overrides and holds off is
/// built once per platform.
pub(crate) fn keymap_for(
    registry: &Arc<registry::Registry>,
    app: Option<&AppCatalog>,
    platform: Platform,
    overrides: Option<&Arc<Overrides>>,
    holds: bool,
) -> BuiltKeymap {
    if app.is_some() || overrides.is_some() || holds {
        return build_keymap(registry, app, platform, overrides, holds);
    }
    let mut kept = registry.keymaps.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((_, built)) = kept.iter().find(|(on, _)| *on == platform) {
        return built.clone();
    }
    let built = build_keymap(registry, None, platform, None, false);
    kept.push((platform, built.clone()));
    built
}
