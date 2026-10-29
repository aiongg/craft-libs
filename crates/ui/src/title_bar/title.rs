//! The document title, centred in the bar between the app's groups.

use egui::text::{LayoutJob, TextWrapping};
use egui::{Rect, Ui, pos2};

use super::TitleBarStyle;
use crate::theme::{Tokens, TokensOf as _};

/// The text of the bar's centred title. The app chooses it, including whether it shows the file
/// name and what it says when no document is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Title {
    text: String,
    unsaved: bool,
}

impl Title {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), unsaved: false }
    }

    /// Marks the document as having unsaved changes with a bullet after the text.
    #[must_use]
    pub fn unsaved(mut self, unsaved: bool) -> Self {
        self.unsaved = unsaved;
        self
    }

    /// The text as drawn, and as the drag area is named.
    pub(super) fn text(&self) -> String {
        if self.unsaved { format!("{} \u{2022}", self.text) } else { self.text.clone() }
    }
}

/// Left edge for a title `width` wide: centred on `center` when it fits in the free gap from
/// `left` to `right` (the facing edges of the groups on either side, `gap` kept from each), else
/// slid into that gap; `None` when the gap is too small for the whole title.
pub(super) fn title_x(center: f32, left: f32, right: f32, width: f32, gap: f32) -> Option<f32> {
    let (lo, hi) = (left + gap, right - gap - width);
    if !(lo.is_finite() && hi.is_finite() && width.is_finite()) || hi < lo {
        return None;
    }
    if !center.is_finite() {
        return Some(lo);
    }
    Some((center - width / 2.0).clamp(lo, hi))
}

/// Where the title goes: the bar, the height its line centres on, the physical `left` and `right`
/// edges of the free gap, and the UI direction.
#[derive(Clone, Copy, Debug)]
pub(super) struct Placement {
    pub bar: Rect,
    pub centre_line: f32,
    pub left: f32,
    pub right: f32,
    pub rtl: bool,
}

/// Paints `title`: whole and centred when it fits, else slid into the gap, else cut with an
/// ellipsis at the side the gap's leading group is on, else nothing when the gap is not wider than
/// [`TitleBarStyle::title_min_gap`]. Paint only: a press on it reaches the drag area below.
///
/// The line is laid out left to right as a whole, so a right-to-left document name keeps its
/// suffix after it.
pub(super) fn paint(ui: &Ui, style: &TitleBarStyle, title: &Title, at: Placement) {
    let font = Tokens::of(ui).font(style.title_role);
    let painter = ui.painter();
    let text = title.text();
    let galley = painter.layout_no_wrap(text.clone(), font.clone(), style.title);
    let top = |height: f32| at.centre_line - height / 2.0;
    if let Some(x) = title_x(at.bar.center().x, at.left, at.right, galley.size().x, style.title_gap) {
        painter.galley(pos2(x, top(galley.size().y)), galley, style.title);
    } else if at.right - at.left > style.title_min_gap {
        let mut job = LayoutJob::simple_singleline(text, font, style.title);
        job.wrap = TextWrapping::truncate_at_width(at.right - at.left - 2.0 * style.title_gap);
        let cut = painter.layout_job(job);
        let x = if at.rtl { at.right - style.title_gap - cut.size().x } else { at.left + style.title_gap };
        painter.galley(pos2(x, top(cut.size().y)), cut, style.title);
    }
}
