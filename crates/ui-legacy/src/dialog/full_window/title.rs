//! [`FullWindowTitle`]: the text of a full-window dialog's title strip.

use egui::Context;

use super::labels::FullWindowLabels;
use crate::localized::{Labels as _, LocaleNumbers};

/// The title strip's text: the workspace's name, then in brackets what it works on and the zoom
/// on screen, "Liquify (Layer 1, 66%)". Without a subject "Liquify (66%)"; without a zoom
/// "Adaptive Wide Angle (Layer 1)"; with neither just the name.
///
/// - A trailing ellipsis of the name is dropped, so a menu item's name ("Liquify…") can be passed
///   as it is.
/// - The zoom is rounded to a whole percent in the locale's percent form ("66%", "66 %").
/// - Every workspace shows the zoom, Filter Gallery included (PhotoCraft's Gallery showed "Fit"
///   while fitted; the zoom bar's select says Fit in View instead).
///
/// ```ignore
/// FullWindowDialog::new("liquify")
///     .title(FullWindowTitle::new("Liquify…").subject(&layer_name).zoom(view.zoom_percent()).text(ctx))
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use = "make the text with `text`"]
pub struct FullWindowTitle {
    name: String,
    subject: Option<String>,
    zoom: Option<f64>,
    labels: Option<FullWindowLabels>,
}

impl FullWindowTitle {
    /// A title of the workspace `name` (translated by the app).
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), subject: None, zoom: None, labels: None }
    }

    /// What the workspace works on: the layer's or the document's name.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into()).filter(|s: &String| !s.is_empty());
        self
    }

    /// The zoom on screen as a percentage, [`FullWindowView::zoom_percent`](super::FullWindowView::zoom_percent).
    pub fn zoom(mut self, percent: f64) -> Self {
        self.zoom = Some(percent);
        self
    }

    /// The words to build the text with (default: the installed words).
    pub fn labels(mut self, labels: FullWindowLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The text: `fullWindow.titleZoom`, `titleSubject` or `titleZoomOnly` with `{name}`,
    /// `{subject}` and `{zoom}` filled, or the bare name.
    pub fn text(&self, ctx: &Context) -> String {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:980-987 (`"{} ({}, {:.0}%)"`, the name's
        // "…" trimmed), caf_ui/view.rs:293-300 (the same), wide_angle_ui.rs:273-275 (no zoom);
        // the percent through `LocaleNumbers::percent` instead of `{:.0}%`.
        let labels = self.labels.clone().unwrap_or_else(|| FullWindowLabels::of_ctx(ctx));
        let name = self.name.trim_end_matches('…').trim_end_matches("...").trim_end();
        let zoom = self.zoom.map(|percent| LocaleNumbers::of_ctx(ctx).percent(percent / 100.0));
        let fill = |template: &str, args: &[(&str, &str)]| craft_i18n::fmt(template, args);
        match (&self.subject, &zoom) {
            (Some(subject), Some(zoom)) => fill(&labels.title_zoom, &[("name", name), ("subject", subject), ("zoom", zoom)]),
            (Some(subject), None) => fill(&labels.title_subject, &[("name", name), ("subject", subject)]),
            (None, Some(zoom)) => fill(&labels.title_zoom_only, &[("name", name), ("zoom", zoom)]),
            (None, None) => name.to_owned(),
        }
    }
}
