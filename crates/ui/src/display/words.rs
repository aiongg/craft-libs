//! Built-in words of the display family.

craft_ui::words! {
    /// The words of the display components.
    pub struct DisplayWords {
        kbd_up: "kbd.up" = "Up";
        kbd_down: "kbd.down" = "Down";
        kbd_left: "kbd.left" = "Left";
        kbd_right: "kbd.right" = "Right";
        kbd_page_up: "kbd.pageUp" = "Page Up";
        kbd_page_down: "kbd.pageDown" = "Page Down";
    }
}

impl DisplayWords {
    /// The name of a key that is a word off macOS (arrows and page keys); `None` for every other key.
    pub(super) fn key(&self, key: craft_keymap::Key) -> Option<&str> {
        use craft_keymap::Key;
        let word = match key {
            Key::Up => &self.kbd_up,
            Key::Down => &self.kbd_down,
            Key::Left => &self.kbd_left,
            Key::Right => &self.kbd_right,
            Key::PageUp => &self.kbd_page_up,
            Key::PageDown => &self.kbd_page_down,
            _ => return None,
        };
        Some(word.as_str())
    }
}
