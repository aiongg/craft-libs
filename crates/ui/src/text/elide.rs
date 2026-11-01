//! Long text shortened with an ellipsis to fit (N11.03): at its end, in its middle, or at its
//! start; paths shortened from the front so the folder or file name that says the most stays.
//!
//! - [`elide`], [`elide_path`], [`elide_chars`]: the text to show, measured by the caller's
//!   `width` (usually [`text_width`]) or counted in characters. They return the text unchanged
//!   (borrowed) when it fits.
//! - [`elided_galley`]: one row laid out no wider than a width, for painters (tab strips, layer
//!   rows, table cells).
//! - The label is [`Text`](crate::primitives::Text) with [`elide`](crate::primitives::Text::elide)
//!   or [`elide_path`](crate::primitives::Text::elide_path): when it shortens its text, the
//!   tooltip and the AccessKit node keep the whole text, so a screen reader and a hover always get
//!   all of it.
//!
//! Cuts fall between characters (`char`s), never inside one. A right-to-left paragraph (its first
//! strong letter is Hebrew, Arabic…) cut at its end keeps its logical start, so the "…" lands at
//! the left, where a right-to-left line ends (PhotoCraft #5159). The result is never empty: a room
//! narrower than "…" still gets "…". craft-text's `bidi::elide` has the same right-to-left rule
//! for apps without craft-ui; craft-ui can't call it (craft-text's `ui` feature depends on
//! craft-ui), so the check (first strong character R or AL, Unicode rule P2) is repeated here.

use std::borrow::Cow;
use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, FontId, Galley, Ui};
use unicode_bidi::{BidiClass, bidi_class};

/// The ellipsis every elision uses: U+2026.
pub const ELLIPSIS: char = '…';

/// Where text loses its characters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ElideAt {
    /// `Background copy 1…`: names, titles, labels beside fixed controls.
    #[default]
    End,
    /// `IMG_2024…_final.psd`: file names, where the extension and version suffix at the end say as
    /// much as the start. The end keeps a quarter of the room (PdfCraft `shorten_middle`).
    Middle,
    /// `…Pictures/Library`: anything whose end says the most.
    Start,
}

/// `text` shortened at `at` until `width` of it, with [`ELLIPSIS`], is at most `max_width`.
/// Borrowed when `text` fits as it is. The longest fitting cut is found by bisection over the
/// character boundaries, so `width` runs about `log2(chars)` times.
// pc: crates/ui-egui/src/text/bidi_text.rs:25 elide
// lightcraft: crates/ui-egui/src/widgets.rs:1189 elide_head
pub fn elide<'a>(text: &'a str, max_width: f32, at: ElideAt, width: impl Fn(&str) -> f32) -> Cow<'a, str> {
    if width(text) <= max_width {
        return Cow::Borrowed(text);
    }
    let ends: Vec<usize> = text.char_indices().map(|(i, _)| i).skip(1).chain(std::iter::once(text.len())).collect();
    let total = ends.len();
    let rtl = is_rtl_paragraph(text);
    // `kept` characters of `text` (never all of them) with the ellipsis where `at` says.
    let cut = |kept: usize| -> String {
        let byte_end = |n: usize| if n == 0 { 0 } else { ends.get(n - 1).copied().unwrap_or(text.len()) };
        match at {
            ElideAt::End => {
                let head = text.get(..byte_end(kept)).unwrap_or_default();
                // A right-to-left line ends at its left: a space before the "…" would show as a gap.
                format!("{}{ELLIPSIS}", if rtl { head.trim_end() } else { head })
            }
            ElideAt::Start => format!("{ELLIPSIS}{}", text.get(byte_end(total - kept)..).unwrap_or_default()),
            ElideAt::Middle => {
                let tail = kept / 4;
                let head = kept - tail;
                format!("{}{ELLIPSIS}{}", text.get(..byte_end(head)).unwrap_or_default(), text.get(byte_end(total - tail)..).unwrap_or_default())
            }
        }
    };
    // The most characters kept with the cut still fitting; `width` only grows with `kept`.
    let (mut lo, mut hi) = (0usize, total - 1);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if width(&cut(mid)) <= max_width { lo = mid } else { hi = mid - 1 }
    }
    Cow::Owned(cut(lo))
}

/// A folder or file path shortened to `max_width` by dropping leading folders first, after
/// either kind of separator (`/Users/me/Pictures/Library` → `…/Pictures/Library`,
/// `C:\Users\me\Pictures` → `…\Pictures`), so its end stays: the end says which folder it is. A
/// last name too long by itself loses its end ([`ElideAt::End`]). A trailing separator is not
/// where a name starts (`…/` alone would say nothing); nothing but separators gives "…".
// lightcraft: crates/ui-egui/src/dialogs/settings/widgets.rs:7 elide_path
pub fn elide_path<'a>(path: &'a str, max_width: f32, width: impl Fn(&str) -> f32) -> Cow<'a, str> {
    if width(path) <= max_width {
        return Cow::Borrowed(path);
    }
    // A separator at the very end is not where a name starts: `…/` alone would say nothing.
    let end = path.trim_end_matches(['/', '\\']).len();
    let mut rest = path;
    while let Some(i) = rest.find(['/', '\\']).filter(|i| path.len() - rest.len() + i + 1 < end) {
        let separator = if rest.get(i..).is_some_and(|r| r.starts_with('\\')) { '\\' } else { '/' };
        rest = rest.get(i + 1..).unwrap_or("");
        let shorter = format!("{ELLIPSIS}{separator}{rest}");
        if width(&shorter) <= max_width {
            return Cow::Owned(shorter);
        }
    }
    let name = rest.trim_end_matches(['/', '\\']);
    if name.is_empty() {
        return Cow::Owned(ELLIPSIS.to_string());
    }
    // Only the last name is left and it is still too long.
    Cow::Owned(elide(name, max_width, ElideAt::End, width).into_owned())
}

/// `text` cut to at most `max_chars` characters with [`ELLIPSIS`] where `at` says, for text in
/// a message or title that wraps but must not grow without end (a 2 000-character web URL as a
/// document name, PdfCraft #236). [`ElideAt::Middle`] keeps `max_chars / 4` at the end and the
/// rest at the start.
// pdfcraft: crates/ui-egui/src/dialogs/prompts.rs:133 shorten_middle
pub fn elide_chars(text: &str, max_chars: usize, at: ElideAt) -> Cow<'_, str> {
    let count = text.chars().count();
    if count <= max_chars {
        return Cow::Borrowed(text);
    }
    let byte_at = |n: usize| text.char_indices().nth(n).map_or(text.len(), |(i, _)| i);
    let kept = max_chars.saturating_sub(1);
    Cow::Owned(match at {
        ElideAt::End => format!("{}{ELLIPSIS}", &text[..byte_at(kept)]),
        ElideAt::Start => format!("{ELLIPSIS}{}", &text[byte_at(count - kept)..]),
        ElideAt::Middle => {
            let tail = max_chars / 4;
            let head = max_chars.saturating_sub(tail + 1);
            format!("{}{ELLIPSIS}{}", &text[..byte_at(head)], &text[byte_at(count - tail)..])
        }
    })
}

/// The width of `text` on one line in `font`, without wrapping: the `width` the functions above
/// take.
pub fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter().layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER).size().x
}

/// A galley [`elided_galley`] laid out, and whether it lost characters.
#[derive(Clone, Debug)]
pub struct Elided {
    pub galley: Arc<Galley>,
    pub elided: bool,
}

/// `text` on one row no wider than `max_width` in `format` (font, colour, italics), shortened at
/// `at`.
/// [`ElideAt::End`] of left-to-right text uses egui's own overflow character (cheapest); the
/// other cases lay out the result of [`elide`]. A `max_width` under 1 is taken as 1.
// pc: crates/ui-egui/src/widgets/tab_strip.rs:19 elided, crates/ui-egui/src/panels/layer_row_ui.rs:117 truncated
pub fn elided_galley(ui: &Ui, text: &str, format: TextFormat, max_width: f32, at: ElideAt) -> Elided {
    let max_width = max_width.max(1.0);
    let wrap = TextWrapping { max_width, max_rows: 1, break_anywhere: true, overflow_character: Some(ELLIPSIS) };
    let (shown, shortened) = if at == ElideAt::End && !is_rtl_paragraph(text) {
        (Cow::Borrowed(text), false)
    } else {
        let font = format.font_id.clone();
        let shown = elide(text, max_width, at, |s| text_width(ui, s, &font));
        let shortened = matches!(shown, Cow::Owned(_));
        (shown, shortened)
    };
    let mut job = LayoutJob::single_section(shown.into_owned(), format);
    job.wrap = wrap;
    let galley = ui.painter().layout_job(job);
    let elided = shortened || galley.elided;
    Elided { galley, elided }
}

/// Whether `text` reads right to left as a paragraph: its first strong letter is Hebrew, Arabic…
fn is_rtl_paragraph(text: &str) -> bool {
    text.chars()
        .find_map(|c| match bidi_class(c) {
            BidiClass::L => Some(false),
            BidiClass::R | BidiClass::AL => Some(true),
            _ => None,
        })
        .unwrap_or(false)
}
