//! The one resolution of a type role to laid-out text: the font of a span, the paragraph job a run starts from, and
//! the width of a line. `TextRun` lays out through the first two and `StyleCx::text_width` through the third, so the
//! sizes that follow the text agree with the text that is drawn.

use super::Span;
use super::tokens::UiTokens;
use crate::fonts::StackFacts;
use crate::primitives::Services;
use craft_i18n::Direction;
use craft_theme::{FontSlot, Rgba, TypeRole};
use craft_ui_rt::{FontFace, RichText, TextLine, TextSection, TextWrap};
use egui::text::TextFormat;
use egui::{Color32, FontFamily};
use std::sync::Arc;

/// The format of `span` in `role`: its font at the role's size (mono, semibold or italic as the span is), with the
/// optical size axis where the face has one and tabular figures (`tnum`) for a tabular span. The span's text and
/// colour flags are not read.
pub(crate) fn span_format(tokens: &UiTokens, fonts: StackFacts, role: TypeRole, span: &Span<'_>, color: Color32, background: Color32) -> TextFormat {
    let (mono, italics) = (span.mono, span.italic);
    let font_id = tokens.font_weighted(fonts, role, mono, span.strong);
    let mut format = TextFormat { font_id, color, italics, background, ..TextFormat::default() };
    if let Some(size) = tokens.optical_size(fonts, role, mono) {
        format.coords.push(*b"opsz", size);
    }
    if span.tabular {
        format.font_features = egui::epaint::text::FontFeatures::new([(*b"tnum", 1)]);
    }
    format
}

/// A paragraph being assembled section by section.
pub(crate) struct RichJob {
    text: String,
    sections: Vec<TextSection>,
    pub(crate) wrap: TextWrap,
    rtl: bool,
    pub(crate) row_align: f32,
}

impl RichJob {
    /// Appends `text` set in `format`.
    pub(crate) fn append(&mut self, text: &str, format: craft_ui_rt::TextFormat) {
        let start = self.text.len();
        self.text.push_str(text);
        self.sections.push(TextSection { bytes: start..self.text.len(), format, indent: 0.0 });
    }

    pub(crate) fn finish(self) -> RichText {
        RichText { text: Arc::from(self.text), sections: self.sections, wrap: self.wrap, rtl: self.rtl, row_align: self.row_align }
    }
}

fn rgba(color: Color32) -> Rgba {
    Rgba(color.to_srgba_unmultiplied())
}

/// An egui text format as the engine's: the face by family (the proportional family is the interface slot, the mono
/// family the mono slot, a named family by its name), colours, lines, features and axes.
pub(crate) fn rich_format(format: &TextFormat) -> craft_ui_rt::TextFormat {
    let face = match &format.font_id.family {
        FontFamily::Proportional => FontFace::Slot(FontSlot::Ui),
        FontFamily::Monospace => FontFace::Slot(FontSlot::Mono),
        FontFamily::Name(name) => FontFace::Family(Arc::from(name.as_ref())),
    };
    let line = |stroke: egui::Stroke| (!stroke.is_empty()).then(|| TextLine { width: stroke.width, color: rgba(stroke.color) });
    craft_ui_rt::TextFormat {
        face,
        size: format.font_id.size,
        line_height: format.line_height,
        color: rgba(format.color),
        background: rgba(format.background),
        italic: format.italics,
        underline: line(format.underline),
        strikethrough: line(format.strikethrough),
        letter_spacing: format.extra_letter_spacing,
        features: format.font_features.as_ref().iter().map(|(tag, value)| (tag.to_be_bytes(), *value)).collect(),
        axes: format.coords.as_ref().iter().map(|(tag, value)| (tag.to_be_bytes(), *value)).collect(),
    }
}

/// The paragraph of an egui layout job, its rows placed by `row_align`.
pub(crate) fn rich_of_job(job: &egui::text::LayoutJob, row_align: f32) -> RichText {
    RichText {
        text: Arc::from(job.text.as_str()),
        sections: job
            .sections
            .iter()
            .map(|s| TextSection { bytes: s.byte_range.start.0..s.byte_range.end.0, format: rich_format(&s.format), indent: s.leading_space })
            .collect(),
        wrap: TextWrap {
            max_width: job.wrap.max_width,
            max_rows: job.wrap.max_rows,
            break_anywhere: job.wrap.break_anywhere,
            overflow: job.wrap.overflow_character,
        },
        rtl: job.direction == egui::epaint::text::TextDirection::RightToLeft,
        row_align,
    }
}

/// An empty paragraph of `direction`. Every row stays at the left edge unless the run aligns its rows
/// (`RichJob::row_align`): egui's own alignment trims the first glyph's offset from an aligned row only, so a run
/// would measure differently by direction.
pub(crate) fn paragraph_job(direction: Direction, ltr: bool, wrap: TextWrap) -> RichJob {
    let rtl = !ltr && direction == Direction::Rtl;
    RichJob { text: String::new(), sections: Vec::new(), wrap, rtl, row_align: 0.0 }
}

/// The width of `text` set on one line in the face of `role`.
pub(crate) fn line_width(services: &dyn Services, tokens: &UiTokens, fonts: StackFacts, direction: Direction, role: TypeRole, text: &str) -> f32 {
    let wrap = TextWrap { max_width: f32::INFINITY, max_rows: usize::MAX, break_anywhere: false, ..TextWrap::default() };
    let mut job = paragraph_job(direction, false, wrap);
    job.append(text, rich_format(&span_format(tokens, fonts, role, &Span::new(""), Color32::PLACEHOLDER, Color32::TRANSPARENT)));
    services.layout(&job.finish()).size.x
}
