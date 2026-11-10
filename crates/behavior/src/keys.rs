//! The key table (spec 9.2): the one place that maps a key press to an intent and modifiers to `MoveMods`.
//! Pure functions over craft-keymap's `Key` and `Mods`, where `cmd` is Ctrl on Windows and Linux and Cmd on
//! macOS and `ctrl` is the Control key on macOS only.

use crate::intent::{DiscloseTo, Granularity, Intent, MoveMods, NavMove};
use craft_host::target::Os;
use craft_keymap::{Chord, Key, Mods};
use emath::Vec2;

/// What the focused stop is, as far as the table's rows differ.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct KeyRole {
    /// A value control with a reset value and no text part: Delete resets it.
    pub resettable: bool,
    /// A collection: Space is its own intent (`Intent::Space`), which the item step turns into its `SpaceAction`.
    pub space: bool,
    /// A reorderable collection: Alt+Up and Alt+Down move the lead item.
    pub reorderable: bool,
    /// A collection with `CollectionProps.rename`.
    pub rename: bool,
    /// `FocusPolicy.pages`: Ctrl+Tab switches pages.
    pub pages: bool,
    /// The component reads arrow nudges: arrows are not moves. One that deletes (a canvas control) takes only the
    /// plain Delete and Backspace; with a modifier held they are the app's.
    pub nudge: bool,
    /// Typed letters reach the component as `Intent::Text` (type-ahead).
    pub type_ahead: bool,
    /// Typed digits reach the component as `Intent::Digit`; the other typed text stays `Intent::Text`.
    pub digits: bool,
    /// A value control: Alt with an arrow is a fine move.
    pub value: bool,
    /// The stop is a text part: Alt+Left and Alt+Right keep their word-move rows.
    pub text: bool,
    /// The component acts on the arrow moves routed to it: they are not shortcut candidates.
    pub moves: bool,
    /// The component acts on Delete and Backspace (text edits, deleting a selection): they reach it as
    /// `Intent::Delete`. Any other role leaves them to the command scope chain.
    pub deletes: bool,
    /// A collection that leaves its arrow keys to the component around it (`ItemKeys::Owner`): they are no moves
    /// of this role.
    pub owner_arrows: bool,
    /// The stop handles `Disclose` of every descendant (an outline collection): on macOS Option+Right and Option+Left
    /// open and close all, which a text or value role keeps for its own moves.
    pub disclose_all: bool,
    /// The stop handles `OpenDetails` (a leaf that declares a double-click action): Alt+Enter on Windows and Linux,
    /// Cmd+Down on macOS. A collection leaves this off: its Enter is `Activate`, which is its double-click meaning.
    pub details: bool,
    /// The item that Ctrl or Cmd with Home goes to: the first one that is not frozen. Zero where nothing is frozen.
    pub home_row: usize,
}

impl KeyRole {
    /// A role with no special rows.
    pub const fn new() -> KeyRole {
        KeyRole {
            resettable: false,
            space: false,
            reorderable: false,
            rename: false,
            pages: false,
            nudge: false,
            type_ahead: false,
            digits: false,
            value: false,
            text: false,
            moves: false,
            deletes: false,
            owner_arrows: false,
            disclose_all: false,
            details: false,
            home_row: 0,
        }
    }

    /// Delete resets the control.
    pub const fn resettable(mut self, on: bool) -> Self {
        self.resettable = on;
        self
    }

    /// Space is the component's own intent.
    pub const fn space(mut self, on: bool) -> Self {
        self.space = on;
        self
    }

    /// Alt+Up and Alt+Down move the lead item.
    pub const fn reorderable(mut self, on: bool) -> Self {
        self.reorderable = on;
        self
    }

    /// Rename has its key.
    pub const fn rename(mut self, on: bool) -> Self {
        self.rename = on;
        self
    }

    /// Ctrl+Tab switches pages.
    pub const fn pages(mut self, on: bool) -> Self {
        self.pages = on;
        self
    }

    /// Arrows are nudges, not moves.
    pub const fn nudge(mut self, on: bool) -> Self {
        self.nudge = on;
        self
    }

    /// Typed letters are delivered as text.
    pub const fn type_ahead(mut self, on: bool) -> Self {
        self.type_ahead = on;
        self
    }

    /// Typed digits are delivered as digits.
    pub const fn digits(mut self, on: bool) -> Self {
        self.digits = on;
        self
    }

    /// Alt with an arrow is a fine move.
    pub const fn value(mut self, on: bool) -> Self {
        self.value = on;
        self
    }

    /// The component acts on the moves routed to it.
    pub const fn moves(mut self, on: bool) -> Self {
        self.moves = on;
        self
    }

    /// Delete and Backspace are the component's own.
    pub const fn deletes(mut self, on: bool) -> Self {
        self.deletes = on;
        self
    }

    /// The arrow keys are the enclosing component's, not this role's moves.
    pub const fn owner_arrows(mut self, on: bool) -> Self {
        self.owner_arrows = on;
        self
    }

    /// The stop opens and closes every descendant.
    pub const fn disclose_all(mut self, on: bool) -> Self {
        self.disclose_all = on;
        self
    }

    /// The stop has a details action.
    pub const fn details(mut self, on: bool) -> Self {
        self.details = on;
        self
    }

    /// Ctrl or Cmd with Home goes to item `row`.
    pub const fn home_row(mut self, row: usize) -> Self {
        self.home_row = row;
        self
    }

    /// The stop is a text part.
    pub const fn text(mut self, on: bool) -> Self {
        self.text = on;
        self
    }
}

/// What a drop of a dragged payload does with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DropOperation {
    /// Move the payload.
    Move,
    /// Copy the payload (Alt, Option on macOS).
    Copy,
    /// Link to the payload (Ctrl+Shift, Cmd+Option on macOS).
    Link,
}

/// The drop operation the held modifiers pick.
pub fn drop_operation(os: Os, mods: Mods) -> DropOperation {
    let link = if os == Os::Mac { mods.cmd && mods.alt } else { mods.cmd && mods.shift };
    if link {
        DropOperation::Link
    } else if mods.alt {
        DropOperation::Copy
    } else {
        DropOperation::Move
    }
}

/// Typed `text` split for a role that takes digits: the ASCII digits it holds, in order, and the text without them.
pub fn split_digits(text: &str) -> (Vec<u8>, String) {
    let digits = text.chars().filter_map(|letter| letter.to_digit(10)).map(|digit| digit as u8).collect();
    (digits, text.chars().filter(|letter| !letter.is_ascii_digit()).collect())
}

fn is_arrow(key: Key) -> bool {
    matches!(key, Key::Up | Key::Down | Key::Left | Key::Right)
}

/// The intent a key press has for a stop of `role`, if any. `Enter` and `Space` as activation are not rows:
/// they follow the focused component's `KeyUse`.
pub fn intent(os: Os, key: Key, mods: Mods, role: KeyRole) -> Option<Intent> {
    let mac = os == Os::Mac;
    let plain = mods.is_none();
    let only = |m: Mods| mods == m;
    let shifted = |m: Mods| mods == Mods { shift: true, ..m };
    // Arrows are nudges for the content, with every modifier: Alt+arrow is no Open or Close.
    if role.nudge && is_arrow(key) {
        return None;
    }
    if !role.reorderable
        && let Some(fine) = fine_move(key, mods, role)
    {
        return Some(fine);
    }
    match key {
        Key::Delete | Key::Backspace if plain && role.resettable => return Some(Intent::Reset),
        Key::Space if plain && role.space => return Some(Intent::Space),
        Key::Menu if plain && !mac => return Some(Intent::ContextMenu),
        Key::F10 if only(Mods::SHIFT) => return Some(Intent::ContextMenu),
        Key::F2 if plain && role.rename && !mac => return Some(Intent::Rename),
        Key::Enter if plain && role.rename && mac => return Some(Intent::Rename),
        Key::Down if mac && role.rename && only(Mods::CMD) => return Some(Intent::Activate),
        Key::Enter if only(Mods::CMD) => return Some(Intent::SubmitForce),
        Key::Up if only(Mods::ALT) && role.reorderable => return Some(Intent::move_item(false)),
        Key::Down if only(Mods::ALT) && role.reorderable => return Some(Intent::move_item(true)),
        Key::Right if mac && role.disclose_all && only(Mods::ALT) => return Some(Intent::disclose(DiscloseTo::Open, true)),
        Key::Left if mac && role.disclose_all && only(Mods::ALT) => return Some(Intent::disclose(DiscloseTo::Close, true)),
        Key::Enter if !mac && role.details && only(Mods::ALT) => return Some(Intent::OpenDetails),
        Key::Down if mac && role.details && only(Mods::CMD) => return Some(Intent::OpenDetails),
        Key::Down if only(Mods::ALT) => return Some(Intent::disclose(DiscloseTo::Open, false)),
        Key::Up if only(Mods::ALT) => return Some(Intent::disclose(DiscloseTo::Close, false)),
        Key::PageUp if !mac && only(Mods::CMD) => return Some(Intent::switch_page(false)),
        Key::PageDown if !mac && only(Mods::CMD) => return Some(Intent::switch_page(true)),
        Key::OpenBracket | Key::OpenCurlyBracket if mac && shifted(Mods::CMD) => return Some(Intent::switch_page(false)),
        Key::CloseBracket | Key::CloseCurlyBracket if mac && shifted(Mods::CMD) => return Some(Intent::switch_page(true)),
        Key::Tab if role.pages => {
            let ctrl = if mac { Mods::CTRL } else { Mods::CMD };
            if only(ctrl) {
                return Some(Intent::switch_page(true));
            }
            if shifted(ctrl) {
                return Some(Intent::switch_page(false));
            }
        }
        _ => {}
    }
    if role.owner_arrows && is_arrow(key) {
        return None;
    }
    movement(mac, key, mods, role)
}

const MODS: [Mods; 16] = {
    let mut all = [Mods::NONE; 16];
    let mut i = 0;
    while i < 16 {
        all[i] = Mods { cmd: i & 1 != 0, ctrl: i & 2 != 0, alt: i & 4 != 0, shift: i & 8 != 0 };
        i += 1;
    }
    all
};

/// The chords that give a stop of `role` on `os` the action `wanted`, from the same rows as [`intent`]. A row that
/// is not a table row has its chords listed here: `Activate` on Enter and Space (they follow the focused
/// component's `KeyUse`) and `Nudge` on the arrow along its axis, for a nudge role. `Custom`, `Edit` and `Access`
/// have no chord: commands and AccessKit carry them.
pub fn chords(os: Os, role: KeyRole, wanted: &Intent) -> Vec<Chord> {
    match wanted {
        Intent::Activate => {
            let space = (!role.space).then_some(Key::Space);
            [Some(Key::Enter), space].into_iter().flatten().map(|key| Chord { mods: Mods::NONE, key }).collect()
        }
        Intent::Nudge(by, mods) if role.nudge => nudge_key(*by).map(|key| Chord { mods: *mods, key }).into_iter().collect(),
        Intent::Nudge(..) | Intent::Custom(_) | Intent::Edit(_) | Intent::Access(_) => Vec::new(),
        _ => Key::ALL
            .iter()
            .flat_map(|&key| MODS.iter().map(move |&mods| Chord { mods, key }))
            .filter(|chord| intent(os, chord.key, chord.mods, role).as_ref() == Some(wanted))
            .collect(),
    }
}

/// Whether some chord gives a stop of `role` on `os` the action `wanted`: the step-7 check of `KeyPath::Table`.
pub fn reaches(os: Os, role: KeyRole, wanted: &Intent) -> bool {
    !chords(os, role, wanted).is_empty()
}

fn nudge_key(by: Vec2) -> Option<Key> {
    match (by.x, by.y) {
        (x, 0.0) if x < 0.0 => Some(Key::Left),
        (x, 0.0) if x > 0.0 => Some(Key::Right),
        (0.0, y) if y < 0.0 => Some(Key::Up),
        (0.0, y) if y > 0.0 => Some(Key::Down),
        _ => None,
    }
}

/// Alt with an arrow on a value role: a fine move, the same on every OS. A text role keeps Left and Right for word
/// moves, so a text value takes the fine step on Up and Down only. Shift extends, which a value reads as the small step.
fn fine_move(key: Key, mods: Mods, role: KeyRole) -> Option<Intent> {
    if !role.value || role.nudge || !mods.alt || mods.cmd || mods.ctrl {
        return None;
    }
    let mv = match key {
        Key::Up => NavMove::Up,
        Key::Down => NavMove::Down,
        Key::Left if !role.text => NavMove::Back,
        Key::Right if !role.text => NavMove::Forward,
        _ => return None,
    };
    Some(Intent::Move(mv, MoveMods::new(mods.shift, false, Granularity::Unit).fine(true)))
}

/// The move rows: arrows, Home, End and the page keys, and the deletes.
fn movement(mac: bool, key: Key, mods: Mods, role: KeyRole) -> Option<Intent> {
    let by_word = if mac { mods.alt } else { mods.cmd };
    let lead = mods.cmd;
    let moved = |mv: NavMove, by: Granularity| Some(Intent::Move(mv, MoveMods::new(mods.shift, lead, by)));
    let word_or_unit = if by_word { Granularity::Word } else { Granularity::Unit };
    // The Control key on macOS belongs to the OS (Ctrl+arrow switches spaces).
    let mac_control = mac && mods.ctrl;
    let plain_alt_free = !mods.alt || (mac && by_word);
    match key {
        Key::Left | Key::Right if mac && mods.cmd && !mods.alt => {
            moved(if key == Key::Left { NavMove::Start } else { NavMove::End }, Granularity::Line)
        }
        Key::Left if plain_alt_free && !mac_control => moved(NavMove::Back, word_or_unit),
        Key::Right if plain_alt_free && !mac_control => moved(NavMove::Forward, word_or_unit),
        Key::Up | Key::Down if mac && mods.cmd && !mods.alt => {
            moved(if key == Key::Up { NavMove::Start } else { NavMove::End }, Granularity::Document)
        }
        Key::Up if !mods.alt => moved(NavMove::Up, Granularity::Unit),
        Key::Down if !mods.alt => moved(NavMove::Down, Granularity::Unit),
        Key::Home | Key::End if !mac || !mods.cmd => {
            let by = if mods.cmd { Granularity::Document } else { Granularity::Line };
            moved(if key == Key::Home { NavMove::Start } else { NavMove::End }, by)
        }
        Key::PageUp if !mods.alt && !mods.cmd && !mods.ctrl => moved(NavMove::PageUp, Granularity::Unit),
        Key::PageDown if !mods.alt && !mods.cmd && !mods.ctrl => moved(NavMove::PageDown, Granularity::Unit),
        // A canvas control takes the plain keys; with a modifier they are the app's (Photoshop fills on Alt+Backspace
        // and Cmd+Backspace and opens Fill on Shift+Backspace). A canvas tool takes no row: it claims the deletes it
        // takes in its state (`CanvasTool::claims` in craft-ui).
        Key::Delete | Key::Backspace if role.deletes && role.nudge && !role.text => {
            mods.is_none().then(|| Intent::delete(key == Key::Delete, Granularity::Unit))
        }
        Key::Delete | Key::Backspace if role.deletes && (!mods.alt || mac) => {
            let by = if by_word { Granularity::Word } else { Granularity::Unit };
            Some(Intent::Delete { forward: key == Key::Delete, by, shift: mods.shift })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::ActionId;

    const NONE: Mods = Mods::NONE;

    fn check(os: Os, key: Key, mods: Mods, role: KeyRole, want: Option<Intent>) {
        assert_eq!(intent(os, key, mods, role), want, "{os:?} {key:?} {mods:?} {role:?}");
    }

    #[test]
    fn table_rows_follow_the_role_and_the_os() {
        let list = KeyRole::new().reorderable(true).space(true);
        let any = KeyRole::new();
        let rename = KeyRole::new().rename(true);
        let pages = KeyRole::new().pages(true);
        let nudge = KeyRole::new().nudge(true);
        let reset = KeyRole::new().resettable(true);
        let deleter = KeyRole::new().deletes(true);
        let alt_mods = Mods::ALT;
        let word = |extend, lead| MoveMods::new(extend, lead, Granularity::Word);
        for os in [Os::Linux, Os::Windows] {
            check(os, Key::Down, alt_mods, list, Some(Intent::move_item(true)));
            check(os, Key::Up, alt_mods, list, Some(Intent::move_item(false)));
            check(os, Key::Down, alt_mods, any, Some(Intent::disclose(DiscloseTo::Open, false)));
            check(os, Key::Up, alt_mods, any, Some(Intent::disclose(DiscloseTo::Close, false)));
            check(os, Key::Space, NONE, list, Some(Intent::Space));
            check(os, Key::Space, NONE, any, None);
            check(os, Key::PageDown, Mods::CMD, any, Some(Intent::switch_page(true)));
            check(os, Key::Enter, Mods::CMD, any, Some(Intent::SubmitForce));
            check(os, Key::F2, NONE, rename, Some(Intent::Rename));
            check(os, Key::F2, NONE, any, None);
            check(os, Key::Menu, NONE, any, Some(Intent::ContextMenu));
            check(os, Key::Tab, Mods::CMD, pages, Some(Intent::switch_page(true)));
            check(os, Key::Tab, Mods { shift: true, ..Mods::CMD }, pages, Some(Intent::switch_page(false)));
            check(os, Key::Tab, Mods::CMD, any, None);
            check(os, Key::Left, Mods::CMD, any, Some(Intent::Move(NavMove::Back, word(false, true))));
            check(os, Key::End, NONE, any, Some(Intent::Move(NavMove::End, MoveMods::new(false, false, Granularity::Line))));
            check(os, Key::Home, Mods::CMD, any, Some(Intent::Move(NavMove::Start, MoveMods::new(false, true, Granularity::Document))));
            check(os, Key::Delete, NONE, reset, Some(Intent::Reset));
            check(os, Key::Delete, NONE, any, None);
            check(os, Key::Backspace, NONE, any, None);
            check(os, Key::Delete, NONE, deleter, Some(Intent::delete(true, Granularity::Unit)));
            check(os, Key::Backspace, Mods::CMD, deleter, Some(Intent::delete(false, Granularity::Word)));
            check(os, Key::Delete, Mods::SHIFT, deleter, Some(Intent::shift_delete(true, Granularity::Unit)));
            check(os, Key::Down, Mods::ALT, nudge, None);
            check(os, Key::Left, NONE, KeyRole::new().owner_arrows(true), None);
        }
        check(Os::Mac, Key::PageDown, Mods::CTRL, any, None);
        check(Os::Mac, Key::PageDown, Mods::CMD, any, None);
        check(Os::Mac, Key::Enter, Mods::CMD, any, Some(Intent::SubmitForce));
        check(Os::Mac, Key::Enter, NONE, rename, Some(Intent::Rename));
        check(Os::Mac, Key::Down, Mods::CMD, rename, Some(Intent::Activate));
        check(Os::Mac, Key::Menu, NONE, any, None);
        check(Os::Mac, Key::CloseBracket, Mods { shift: true, ..Mods::CMD }, any, Some(Intent::switch_page(true)));
        check(Os::Mac, Key::Tab, Mods::CTRL, pages, Some(Intent::switch_page(true)));
        check(Os::Mac, Key::Left, Mods::ALT, any, Some(Intent::Move(NavMove::Back, word(false, false))));
        check(Os::Mac, Key::Left, Mods::CMD, any, Some(Intent::Move(NavMove::Start, MoveMods::new(false, true, Granularity::Line))));
        check(Os::Mac, Key::Down, Mods::CMD, any, Some(Intent::Move(NavMove::End, MoveMods::new(false, true, Granularity::Document))));
        for os in [Os::Linux, Os::Windows, Os::Mac] {
            check(os, Key::F10, Mods::SHIFT, any, Some(Intent::ContextMenu));
            check(os, Key::F10, NONE, any, None);
            check(os, Key::Right, Mods::SHIFT, any, Some(Intent::Move(NavMove::Forward, MoveMods::new(true, false, Granularity::Unit))));
            check(os, Key::Left, NONE, nudge, None);
            check(os, Key::PageUp, NONE, any, Some(Intent::Move(NavMove::PageUp, MoveMods::PLAIN)));
        }
    }

    #[test]
    fn alt_arrows_are_fine_moves_on_a_value_role() {
        let fine = |mv, extend| Some(Intent::Move(mv, MoveMods::new(extend, false, Granularity::Unit).fine(true)));
        let value = KeyRole::new().value(true);
        let number = value.text(true);
        let text = KeyRole::new().text(true);
        for os in [Os::Linux, Os::Windows, Os::Mac] {
            check(os, Key::Up, Mods::ALT, value, fine(NavMove::Up, false));
            check(os, Key::Down, Mods::ALT, value, fine(NavMove::Down, false));
            check(os, Key::Right, Mods::ALT, value, fine(NavMove::Forward, false));
            check(os, Key::Left, Mods { shift: true, ..Mods::ALT }, value, fine(NavMove::Back, true));
            check(os, Key::Up, Mods::ALT, number, fine(NavMove::Up, false));
            check(os, Key::Up, Mods::ALT, text, Some(Intent::disclose(DiscloseTo::Close, false)));
        }
        let word = MoveMods::new(false, false, Granularity::Word);
        check(Os::Mac, Key::Left, Mods::ALT, text, Some(Intent::Move(NavMove::Back, word)));
        check(Os::Mac, Key::Left, Mods::ALT, number, Some(Intent::Move(NavMove::Back, word)));
        check(Os::Linux, Key::Right, Mods::ALT, number, None);
    }

    #[test]
    fn a_canvas_control_takes_the_plain_deletes_and_leaves_the_modified_ones() {
        let canvas = KeyRole::new().nudge(true).deletes(true);
        let list = KeyRole::new().deletes(true);
        let text = KeyRole::new().text(true).deletes(true);
        for os in [Os::Linux, Os::Windows, Os::Mac] {
            check(os, Key::Backspace, NONE, canvas, Some(Intent::delete(false, Granularity::Unit)));
            check(os, Key::Delete, NONE, canvas, Some(Intent::delete(true, Granularity::Unit)));
            for mods in [Mods::ALT, Mods::SHIFT, Mods::CMD] {
                check(os, Key::Backspace, mods, canvas, None);
            }
            check(os, Key::Delete, Mods::SHIFT, list, Some(Intent::shift_delete(true, Granularity::Unit)));
        }
        check(Os::Linux, Key::Backspace, Mods::ALT, list, None);
        check(Os::Mac, Key::Backspace, Mods::ALT, text, Some(Intent::delete(false, Granularity::Word)));
        check(Os::Linux, Key::Backspace, Mods::ALT, text, None);
    }

    #[test]
    fn disclose_all_and_details_rows_follow_the_role_and_the_os() {
        let tree = KeyRole::new().disclose_all(true);
        let leaf = KeyRole::new().details(true);
        let all = |to| Some(Intent::disclose(to, true));
        check(Os::Mac, Key::Right, Mods::ALT, tree, all(DiscloseTo::Open));
        check(Os::Mac, Key::Left, Mods::ALT, tree, all(DiscloseTo::Close));
        // A role that does not handle disclose-all keeps the word move; so does a text role.
        let word = MoveMods::new(false, false, Granularity::Word);
        check(Os::Mac, Key::Right, Mods::ALT, KeyRole::new(), Some(Intent::Move(NavMove::Forward, word)));
        check(Os::Mac, Key::Right, Mods::ALT, KeyRole::new().text(true), Some(Intent::Move(NavMove::Forward, word)));
        check(Os::Linux, Key::Right, Mods::ALT, tree, None);
        check(Os::Linux, Key::Enter, Mods::ALT, leaf, Some(Intent::OpenDetails));
        check(Os::Windows, Key::Enter, Mods::ALT, leaf, Some(Intent::OpenDetails));
        check(Os::Mac, Key::Down, Mods::CMD, leaf, Some(Intent::OpenDetails));
        check(Os::Mac, Key::Enter, Mods::ALT, leaf, None);
        check(Os::Linux, Key::Enter, Mods::ALT, KeyRole::new(), None);
        check(Os::Mac, Key::Down, Mods::CMD, KeyRole::new(), Some(Intent::Move(NavMove::End, MoveMods::new(false, true, Granularity::Document))));
    }

    #[test]
    fn reaches_and_chords_are_the_rows_read_backwards() {
        let tree = KeyRole::new().disclose_all(true);
        let open_all = Intent::disclose(DiscloseTo::Open, true);
        assert_eq!(chords(Os::Mac, tree, &open_all), vec![Chord { mods: Mods::ALT, key: Key::Right }]);
        assert!(!reaches(Os::Mac, KeyRole::new(), &open_all));
        assert!(!reaches(Os::Linux, tree, &open_all));
        let open = Intent::disclose(DiscloseTo::Open, false);
        for os in [Os::Linux, Os::Windows, Os::Mac] {
            assert_eq!(chords(os, KeyRole::new(), &open), vec![Chord { mods: Mods::ALT, key: Key::Down }]);
            assert!(!reaches(os, KeyRole::new().nudge(true), &open));
            assert!(reaches(os, KeyRole::new(), &Intent::ContextMenu));
            assert!(!reaches(os, KeyRole::new(), &Intent::Rename));
            assert!(reaches(os, KeyRole::new().rename(true), &Intent::Rename));
            assert!(!reaches(os, KeyRole::new(), &Intent::Custom(ActionId::new("flip"))));
            assert!(!reaches(os, KeyRole::new(), &Intent::OpenDetails));
            assert!(reaches(os, KeyRole::new().details(true), &Intent::OpenDetails));
        }
        assert_eq!(chords(Os::Linux, KeyRole::new().details(true), &Intent::OpenDetails), vec![Chord { mods: Mods::ALT, key: Key::Enter }]);
        let activate = chords(Os::Linux, KeyRole::new(), &Intent::Activate);
        assert_eq!(activate.len(), 2);
        assert_eq!(chords(Os::Linux, KeyRole::new().space(true), &Intent::Activate).len(), 1);
    }

    #[test]
    fn a_nudge_is_reached_by_its_arrow_on_a_nudge_role_only() {
        let left = Intent::Nudge(Vec2::new(-1.0, 0.0), Mods::SHIFT);
        let role = KeyRole::new().nudge(true);
        assert_eq!(chords(Os::Linux, role, &left), vec![Chord { mods: Mods::SHIFT, key: Key::Left }]);
        assert!(!reaches(Os::Linux, KeyRole::new(), &left));
        assert!(!reaches(Os::Linux, role, &Intent::Nudge(Vec2::new(1.0, 1.0), Mods::NONE)));
    }
}
