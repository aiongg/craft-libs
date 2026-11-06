//! `craft_ui::text::elide`, measured in characters (one unit per char).

use std::borrow::Cow;

use super::elide::*;

fn width(s: &str) -> f32 {
    s.chars().count() as f32
}

// End keeps the start, Middle keeps a quarter at the end, Start keeps the end; text that fits is
// borrowed; a room narrower than "…" gives "…"; cuts never split a char.
#[test]
fn elide_cuts_where_asked_on_char_boundaries() {
    let name = "Background copy 12";
    assert!(matches!(elide(name, 18.0, ElideAt::End, width), Cow::Borrowed(_)));
    assert_eq!(elide(name, 10.0, ElideAt::End, width), "Backgroun…");
    assert_eq!(elide(name, 10.0, ElideAt::Start, width), "…d copy 12");
    // 10 room: 9 kept, a quarter (2) at the end.
    assert_eq!(elide("IMG_2024_holiday_final.psd", 10.0, ElideAt::Middle, width), "IMG_202…sd");
    assert_eq!(elide(name, 0.0, ElideAt::End, width), "…");
    assert_eq!(elide(name, 0.0, ElideAt::Middle, width), "…");
    // Multi-byte characters are cut whole.
    assert_eq!(elide("日本語のファイル名", 4.0, ElideAt::End, width), "日本語…");
    assert_eq!(elide("日本語のファイル名", 4.0, ElideAt::Start, width), "…イル名");
}

// A right-to-left name cut at its end keeps its logical start ("لاشي مك…"); a left-to-right name
// in a right-to-left UI still loses its end.
#[test]
fn right_to_left_text_keeps_its_logical_start() {
    assert_eq!(elide("لاشي مكتوب هنا", 8.0, ElideAt::End, width), "لاشي مك…");
    assert_eq!(elide("לשלום עולם גדול", 11.0, ElideAt::End, width), "לשלום עולם…");
    assert_eq!(elide("نص", 0.0, ElideAt::End, width), "…");
    assert_eq!(elide("Layer copy 12", 8.0, ElideAt::End, width), "Layer c…");
}

// "/Users/me/Pictures/Library" drops folders to "…/Pictures/Library", a Windows path keeps "\",
// a trailing separator is not a name, a last name too long loses its end, "///" gives "…".
#[test]
fn paths_drop_leading_folders_first() {
    let path = "/Users/me/Pictures/Library";
    assert!(matches!(elide_path(path, 26.0, width), Cow::Borrowed(_)));
    assert_eq!(elide_path(path, 20.0, width), "…/Pictures/Library");
    assert_eq!(elide_path("C:\\Users\\me\\Pictures", 12.0, width), "…\\Pictures");
    assert_eq!(elide_path("/Users/me/Pictures/", 12.0, width), "…/Pictures/");
    assert_eq!(elide_path("/Users/me/Pictures", 6.0, width), "Pictu…");
    assert_eq!(elide_path("///////////", 5.0, width), "…");
}

// elide_chars(name, 80, Middle) of a 200-char name keeps 59 + "…" + 20 chars.
#[test]
fn char_limits_keep_start_and_end() {
    let name: String = (0..200u32).map(|i| char::from_digit(i % 10, 10).unwrap()).collect();
    let cut = elide_chars(&name, 80, ElideAt::Middle);
    assert_eq!(cut.chars().count(), 80);
    assert_eq!(cut.chars().take(59).collect::<String>(), name[..59]);
    assert_eq!(cut.chars().nth(59), Some(ELLIPSIS));
    assert_eq!(cut.chars().skip(60).collect::<String>(), name[180..]);
    assert!(matches!(elide_chars("short", 80, ElideAt::End), Cow::Borrowed(_)));
    assert_eq!(elide_chars("abcdef", 4, ElideAt::End), "abc…");
    assert_eq!(elide_chars("abcdef", 4, ElideAt::Start), "…def");
}
