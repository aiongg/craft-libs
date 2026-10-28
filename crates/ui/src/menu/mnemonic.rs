//! Mnemonics: the letter that runs a row from the keyboard in an open menu, underlined while Alt
//! is held or after the menu was opened from the keyboard, as Windows menus do.
//!
//! They are assigned per menu level from the labels as shown (so translated), unique within the
//! level: the row's preferred letter where the label has it, else the first unused letter, word
//! starts first. Only ASCII letters and digits are keys the keyboard reports as such, so a label
//! without one (Japanese, Chinese…) shows its English label's letter after it: "文件(F)"
//! ([`localized`]).

use egui::text::LayoutJob;
use egui::{Context, Event, Id, Key, Modifiers, Stroke, TextFormat};

fn setting_id() -> Id {
    Id::unique("craft-ui-menu-mnemonics")
}

/// Whether menus have mnemonics in `ctx`: their letters are underlined while Alt is held or after
/// a menu was opened from the keyboard, and a letter in an open menu runs its row. Off on macOS
/// (and in a browser there), where Option types characters and the native menus have none; on
/// elsewhere. [`set_mnemonics`] overrides it.
pub fn mnemonics(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(setting_id())).unwrap_or_else(|| !crate::controls::kbd::is_mac(ctx.os()))
}

/// Turns menu mnemonics on or off for `ctx` (an app built for macOS testing its Windows menus),
/// or back to the platform's default with `None`.
pub fn set_mnemonics(ctx: &Context, on: Option<bool>) {
    ctx.data_mut(|d| {
        if let Some(on) = on {
            d.insert_temp(setting_id(), on);
        } else {
            d.remove::<bool>(setting_id());
        }
    });
}

/// The mnemonic a key press stands for: the letter or digit of a plain key.
pub(crate) fn key_char(key: Key) -> Option<char> {
    let name = key.name();
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => Some(c.to_ascii_uppercase()),
        _ => None,
    }
}

/// Mnemonics for `labels`, unique within the group: the preferred letter when the label has it,
/// else the first unused ASCII letter or digit, trying the first character of each word before
/// the others. Compared case-insensitively; the returned letter is upper case.
pub(crate) fn assign(labels: &[&str], preferred: &[Option<char>]) -> Vec<Option<char>> {
    let mut used: Vec<char> = Vec::new();
    let mut out = Vec::with_capacity(labels.len());
    for (i, label) in labels.iter().enumerate() {
        let upper: Vec<char> = label.chars().map(|c| c.to_ascii_uppercase()).collect();
        let has = |c: char| upper.contains(&c.to_ascii_uppercase());
        let mut pick = preferred.get(i).copied().flatten().map(|c| c.to_ascii_uppercase()).filter(|&c| has(c) && !used.contains(&c));
        if pick.is_none() {
            let starts = label.split(|c: char| !c.is_alphanumeric()).filter_map(|w| w.chars().next());
            let rest = label.chars();
            pick = starts.chain(rest).filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).find(|c| !used.contains(c));
        }
        if let Some(c) = pick {
            used.push(c);
        }
        out.push(pick);
    }
    out
}

/// The labels to show and their mnemonics for `labels` as the app shows them, whose English
/// originals are `english` (same order; `None` when the app gave none). A label with an ASCII
/// letter or digit keeps its text and is assigned as [`assign`] does. One without (Chinese,
/// Japanese, Korean…) can't be typed on its own, so it shows its English label's letter after it
/// in parentheses and takes that letter, as Windows menus do in those languages ("文件(F)").
/// Without an English label it keeps its text and gets no mnemonic. Mnemonics stay unique within
/// the group, and a shown "(F)" is always the row's key.
pub(crate) fn localized(labels: &[&str], english: &[Option<&str>], preferred: &[Option<char>]) -> (Vec<String>, Vec<Option<char>>) {
    let typed = |l: &str| l.chars().any(|c| c.is_ascii_alphanumeric());
    // The key goes before a trailing ellipsis, as Windows writes it: "新建(N)…".
    let with_key = |l: &str, k: char| match l.strip_suffix('…').map(|b| (b, "…")).or_else(|| l.strip_suffix("...").map(|b| (b, "..."))) {
        Some((base, tail)) => format!("{base}({k}){tail}"),
        None => format!("{l}({k})"),
    };
    // Labels without an English one take part with a letterless stand-in, so the others keep
    // their places.
    let english: Vec<&str> = english.iter().map(|e| e.unwrap_or("")).collect();
    let english_keys = assign(&english, preferred);
    let mut shown = Vec::with_capacity(labels.len());
    let mut pref = Vec::with_capacity(labels.len());
    for (i, label) in labels.iter().enumerate() {
        match english_keys.get(i).copied().flatten().filter(|_| !typed(label)) {
            Some(k) => {
                shown.push(with_key(label, k));
                pref.push(Some(k));
            }
            None => {
                shown.push((*label).to_owned());
                pref.push(preferred.get(i).copied().flatten());
            }
        }
    }
    // The suffixed labels take their keys first: a label with letters of its own must not take
    // the key a suffix shows ("Camera Raw 滤镜…" before "转换为智能滤镜(C)" leaves C to the latter).
    let suffixed = |i: &usize| shown.get(*i).map(String::as_str) != labels.get(*i).copied();
    let order: Vec<usize> = (0..shown.len()).filter(suffixed).chain((0..shown.len()).filter(|i| !suffixed(i))).collect();
    let ordered: Vec<&str> = order.iter().map(|&i| shown.get(i).map_or("", String::as_str)).collect();
    let ordered_pref: Vec<Option<char>> = order.iter().map(|&i| pref.get(i).copied().flatten()).collect();
    let mut keys = vec![None; shown.len()];
    for (&i, key) in order.iter().zip(assign(&ordered, &ordered_pref)) {
        if let Some(slot) = keys.get_mut(i) {
            *slot = key;
        }
    }
    (shown, keys)
}

/// Appends `label` to `job` in `format`, with the first occurrence of `mnemonic`
/// (case-insensitively) underlined.
pub(crate) fn append_underlined(job: &mut LayoutJob, label: &str, mnemonic: Option<char>, format: &TextFormat) {
    let at = mnemonic.and_then(|m| label.char_indices().find(|(_, c)| c.to_ascii_uppercase() == m).map(|(i, c)| (i, c.len_utf8())));
    let parts = match at {
        Some((i, len)) => [label.get(..i), label.get(i..i + len), label.get(i + len..)],
        None => [Some(label), None, None],
    };
    let mut any = false;
    for (k, part) in parts.into_iter().enumerate() {
        let Some(part) = part.filter(|p| !p.is_empty()) else { continue };
        let underline = if k == 1 { Stroke::new(1.0, format.color) } else { Stroke::NONE };
        job.append(part, 0.0, TextFormat { underline, ..format.clone() });
        any = true;
    }
    if !any {
        job.append("", 0.0, format.clone());
    }
}

/// The first plain letter or digit pressed this pass (no Ctrl, Cmd or Alt), with its key.
pub(crate) fn letter_pressed(ctx: &Context) -> Option<(Key, char)> {
    ctx.input(|i| {
        i.events.iter().find_map(|e| match e {
            Event::Key { key, pressed: true, modifiers, .. } if !modifiers.command && !modifiers.ctrl && !modifiers.alt && !modifiers.mac_cmd => {
                key_char(*key).map(|c| (*key, c))
            }
            _ => None,
        })
    })
}

/// Takes `key` (with `mods`) out of this pass's events, and the text it typed: a letter a menu used
/// doesn't also land in a text field.
pub(crate) fn consume_letter(ctx: &Context, mods: Modifiers, key: Key, c: char) {
    ctx.input_mut(|i| {
        i.consume_key(mods, key);
        i.events.retain(|e| !matches!(e, Event::Text(t) if t.chars().count() == 1 && t.chars().all(|t| t.to_ascii_uppercase() == c)));
    });
}
