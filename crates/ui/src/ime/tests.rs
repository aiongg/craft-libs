use super::*;
use egui::{Context, Event, ImeEvent, Key, Modifiers, PointerButton, RawInput, Rect, pos2, vec2};

fn preedit(text: &str, active: Option<std::ops::Range<usize>>) -> Event {
    Event::Ime(ImeEvent::Preedit { text: text.into(), active_range_chars: active })
}

fn commit(text: &str) -> Event {
    Event::Ime(ImeEvent::Commit(text.into()))
}

fn key_with(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

fn key(key: Key) -> Event {
    key_with(key, Modifiers::NONE)
}

fn text(t: &str) -> Event {
    Event::Text(t.into())
}

fn click() -> Event {
    Event::PointerButton { pos: pos2(5.0, 5.0), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }
}

/// Feeds passes of `(time, events)` and returns the text the editor ends with and every key it
/// was given. Backspace deletes the last char; other keys are only recorded.
fn type_passes(composer: &mut Composer, passes: &[(f64, Vec<Event>)]) -> (String, Vec<Key>) {
    let mut doc = String::new();
    let mut keys = Vec::new();
    for (time, events) in passes {
        for edit in composer.process(events, *time).edits {
            match edit {
                Edit::Insert(t) => doc.push_str(&t),
                Edit::Key { key: Key::Backspace, .. } => {
                    doc.pop();
                    keys.push(Key::Backspace);
                }
                Edit::Key { key, .. } => keys.push(key),
                _ => {}
            }
        }
    }
    (doc, keys)
}

fn inserts(edits: &[Edit]) -> Vec<&str> {
    edits.iter().filter_map(|e| if let Edit::Insert(t) = e { Some(t.as_str()) } else { None }).collect()
}

#[test]
fn pinyin_on_wayland_commits_once_and_ignores_trailing_empty_preedits() {
    let mut c = Composer::new();
    let (doc, keys) = type_passes(
        &mut c,
        &[
            (0.0, vec![preedit("c", Some(1..1))]),
            (0.1, vec![preedit("ce", Some(2..2))]),
            (0.2, vec![preedit("ce shi", Some(6..6))]),
            // Space picks the first candidate (fcitx5 on GNOME Wayland).
            (0.3, vec![preedit("", None), commit("测试"), preedit("", Some(0..0)), preedit("", None)]),
            (0.4, vec![preedit("", None)]),
            (1.0, vec![preedit("", None), key(Key::Enter)]),
        ],
    );
    assert_eq!(doc, "测试");
    assert_eq!(keys, [Key::Enter], "an Enter long after the commit is the user's");
    assert!(!c.is_composing());
}

#[test]
fn pinyin_on_windows_keeps_candidate_keys_for_the_ime() {
    let mut c = Composer::new();
    let (doc, keys) = type_passes(
        &mut c,
        &[
            (0.0, vec![preedit("ni", Some(2..2))]),
            // Picking candidate 2: the key and its text are the IME's.
            (0.1, vec![key(Key::Num2), text("2"), preedit("你", Some(1..1))]),
            #[expect(deprecated)]
            (0.2, vec![preedit("你", Some(1..1)), preedit("", None), commit("你"), Event::Ime(ImeEvent::Disabled)]),
        ],
    );
    assert_eq!(doc, "你");
    assert!(keys.is_empty());
}

#[test]
fn korean_two_set_splits_syllables_and_types_one_space() {
    let mut c = Composer::new();
    let (doc, keys) = type_passes(
        &mut c,
        &[
            // g k s: ㅎ 하 한, the first key composing like the rest.
            (0.0, vec![preedit("ㅎ", None)]),
            (0.1, vec![preedit("하", None)]),
            (0.2, vec![preedit("한", None)]),
            // k moves ㄴ to the next syllable: 하 is committed, 나 composes.
            (0.3, vec![commit("하"), preedit("나", None)]),
            // Space after a syllable: commit, then the space itself.
            (0.4, vec![commit("나"), key(Key::Space), text(" ")]),
            // Apple Korean's Space with no syllable: committed and also sent as the key's text.
            (0.8, vec![preedit(" ", None), commit(" "), preedit("", None), key(Key::Space), text(" ")]),
            // 한 then 5: the digit after the syllable is kept.
            (1.2, vec![preedit("한", None)]),
            (1.3, vec![commit("한"), key(Key::Num5), text("5")]),
        ],
    );
    assert_eq!(doc, "하나  한5");
    assert_eq!(keys, [Key::Num5]);
}

#[test]
fn korean_backspace_that_empties_the_syllable_deletes_nothing_else() {
    let mut c = Composer::new();
    let (doc, keys) = type_passes(
        &mut c,
        &[
            (0.0, vec![text("a")]),
            (0.1, vec![preedit("ㅎ", None)]),
            (0.2, vec![preedit("", None), key(Key::Backspace)]),
            (0.6, vec![key(Key::Backspace)]),
        ],
    );
    assert_eq!(doc, "", "only the second Backspace reaches the text");
    assert_eq!(keys, [Key::Backspace]);
}

#[test]
fn a_raw_first_jamo_is_typed_not_lost() {
    // Unpatched macOS sends the first key in a fresh app as a raw jamo (fixed in the shared
    // winit); it is still typed.
    let mut c = Composer::new();
    let (doc, _) = type_passes(&mut c, &[(0.0, vec![key(Key::G), text("ㅎ")]), (0.1, vec![preedit("ㅏ", None)])]);
    assert_eq!(doc, "ㅎ");
    assert_eq!(c.preedit().map(|p| p.text), Some("ㅏ"));
}

#[test]
fn japanese_conversion_tracks_the_segment_and_guards_the_confirming_enter() {
    let mut c = Composer::new();
    c.process(&[preedit("にほんご", Some(4..4))], 0.0);
    // Space converts: the key is the IME's.
    let p = c.process(&[key(Key::Space), preedit("日本語", Some(0..3))], 0.1);
    assert!(p.events.is_empty(), "the converting Space is removed from the app's input");
    assert_eq!(c.preedit().and_then(|p| p.active.cloned()), Some(0..3));
    // Moving to the next segment.
    c.process(&[key(Key::ArrowRight), preedit("日本語", Some(2..3))], 0.2);
    assert_eq!(c.preedit().and_then(|p| p.active.cloned()), Some(2..3));
    // An out-of-range segment is dropped rather than trusted.
    c.process(&[preedit("日本語", Some(2..9))], 0.25);
    assert_eq!(c.preedit().map(|p| p.active), Some(None));
    let p = c.process(&[commit("日本語"), key(Key::Enter)], 0.3);
    assert_eq!(inserts(&p.edits), ["日本語"]);
    assert!(!p.edits.iter().any(|e| matches!(e, Edit::Key { .. })), "the confirming Enter doesn't submit");
    // The next Enter is the user's.
    let p = c.process(&[key(Key::Enter)], 0.9);
    assert_eq!(p.edits, [Edit::Key { key: Key::Enter, modifiers: Modifiers::NONE, repeat: false }]);
    // An Enter forwarded in the pass after the commit is still the IME's.
    c.process(&[preedit("か", None)], 1.0);
    c.process(&[commit("課")], 1.1);
    let p = c.process(&[key(Key::Enter)], 1.11);
    assert!(p.edits.is_empty());
}

#[test]
fn a_bare_line_break_commit_keeps_the_marked_text() {
    let mut c = Composer::new();
    let (doc, keys) = type_passes(&mut c, &[(0.0, vec![preedit("かな", None)]), (0.1, vec![commit("\n"), key(Key::Enter)])]);
    assert_eq!(doc, "かな");
    assert!(keys.is_empty());
}

#[test]
fn vietnamese_telex_keeps_backspace_between_its_inserts() {
    // Unikey without preedit: "toaf" replaces the a with à by Backspace and text.
    let mut c = Composer::new();
    let p = c.process(&[text("t"), text("o"), text("a"), key(Key::Backspace), text("à")], 0.0);
    assert_eq!(
        p.edits,
        [
            Edit::Insert("t".into()),
            Edit::Insert("o".into()),
            Edit::Insert("a".into()),
            Edit::Key { key: Key::Backspace, modifiers: Modifiers::NONE, repeat: false },
            Edit::Insert("à".into()),
        ]
    );
    // IBus-Unikey with preedit: the word composes and Space commits it, then types the space.
    let (doc, _) = type_passes(
        &mut c,
        &[
            (1.0, vec![preedit("t", None)]),
            (1.1, vec![preedit("to", None)]),
            (1.2, vec![preedit("toa", None)]),
            (1.3, vec![preedit("toà", None)]),
            (1.4, vec![preedit("", None), commit("toà"), key(Key::Space), text(" ")]),
        ],
    );
    assert_eq!(doc, "toà ");
}

#[test]
fn commits_without_a_composition_leave_the_next_keys_to_the_user() {
    // IBus and fcitx send plain letters as commits.
    let mut c = Composer::new();
    let (doc, keys) = type_passes(&mut c, &[(0.0, vec![commit("a"), commit("b"), key(Key::Backspace)])]);
    assert_eq!(doc, "a");
    assert_eq!(keys, [Key::Backspace]);
    for k in [Key::Enter, Key::Escape, Key::Delete, Key::Space] {
        let (_, keys) = type_passes(&mut c, &[(1.0, vec![commit("x"), key(k)])]);
        assert_eq!(keys, [k], "{k:?} after a bare commit is the user's");
    }
    // Telex through IBus without preedit: Backspace replaces the a.
    let (doc, _) = type_passes(&mut c, &[(2.0, vec![commit("t"), commit("o"), commit("a"), key(Key::Backspace), commit("à")])]);
    assert_eq!(doc, "toà");
    // A bare commit of a space guards only the Space key that typed it.
    let (doc, keys) = type_passes(&mut c, &[(3.0, vec![commit(" "), key(Key::Space), text(" ")]), (3.01, vec![commit(" "), key(Key::Backspace)])]);
    assert_eq!(doc, " ");
    assert_eq!(keys, [Key::Backspace]);
}

#[test]
fn a_line_break_with_nothing_composing_is_enter() {
    let enter = |modifiers| Edit::Key { key: Key::Enter, modifiers, repeat: false };
    let mut c = Composer::new();
    assert_eq!(c.process(&[commit("\n")], 0.0).edits, [enter(Modifiers::NONE)]);
    assert_eq!(c.process(&[text("\n")], 1.0).edits, [enter(Modifiers::NONE)]);
    // The key and its text, in either order, are one Enter.
    assert_eq!(c.process(&[key(Key::Enter), text("\r")], 2.0).edits, [enter(Modifiers::NONE)]);
    assert_eq!(c.process(&[commit("\n"), key(Key::Enter), text("\n")], 3.0).edits, [enter(Modifiers::NONE)]);
    // Shift+Enter stays Shift+Enter.
    let p = c.process(&[Event::ModifiersChanged(Modifiers::SHIFT), commit("\r")], 4.0);
    assert_eq!(p.edits, [enter(Modifiers::SHIFT)]);
}

#[test]
fn dead_keys_compose_or_arrive_as_text() {
    let mut c = Composer::new();
    // macOS: ⌥E marks the accent, the next key commits the letter.
    let (doc, _) = type_passes(&mut c, &[(0.0, vec![preedit("´", None)]), (0.1, vec![preedit("", None), commit("í")])]);
    assert_eq!(doc, "í");
    // Windows, X11 and Wayland compose in the keyboard layout: only the result arrives.
    let (doc, _) = type_passes(&mut c, &[(1.0, vec![text("é")])]);
    assert_eq!(doc, "é");
    // Escape cancels a marked accent without reaching the editor.
    let (doc, keys) = type_passes(&mut c, &[(2.0, vec![preedit("¨", None)]), (2.1, vec![preedit("", None), key(Key::Escape)])]);
    assert_eq!(doc, "");
    assert!(keys.is_empty());
}

#[test]
fn a_click_while_composing_keeps_the_text_and_drops_the_late_commit() {
    let mut c = Composer::new();
    c.process(&[preedit("ce", None)], 0.0);
    let p = c.process(&[click()], 0.1);
    assert_eq!(p.edits, [Edit::Preedit { text: String::new(), active: None }, Edit::Insert("ce".into())]);
    assert_eq!(p.events, [click()], "the app still handles the click");
    // Windows commits the dropped composition when the IME is interrupted.
    let p = c.process(&[preedit("", Some(0..0)), commit("ce")], 0.15);
    assert!(inserts(&p.edits).is_empty());
    // The same text typed later is typed.
    let p = c.process(&[commit("ce")], 2.0);
    assert_eq!(inserts(&p.edits), ["ce"]);
}

#[test]
fn shortcut_chords_dont_type_but_altgr_does() {
    let mut c = Composer::new();
    let p = c.process(&[key_with(Key::S, Modifiers::COMMAND), text("s")], 0.0);
    assert!(inserts(&p.edits).is_empty());
    assert!(matches!(p.edits[..], [Edit::Key { key: Key::S, .. }]));
    let altgr = Modifiers { alt: true, ctrl: true, ..Modifiers::NONE };
    let p = c.process(&[key_with(Key::Q, altgr), text("@")], 0.1);
    assert_eq!(inserts(&p.edits), ["@"]);
}

#[test]
fn clipboard_waits_for_the_composition() {
    let mut c = Composer::new();
    let p = c.process(&[preedit("ni", None), Event::Paste("x".into()), Event::Copy], 0.0);
    assert!(!p.edits.iter().any(|e| matches!(e, Edit::Paste(_) | Edit::Copy)));
    let p = c.process(&[preedit("", None), Event::Paste("x".into())], 0.1);
    assert!(p.edits.contains(&Edit::Paste("x".into())));
}

fn run(ctx: &Context, time: f64, events: Vec<Event>, f: impl FnMut(&Context)) -> egui::FullOutput {
    let raw = RawInput { time: Some(time), events, screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))), ..Default::default() };
    let mut f = f;
    let mut out = ctx.run_ui(raw, |ui| f(ui.ctx()));
    out.textures_delta.clear();
    out
}

#[test]
fn the_ime_is_on_at_the_caret_only_while_editing() {
    let ctx = Context::default();
    let mut c = Composer::new();
    let caret = Rect::from_min_size(pos2(100.0, 50.0), vec2(0.0, 16.0));
    let out = run(&ctx, 0.0, vec![preedit("に", None), key(Key::Space)], |ctx| {
        c.update(ctx, true);
        assert!(ctx.input(|i| i.events.is_empty()), "the IME's events are removed for later handlers");
        c.set_caret(ctx, caret);
    });
    let ime = out.platform_output.ime.expect("IME on while a caret is on the canvas");
    assert_eq!(ime.cursor_rect, Rect::from_min_size(pos2(100.0, 50.0), vec2(1.0, 16.0)));
    assert!(!ime.should_interrupt_composition);

    // A click away keeps the text and interrupts the IME.
    let mut edits = Vec::new();
    let out = run(&ctx, 0.1, vec![click()], |ctx| {
        edits = c.update(ctx, true);
        c.set_caret(ctx, caret);
    });
    assert!(edits.contains(&Edit::Insert("に".into())));
    assert!(out.platform_output.ime.is_some_and(|i| i.should_interrupt_composition));

    // Leaving the canvas keeps marked text and turns the IME off.
    run(&ctx, 0.2, vec![preedit("か", None)], |ctx| {
        c.update(ctx, true);
    });
    let out = run(&ctx, 0.3, vec![], |ctx| edits = c.update(ctx, false));
    assert_eq!(edits, [Edit::Preedit { text: String::new(), active: None }, Edit::Insert("か".into())]);
    assert!(out.platform_output.ime.is_none());
}

#[test]
fn update_starts_from_the_modifiers_held() {
    // Cmd went down while another view had the keyboard; its ModifiersChanged went there.
    let ctx = Context::default();
    let mut c = Composer::new();
    run(&ctx, 0.0, vec![Event::ModifiersChanged(Modifiers::COMMAND)], |_| {});
    let mut edits = Vec::new();
    run(&ctx, 0.1, vec![text("s")], |ctx| edits = c.update(ctx, true));
    assert!(edits.is_empty(), "Cmd+S doesn't type an s");
}

#[test]
fn a_caret_off_the_canvas_keeps_the_candidates_on_screen() {
    let ctx = Context::default();
    let mut c = Composer::new();
    let out = run(&ctx, 0.0, vec![], |ctx| {
        c.update(ctx, true);
        c.set_caret(ctx, Rect::from_min_size(pos2(-50.0, 900.0), vec2(2.0, 20.0)));
    });
    let r = out.platform_output.ime.map(|i| i.cursor_rect);
    assert_eq!(r, Some(Rect::from_min_size(pos2(0.0, 580.0), vec2(2.0, 20.0))));
}
