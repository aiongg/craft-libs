//! A decoded frame fitted in a panel, with a status line (N1.11).

use craft_theme::TypeRole;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, CornerRadius, IdSalt, Painter, Rect, Response, Sense, TextureId, Ui, Vec2, WidgetInfo, pos2};

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
    painter.rect_filled(rect, CornerRadius::ZERO, Color32::BLACK);
    let (target, uv) = match fit {
        FrameFit::Letterbox => {
            let valid = size.x.is_finite() && size.y.is_finite() && size.x > 0.0 && size.y > 0.0;
            if !valid {
                return;
            }
            let scale = (rect.width() / size.x).min(rect.height() / size.y);
            (Rect::from_center_size(rect.center(), size * scale), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)))
        }
        FrameFit::Crop(crop) => (rect, crop_uv(crop)),
    };
    painter.image(texture, target, uv, Color32::WHITE);
}

/// The part of the frame a crop leaves visible, as texture coordinates.
fn crop_uv(crop: [f32; 4]) -> Rect {
    let [left, top, right, bottom] = crop.map(|v| if v.is_finite() { v.clamp(0.0, 0.99) } else { 0.0 });
    Rect::from_min_max(pos2(left, top), pos2((1.0 - right).max(left + 0.01), (1.0 - bottom).max(top + 0.01)))
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
        let tokens = Tokens::of(ui);
        let format = role_format(&tokens, TypeRole::Caption, tokens.color.text.muted.to_egui());
        let id = ui.make_persistent_id(self.id_salt);
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let response = ui.interact(rect, id, Sense::hover());
        let status = self.status.filter(|text| !text.is_empty());
        response.widget_info(|| match status {
            Some(text) => WidgetInfo::labeled(Role::Image, ui.is_enabled(), text),
            None => WidgetInfo::new(Role::Image),
        });
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let galley = status.map(|text| {
            let mut job = LayoutJob::simple_singleline(text.to_owned(), format.font_id.clone(), format.color);
            job.wrap.max_width = rect.width();
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        });
        let painter = ui.painter_at(rect);
        match (self.frame, galley) {
            (Some((texture, size)), galley) => {
                let strip = galley.as_ref().map_or(0.0, |galley| galley.size().y + ui.spacing().item_spacing.y);
                let frame_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, (rect.max.y - strip).max(rect.min.y)));
                paint_frame(&painter, frame_rect, texture, size, self.fit);
                if let Some(galley) = galley {
                    let at = pos2(rect.center().x - galley.size().x / 2.0, rect.max.y - galley.size().y);
                    painter.galley(at, galley, format.color);
                }
            }
            (None, Some(galley)) => {
                let at = rect.center() - galley.size() / 2.0;
                painter.galley(at, galley, format.color);
            }
            (None, None) => {}
        }
        response
    }
}
