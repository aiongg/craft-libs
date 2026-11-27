//! The key stage on craft-ui-rt (docs/contract-rt.md sections 4 and 10). ui-rt routes a platform
//! key event to the focused node and bubbles it up the node's ancestors until one handles it, then
//! to the window root. Each contract node claims at event time from what its last run kept, so no
//! run decides where a key goes:
//!
//! 1. The focused node takes the keys its `KeyUse` takes, as intents of the key table
//!    (`craft_behavior::keys::intent`) read in its role: activation, the moves it keeps, deletes,
//!    Enter as Submit when it submits, Tab when it keeps Tab, its edit chords as `Intent::Edit`, an
//!    Escape its run held as Cancel, a recorder's presses and claimed chords as chords. The intent
//!    is queued on the node (`NodeInput::Intent`), which marks its path, so its run sees it in the
//!    frame it arrived. A typing node also holds the plain keys that type (their text arrives as
//!    `Event::Text`), queuing nothing.
//! 2. A roving group's owner takes the moves its `Roving` takes ([`super::focus::rove`]).
//! 3. A trapping scope wraps Tab among its stops; a layer child takes the Enter its scope submits
//!    and the Escape its scope cancels (clear first, then close) ([`super::intents`]).
//! 4. The window root takes Tab and F6 ([`super::focus::window_keys`]), then runs the command the
//!    chord is bound to ([`super::commands`]).
//! 5. An Escape nothing took dismisses the topmost layer child that asks (ui-rt).

use craft_behavior::keys::intent;
use craft_behavior::{Intent, KeyRole, NavMove, OwnerClass};
use craft_keymap::Chord;
use craft_ui_rt::{EventCx, Key, KeyInput, Vec2};

use super::commands::CommandBoard;
use super::focus::FocusKept;
use super::node::NodeInput;
use crate::runtime::focus::{KeepKeys, KeyUse};

/// What the focused node does with a key.
pub(crate) enum Taken {
    /// It passes the key up its path.
    No,
    /// It holds the key and queues nothing: a typing key whose text follows, a repeat of a chord.
    Held,
    /// It takes the key as this input for its next run.
    Queue(NodeInput),
}

/// The key role of a focused stop with `keys`: which rows of the key table apply to it.
pub(crate) fn role_of(keys: &KeyUse) -> KeyRole {
    let mut role = KeyRole::new();
    role.value = keys.value;
    role.moves = keys.moves;
    role.nudge = keys.nudge;
    role.deletes = keys.deletes;
    role.type_ahead = keys.type_ahead;
    role.text = keys.owner == OwnerClass::Text;
    role.document = keys.chrome_key;
    role
}

/// Whether the move `mv` is a text caret's own: Home and End of a text field, which no roving group
/// around it takes.
fn caret_move(mv: NavMove, keys: &KeyUse) -> bool {
    matches!(mv, NavMove::Start | NavMove::End) && keys.owner == OwnerClass::Text
}

/// Whether a node with `keys` keeps the move `mv` from the roving group and scroll owner around it.
fn keeps_move(mv: NavMove, keys: &KeyUse) -> bool {
    if caret_move(mv, keys) || keys.moves || keys.value {
        return true;
    }
    match mv {
        NavMove::Back | NavMove::Forward => matches!(keys.keep, KeepKeys::InlineArrows | KeepKeys::BlockArrows | KeepKeys::Tab),
        NavMove::Up | NavMove::Down => matches!(keys.keep, KeepKeys::BlockArrows | KeepKeys::Tab),
        _ => keys.owner == OwnerClass::Text,
    }
}

fn is_arrow(key: Key) -> bool {
    matches!(key, Key::Up | Key::Down | Key::Left | Key::Right)
}

/// The text a plain press types: letters, digits, symbols and Space with no Cmd, Ctrl or Alt.
fn types(k: &KeyInput) -> bool {
    Chord::new(k.mods, k.key).is_plain() && (k.key == Key::Space || k.key.is_letter() || k.key.is_digit() || k.key.is_symbol())
}

fn queue(intent: Intent, k: &KeyInput) -> Taken {
    Taken::Queue(NodeInput::Intent { intent, horizontal: matches!(k.key, Key::Left | Key::Right), shift: k.mods.shift })
}

/// The page or end move a key a focused descendant left makes on the scroll owner it bubbled to (Page Up, Page
/// Down, Home, End), as that owner's next run takes it (`Focus::take_scroll_moves`).
pub(crate) fn scroll_move(cx: &EventCx<'_>, k: &KeyInput) -> Option<NodeInput> {
    if !k.down {
        return None;
    }
    match intent(cx.runtime().os(), k.key, k.mods, KeyRole::new()) {
        Some(Intent::Move(mv @ (NavMove::PageUp | NavMove::PageDown | NavMove::Start | NavMove::End), mods)) => {
            match queue(Intent::Move(mv, mods), k) {
                Taken::Queue(input) => Some(input),
                _ => None,
            }
        }
        _ => None,
    }
}

/// What the focused node with `focus` does with `k`.
pub(crate) fn take(focus: &FocusKept, cx: &EventCx<'_>, k: &KeyInput) -> Taken {
    let Some(keys) = focus.stop.as_ref() else { return Taken::No };
    if !k.down {
        return Taken::No;
    }
    let chord = Chord::new(k.mods, k.key);
    let typing = matches!(keys.owner, OwnerClass::Text | OwnerClass::Recorder);
    // A recorder reads every press as a chord; no row of the key table applies to it.
    if keys.owner == OwnerClass::Recorder || keys.claims.contains(&chord) {
        return if k.repeat { Taken::Held } else { Taken::Queue(NodeInput::Chord(chord)) };
    }
    if k.key == Key::Escape {
        return if focus.holds_escape && scope_escape(cx, k) { queue(Intent::Cancel, k) } else { Taken::No };
    }
    // The edit chords the component lists reach it as edits, ahead of the commands around it.
    if keys.edits != crate::runtime::focus::EditSet::NONE
        && let Some(edit) = cx.provided::<CommandBoard>().and_then(|board| board.edit_of(&chord))
        && keys.edits.contains(edit)
    {
        return queue(Intent::Edit(edit), k);
    }
    let plain = k.mods.is_none();
    match intent(cx.runtime().os(), k.key, k.mods, focus.role.clone().unwrap_or_else(|| role_of(keys))) {
        Some(Intent::Move(mv, mods)) if keeps_move(mv, keys) => queue(Intent::Move(mv, mods), k),
        Some(Intent::Move(..)) => Taken::No,
        Some(Intent::Activate) if keys.activate.enter() || keys.activate.space() => queue(Intent::Activate, k),
        Some(Intent::Activate) => Taken::No,
        Some(Intent::SubmitForce) if k.key == Key::Enter && keys.activate.enter() => queue(Intent::Activate, k),
        Some(Intent::SubmitForce) if keys.submits => queue(Intent::SubmitForce, k),
        Some(Intent::SubmitForce) => Taken::No,
        Some(Intent::Delete { .. }) if !keys.deletes => Taken::No,
        Some(found) => queue(found, k),
        None => match k.key {
            Key::Enter if keys.activate.enter() && plain && !keys.submits => queue(Intent::Activate, k),
            Key::Enter if keys.submits && !k.mods.cmd && !k.mods.ctrl => queue(Intent::Submit, k),
            Key::Space if keys.activate.space() && plain && !typing => queue(Intent::Activate, k),
            Key::Tab if keys.keep == KeepKeys::Tab && !(k.mods.cmd || k.mods.ctrl || k.mods.alt) => {
                queue(if k.mods.shift { Intent::BackTab } else { Intent::Tab }, k)
            }
            key if keys.nudge && is_arrow(key) => Taken::Queue(NodeInput::Nudge { direction: nudge_direction(key), mods: k.mods }),
            _ if (typing || keys.type_ahead) && types(k) => Taken::Held,
            _ => Taken::No,
        },
    }
}

/// Whether an Escape press is the scopes' escape (an open draft's Cancel, a layer's dismissal): a
/// plain Escape, or one with Shift or Alt that the window's keymap binds to no enabled command,
/// which is that command's chord (a place cursor's Shift+Escape). Cmd and Ctrl chords with Escape
/// are commands' keys.
pub(crate) fn scope_escape(cx: &EventCx<'_>, k: &KeyInput) -> bool {
    if k.mods.cmd || k.mods.ctrl {
        return false;
    }
    k.mods.is_none()
        || cx.provided::<CommandBoard>().is_none_or(|board| {
            board.command_for(&Chord::new(k.mods, k.key), false, super::commands::ModalAt::Free, super::commands::FocusedEdits::NONE).is_none()
        })
}

fn nudge_direction(key: Key) -> Vec2 {
    match key {
        Key::Left => Vec2::new(-1.0, 0.0),
        Key::Right => Vec2::new(1.0, 0.0),
        Key::Up => Vec2::new(0.0, -1.0),
        _ => Vec2::new(0.0, 1.0),
    }
}

/// The intent typed or pasted `text` is for a focused node with `keys`: text for a typing node and,
/// when it holds letters, for one that types ahead; `None` for any other node.
pub(crate) fn text_intent(keys: Option<&KeyUse>, text: &str) -> Option<Intent> {
    let keys = keys?;
    let letters = !text.trim().is_empty();
    (keys.owner == OwnerClass::Text || (keys.type_ahead && letters)).then(|| Intent::Text(std::sync::Arc::from(text)))
}
