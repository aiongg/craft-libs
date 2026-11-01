//! A decoded frame fitted in a panel, with a status line (N1.11).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Painter, Rect, Response, TextureId, Ui, Vec2};

/// How a frame fills its box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FrameFit {
    /// Scaled to fit, centred, the rest black (letterbox or pillarbox).
    #[default]
    Letterbox,
    /// The part a crop leaves (left, top, right, bottom fractions, each clamped to 0…0.99)
    /// stretched over the whole box, as PowerPoint shows a cropped video.
    Crop([f32; 4]),
}

/// Paint the frame `texture` (`size` pixels) into `rect` on black, fitted by `fit`. Non-finite
/// crop values count as 0; the visible part keeps at least 1 % of each axis.
// deckcraft: crates/ui-egui/src/media.rs:417 `paint_frame`, `:410` `crop_uv`.
pub fn paint_frame(painter: &Painter, rect: Rect, texture: TextureId, size: Vec2, fit: FrameFit) {
    todo!()
}

/// A preview panel: the frame fitted on black and, under it, a status line in the `caption`
/// role ("Movie.mov · 00:01:02:03", "Offline", a decode error). Without a frame it shows the
/// status alone, centred. Not interactive; AccessKit: an image named by the status.
pub struct FramePreview<'a> {
    id_salt: IdSalt,
    frame: Option<(TextureId, Vec2)>,
    fit: FrameFit,
    status: Option<&'a str>,
}

impl<'a> FramePreview<'a> {
    pub fn new(id_salt: impl AsIdSalt, frame: Option<(TextureId, Vec2)>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), frame, fit: FrameFit::Letterbox, status: None }
    }

    pub fn fit(mut self, fit: FrameFit) -> Self {
        self.fit = fit;
        self
    }

    pub fn status(mut self, text: &'a str) -> Self {
        self.status = Some(text);
        self
    }

    // soundcraft: crates/ui-egui/src/panels/video_window.rs:21 `video_status_text`, `:33`
    // `draw_video_surface`.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
