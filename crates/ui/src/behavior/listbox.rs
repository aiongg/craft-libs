//! [`Listbox`]: the active option of a WAI-ARIA listbox whose focus stays on its owner, moved by
//! the keys, type-ahead and the pointer.

use std::borrow::Cow;

use egui::{Event, Id, Key, Modifiers, Ui};

/// How long a type-ahead search waits for the next character before it starts again, in seconds:
/// macOS and Windows list boxes keep a prefix for about a second.
pub const TYPE_AHEAD_PAUSE: f64 = 1.0;

/// The options a [`Listbox`] moves over, as its control shows them. Each option is the control's
/// own index, stable while a search filters the list; a position counts the options shown, from
/// 0 at the top.
pub trait ListOptions {
    /// How many options are shown; group headers and separators are not options.
    fn len(&self) -> usize;

    /// Whether no option is shown.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The option shown at position `at`, below [`len`](ListOptions::len).
    fn option_at(&self, at: usize) -> usize;

    /// The position of `option` among those shown, if it is shown. The default looks through
    /// them all; a control that knows the answer sooner says so.
    fn position(&self, option: usize) -> Option<usize> {
        (0..self.len()).find(|&at| self.option_at(at) == option)
    }

    /// Whether `option` can be chosen; the keys and type-ahead skip one that can't.
    fn enabled(&self, option: usize) -> bool;

    /// The text type-ahead matches against.
    fn text(&self, option: usize) -> Cow<'_, str>;
}

/// The behaviour of a WAI-ARIA listbox whose keyboard focus stays on its owner (a select's box, a
/// combo box's field, a search field) while the list is open: one *active* option, apart from
/// the chosen one, which the owner names to AccessKit as its active descendant.
///
/// - Down and Up move to the next and previous enabled option, without wrapping; with none
///   active, Down goes to the first and Up to the last.
/// - Page Down and Page Up move a page of options and stop at the ends. Home and End, Ctrl+Home
///   and Ctrl+End, and on macOS Cmd+Up and Cmd+Down go to the first and last enabled option,
///   where the owner lets them ([`keys`](Listbox::keys)'s `home_end`: a text field keeps them
///   for its caret).
/// - The keys match their modifiers exactly: Shift with an arrow stays free for a list that
///   selects a range.
/// - Type-ahead: characters typed within [`TYPE_AHEAD_PAUSE`] of each other make a prefix, and
///   the first enabled option from the active one whose text starts with it (ignoring case)
///   becomes active. The same character typed again moves on to the next option starting with
///   it. A character that matches nothing after the ones before it starts a new prefix with
///   itself ("m" then "d" goes to the first option starting with "d"). A key that moves the
///   active option starts the prefix again.
/// - The pointer makes the option under it active where the control lets it
///   ([`set_active`](Listbox::set_active)): a select does; a combo box's pointer only highlights
///   a row, so Enter keeps the typed text.
/// - A key or type-ahead move asks for the active option to be scrolled into view
///   ([`take_reveal`](Listbox::take_reveal)); a pointer move doesn't.
/// - With the keyboard focus on the owner, the active option is the keyboard's focus cue while
///   the keyboard put it there ([`keyboard`](Listbox::keyboard)).
/// - With no option shown, the keys and characters are taken and change nothing.
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
    keyboard: bool,
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

    /// Whether the keyboard made the active option active (a key, type-ahead, or a list opened
    /// from the keyboard), so the owner draws the keyboard focus cue on it. The pointer clears
    /// it.
    pub fn keyboard(&self) -> bool {
        self.state.keyboard && self.state.active.is_some()
    }

    /// Makes `option` active without scrolling: the pointer moved onto it.
    pub fn set_active(&mut self, option: Option<usize>) {
        self.state.active = option;
        self.state.keyboard = false;
    }

    /// Makes `option` active and asks for it to be scrolled into view: the list opened on it, a
    /// search left it first, or a key moved to it. `keyboard` says whether the keyboard did it
    /// ([`keyboard`](Listbox::keyboard)).
    pub fn reveal(&mut self, option: Option<usize>, keyboard: bool) {
        self.state.active = option;
        self.state.keyboard = keyboard;
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

    /// Applies the arrow keys, Page Up and Page Down, and with `home_end` the keys that go to the
    /// first and last option, taking them from the input. `page` is how many options the list
    /// shows at once. Returns whether the active option moved.
    pub fn keys(&mut self, ui: &Ui, options: &impl ListOptions, page: usize, home_end: bool) -> bool {
        let pressed = |modifiers, key| take_key(ui, modifiers, key);
        let to_end =
            |key, cmd_arrow| home_end && (pressed(Modifiers::NONE, key) || pressed(Modifiers::CTRL, key) || pressed(Modifiers::MAC_CMD, cmd_arrow));
        let step = if pressed(Modifiers::NONE, Key::ArrowDown) {
            Step::Next
        } else if pressed(Modifiers::NONE, Key::ArrowUp) {
            Step::Previous
        } else if pressed(Modifiers::NONE, Key::PageDown) {
            Step::PageDown
        } else if pressed(Modifiers::NONE, Key::PageUp) {
            Step::PageUp
        } else if to_end(Key::Home, Key::ArrowUp) {
            Step::First
        } else if to_end(Key::End, Key::ArrowDown) {
            Step::Last
        } else {
            return false;
        };
        self.state.typed.clear();
        let len = options.len();
        if len == 0 {
            return false;
        }
        let enabled = |at: &usize| options.enabled(options.option_at(*at));
        let at = self.state.active.and_then(|a| options.position(a));
        let last = len - 1;
        let page = page.max(1);
        let target = match step {
            Step::Next => return self.move_to(Self::neighbour(options, self.state.active, true)),
            Step::Previous => return self.move_to(Self::neighbour(options, self.state.active, false)),
            Step::PageDown => {
                let from = at.map_or(0, |p| (p + page).min(last));
                (from..len).find(enabled).or_else(|| (at.map_or(0, |p| p + 1)..from).rev().find(enabled))
            }
            Step::PageUp => {
                let from = at.map_or(last, |p| p.saturating_sub(page));
                (0..=from).rev().find(enabled).or_else(|| (from..at.unwrap_or(len)).find(enabled))
            }
            Step::First => (0..len).find(enabled),
            Step::Last => (0..len).rev().find(enabled),
        };
        self.move_to(target.map(|p| options.option_at(p)))
    }

    /// The enabled option after `from` among those shown (`next`) or before it, as Down and Up
    /// move: none past the ends; with `from` none or not shown, the first or the last. A closed
    /// list's Up and Down and its wheel step the value with it.
    pub fn neighbour(options: &impl ListOptions, from: Option<usize>, next: bool) -> Option<usize> {
        let len = options.len();
        let enabled = |at: &usize| options.enabled(options.option_at(*at));
        let at = from.and_then(|o| options.position(o));
        let found = match (at, next) {
            (Some(p), true) => (p + 1..len).find(enabled),
            (Some(p), false) => (0..p).rev().find(enabled),
            (None, true) => (0..len).find(enabled),
            (None, false) => (0..len).rev().find(enabled),
        };
        found.map(|at| options.option_at(at))
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
        let mut chars = Vec::new();
        ui.input_mut(|i| {
            i.events.retain(|event| match event {
                Event::Text(text) if !(typed.is_empty() && text.trim().is_empty()) && !text.chars().any(char::is_control) => {
                    chars.extend(text.chars().flat_map(char::to_lowercase));
                    false
                }
                _ => true,
            });
        });
        let len = options.len();
        if chars.is_empty() || len == 0 {
            self.state.typed = typed;
            return false;
        }
        self.state.typed_at = now;
        let mut active = self.state.active;
        let mut target = None;
        for c in chars {
            // A space only counts inside a prefix.
            if c.is_whitespace() && typed.is_empty() {
                continue;
            }
            typed.push(c);
            let mut found = Self::find(options, active, &typed);
            // A letter that leads nowhere after the others starts a new prefix ("m" then "d" is D).
            if found.is_none() && typed.chars().count() > 1 {
                typed.clear();
                if !c.is_whitespace() {
                    typed.push(c);
                    found = Self::find(options, active, &typed);
                }
            }
            if found.is_some() {
                target = found;
                active = found;
            }
        }
        self.state.typed = typed;
        self.move_to(target)
    }

    /// The enabled option whose text starts with `typed` (lowercase), searching on from `active`
    /// and wrapping. "aaa" cycles through the options starting with "a".
    fn find(options: &impl ListOptions, active: Option<usize>, typed: &str) -> Option<usize> {
        let len = options.len();
        let mut chars = typed.chars();
        let first = chars.next().unwrap_or_default();
        let (prefix, skip_active) = if chars.all(|c| c == first) { (first.to_string(), true) } else { (typed.to_string(), false) };
        let start = active.and_then(|a| options.position(a)).map_or(0, |p| p + usize::from(skip_active));
        (0..len)
            .map(|step| options.option_at((start + step) % len))
            .find(|&o| options.enabled(o) && options.text(o).to_lowercase().starts_with(&prefix))
    }

    /// Whether a type-ahead prefix is under way, so a space belongs to it.
    pub fn typing(&self, ui: &Ui) -> bool {
        !self.state.typed.is_empty() && ui.input(|i| i.time) - self.state.typed_at <= TYPE_AHEAD_PAUSE
    }

    fn move_to(&mut self, target: Option<usize>) -> bool {
        let moved = target.is_some_and(|option| self.state.active != Some(option));
        if target.is_some() {
            self.reveal(target, true);
        }
        moved
    }
}

/// Where a key moves the active option.
#[derive(Clone, Copy)]
enum Step {
    Next,
    Previous,
    PageDown,
    PageUp,
    First,
    Last,
}

/// Takes the presses of `key` with exactly `modifiers` held (Ctrl and Cmd as egui's
/// [`Modifiers::matches_exact`] counts them) from the input; returns whether there were any.
pub(crate) fn take_key(ui: &Ui, modifiers: Modifiers, key: Key) -> bool {
    ui.input_mut(|i| {
        let before = i.events.len();
        i.events.retain(|event| !matches!(event, Event::Key { key: k, pressed: true, modifiers: m, .. } if *k == key && m.matches_exact(modifiers)));
        i.events.len() < before
    })
}
