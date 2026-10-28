//! A [`Chord`] as a [`Kbd`], for menus and tooltips that show shortcuts.

use craft_ui::controls::Kbd;
use egui::os::OperatingSystem;

use crate::chord::{Chord, Platform};

/// The chord as `craft_ui`'s shortcut text for `platform`: ⌃⌥⇧⌘ and the key on a Mac, as
/// [`Chord::display`] writes it, `Ctrl+Alt+Shift+` and the key elsewhere. Off a Mac `ctrl` folds
/// into `cmd`, as for [`Chord::effective`].
pub fn kbd(chord: &Chord, platform: Platform) -> Kbd {
    let os = match platform {
        Platform::Mac => OperatingSystem::Mac,
        Platform::Windows => OperatingSystem::Windows,
        _ => OperatingSystem::Nix,
    };
    Kbd::new(&chord.to_egui_shortcut()).os(os)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chord_is_written_like_chord_display_in_each_platforms_order() {
        for text in ["Cmd+N", "Cmd+Shift+S", "Cmd+Ctrl+Alt+Shift+K", "Ctrl+Alt+Z", "Shift+F5", "G"] {
            let chord = Chord::parse(text).expect("a chord");
            for platform in [Platform::Mac, Platform::Windows, Platform::Linux] {
                let os = match platform {
                    Platform::Mac => OperatingSystem::Mac,
                    Platform::Windows => OperatingSystem::Windows,
                    _ => OperatingSystem::Nix,
                };
                assert_eq!(kbd(&chord, platform).text(os), chord.display(platform), "{text} on {platform:?}");
            }
        }
    }
}
