//! [`Listbox`]: the active option of a WAI-ARIA listbox whose focus stays on its owner, moved by
//! the keys, type-ahead and the pointer.

use egui::{Event, Id, Key, Modifiers, Ui};

/// How long a type-ahead search waits for the next character before it starts again, in seconds:
/// macOS and Windows list boxes keep a prefix for about a second.
pub const TYPE_AHEAD_PAUSE: f64 = 1.0;

/// The options a [`Listbox`] moves over, as its control knows them: each option is the control's
/// own index, stable while a search filters the list.
pub trait ListOptions {
    /// The options shown, top to bottom; group headers and separators are not options.
    fn shown(&self) -> &[usize];
    /// Whether `option` can be chosen; the keys and type-ahead skip one that can't.
    fn enabled(&self, option: usize) -> bool;
    /// The text type-ahead matches against.
    fn text(&self, option: usize) -> String;
}

/// The behaviour of a WAI-ARIA listbox whose keyboard focus stays on its owner (a select's box, a
/// combo box's field, a search field) while the list is open: one *active* option, apart from
/// the chosen one, which the owner names to AccessKit as its active descendant.
///
/// - Down and Up move to the next and previous enabled option, without wrapping; with none
///   active, Down goes to the first and Up to the last.
/// - Page Down and Page Up move a page of options and stop at the ends; Home and End go to the
///   first and last enabled option, where the owner lets them ([`keys`](Listbox::keys)'s
///   `home_end`: a text field keeps them for its caret).
/// - Type-ahead: characters typed within [`TYPE_AHEAD_PAUSE`] of each other make a prefix, and
///   the first enabled option from the active one whose text starts with it (ignoring case)
///   becomes active. The same character typed again moves on to the next option starting with
///   it.
/// - The pointer makes the option under it active ([`set_active`](Listbox::set_active)).
/// - A key or type-ahead move asks for the active option to be scrolled into view
///   ([`take_reveal`](Listbox::take_reveal)); a pointer move doesn't.
///
/// The control loads the listbox each frame, feeds it input while the list is open, draws the
/// options with the active one highlighted, and [`store`](Listbox::store)s it.
#[derive(Clone, Debug)]
#[must_use = "store the listbox with `store`"]
pub struct Listbox {
    id: Id,
    state: State,
}

#[derive(Clone, Debug, Default)]
struct State {
    active: Option<usize>,
    reveal: bool,
    typed: String,
    typed_at: f64,
}

impl Listbox {
    /// The listbox with id `id`, as its control left it.
    pub fn load(ui: &Ui, id: Id) -> Self {
        Self { id, state: ui.data(|d| d.get_temp::<State>(Self::key(id))).unwrap_or_default() }
    }

    /// Keeps the listbox for the next frame.
    pub fn store(self, ui: &Ui) {
        ui.data_mut(|d| d.insert_temp(Self::key(self.id), self.state));
    }

    fn key(id: Id) -> Id {
        id.with("craft_ui::behavior::listbox")
    }

    /// The active option.
    pub fn active(&self) -> Option<usize> {
        self.state.active
    }

    /// Makes `option` active without scrolling: the pointer moved onto it.
    pub fn set_active(&mut self, option: Option<usize>) {
        self.state.active = option;
    }

    /// Makes `option` active and asks for it to be scrolled into view: the list opened on it, or
    /// a key moved to it.
    pub fn reveal(&mut self, option: Option<usize>) {
        self.state.active = option;
        self.state.reveal = option.is_some();
    }

    /// The active option, once, when it is to be scrolled into view this frame.
    pub fn take_reveal(&mut self) -> Option<usize> {
        std::mem::take(&mut self.state.reveal).then_some(self.state.active).flatten()
    }

    /// Clears the active option and the type-ahead prefix: the list closed.
    pub fn reset(&mut self) {
        self.state = State::default();
    }

    /// Applies the arrow keys, Page Up and Page Down, and with `home_end` Home and End, taking
    /// them from the input. `page` is how many options the list shows at once. Returns whether
    /// the active option moved.
    pub fn keys(&mut self, ui: &Ui, options: &impl ListOptions, page: usize, home_end: bool) -> bool {
        let shown = options.shown();
        let enabled = |p: &usize| options.enabled(shown[*p]);
        let at = self.state.active.and_then(|a| shown.iter().position(|&o| o == a));
        let last = shown.len().saturating_sub(1);
        let pressed = |key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        let target = if pressed(Key::ArrowDown) {
            match at {
                Some(p) => (p + 1..shown.len()).find(enabled),
                None => (0..shown.len()).find(enabled),
            }
        } else if pressed(Key::ArrowUp) {
            match at {
                Some(p) => (0..p).rev().find(enabled),
                None => (0..shown.len()).rev().find(enabled),
            }
        } else if pressed(Key::PageDown) {
            let from = at.map_or(0, |p| (p + page.max(1)).min(last));
            (from..shown.len()).find(enabled).or_else(|| (at.map_or(0, |p| p + 1)..from).rev().find(enabled))
        } else if pressed(Key::PageUp) {
            let from = at.map_or(last, |p| p.saturating_sub(page.max(1)));
            (0..=from).rev().find(enabled).or_else(|| (from..at.unwrap_or(shown.len())).find(enabled))
        } else if home_end && pressed(Key::Home) {
            (0..shown.len()).find(enabled)
        } else if home_end && pressed(Key::End) {
            (0..shown.len()).rev().find(enabled)
        } else {
            None
        };
        self.move_to(target.map(|p| shown[p]))
    }

    /// Applies the characters typed this frame as type-ahead, taking them from the input. A
    /// space counts only inside a prefix under way, so Space alone stays a key for the control.
    /// Returns whether the active option moved.
    pub fn type_ahead(&mut self, ui: &Ui, options: &impl ListOptions) -> bool {
        let now = ui.input(|i| i.time);
        if now - self.state.typed_at > TYPE_AHEAD_PAUSE {
            self.state.typed.clear();
        }
        let mut typed = std::mem::take(&mut self.state.typed);
        let mut took = false;
        ui.input_mut(|i| {
            i.events.retain(|event| match event {
                Event::Text(text) if !(typed.is_empty() && text.trim().is_empty()) && !text.chars().any(char::is_control) => {
                    typed.push_str(&text.to_lowercase());
                    took = true;
                    false
                }
                _ => true,
            });
        });
        self.state.typed = typed;
        if !took {
            return false;
        }
        self.state.typed_at = now;
        // "aaa" cycles through the options starting with "a".
        let mut chars = self.state.typed.chars();
        let first = chars.next().unwrap_or_default();
        let (prefix, skip_active) = if chars.all(|c| c == first) { (first.to_string(), true) } else { (self.state.typed.clone(), false) };
        let shown = options.shown();
        let start = self.state.active.and_then(|a| shown.iter().position(|&o| o == a)).map_or(0, |p| p + usize::from(skip_active));
        let target = (0..shown.len())
            .map(|step| shown[(start + step) % shown.len()])
            .find(|&o| options.enabled(o) && options.text(o).to_lowercase().starts_with(&prefix));
        self.move_to(target)
    }

    /// Whether a type-ahead prefix is under way, so a space belongs to it.
    pub fn typing(&self, ui: &Ui) -> bool {
        !self.state.typed.is_empty() && ui.input(|i| i.time) - self.state.typed_at <= TYPE_AHEAD_PAUSE
    }

    fn move_to(&mut self, target: Option<usize>) -> bool {
        match target {
            Some(option) if self.state.active != Some(option) => {
                self.reveal(Some(option));
                true
            }
            Some(_) => {
                self.state.reveal = true;
                false
            }
            None => false,
        }
    }
}
