//! Pick from preview: the eyedropper over the Theme workspace's preview.
//!
//! While picking is on, an overlay covers the preview. It takes the clicks, so nothing below it
//! sees them, shows a crosshair, outlines the shape under the pointer, names its token in a
//! tooltip at the pointer, and reports a click on a shape that has a token.
use craft_theme::TokenRef;
use craft_ui::controls::TooltipExt as _;
use craft_ui::theme::{StrokeExt as _, ToEgui, Tokens, TokensOf, scale};
use egui::{Area, Context, CursorIcon, Id, Order, Pos2, Rect, Sense, StrokeKind, Ui, UiKind};

use crate::usage::TileProbe;

/// The tooltip of a colour that no token painted.
pub(super) const BLENDED: &str = "Blended colour (no token)";

/// What picking has in a frame.
#[derive(Default)]
pub(super) struct Picker {
    /// Picking is on.
    pub(super) on: bool,
    /// Counts the pick sessions and clicks: the app's probe is of one value of it.
    pub(super) generation: u32,
    /// The stories the Components preview's tiles drew in this frame, for the pointer to look in.
    pub(super) tiles: Vec<TileProbe>,
    /// The shape the pointer was over in the last frame.
    pub(super) hovered: Option<Hit>,
}

/// The shape under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hit {
    /// Its token or field; `None` for a computed colour.
    pub(crate) token: Option<TokenRef>,
    /// Where it was painted, on screen.
    pub(crate) rect: Rect,
}

impl Picker {
    /// Turns picking on or off. Turning it on probes anew.
    pub(super) fn set(&mut self, on: bool) {
        if on && !self.on {
            self.generation += 1;
        }
        self.on = on;
    }

    /// Whether the Alt key picks the component field instead of the semantic token.
    pub(super) fn field_requested(ctx: &Context) -> bool {
        ctx.input(|i| i.modifiers.alt)
    }

    /// The shape of the tiles drawn this frame at `pos`.
    pub(super) fn tile_hit(&self, pos: Pos2) -> Option<Hit> {
        let tile = self.tiles.iter().rev().find(|t| t.clip.contains(pos))?;
        let used = tile.usage.use_at(pos - tile.origin)?;
        Some(Hit { token: used.token, rect: used.visible().translate(tile.origin) })
    }

    /// Covers `rect` with the overlay. `above` puts it in a layer over everything, such as an
    /// open flyout, which also stops the wheel scrolling what lies below; otherwise it is the
    /// last widget of `ui`. `find` returns the shape at a position. Returns the token that a click
    /// picked.
    pub(super) fn overlay(&mut self, ui: &mut Ui, rect: Rect, above: bool, find: impl Fn(&Self, Pos2) -> Option<Hit>) -> Option<TokenRef> {
        let id = Id::NULL.with("gallery.pick");
        let mut picked = None;
        let mut show = |ui: &mut Ui| {
            let marquee = Tokens::of(ui).color.overlay.marquee.to_egui();
            let response = if above { ui.allocate_rect(rect, Sense::click()) } else { ui.interact(rect, id, Sense::click()) };
            self.hovered = None;
            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                if let Some(pos) = response.hover_pos().or_else(|| response.interact_pointer_pos()) {
                    self.hovered = find(self, pos);
                }
            }
            if let Some(hit) = self.hovered {
                let painter = ui.painter_at(rect);
                painter.rect_filled(hit.rect, 0.0, marquee.gamma_multiply(0.2));
                painter.rect_stroke(hit.rect, 0.0, scale::Stroke::W2.egui(ui.ctx(), marquee), StrokeKind::Outside);
                let text = hit.token.map_or_else(|| BLENDED.to_owned(), TokenRef::path);
                response.clone().tooltip_at_pointer(text);
            }
            if response.clicked() {
                picked = self.hovered.and_then(|h| h.token);
            }
        };
        if above {
            Area::new(id).order(Order::Foreground).fixed_pos(rect.min).kind(UiKind::Modal).sense(Sense::click()).interactable(true).show(
                ui.ctx(),
                |ui| {
                    ui.set_clip_rect(rect);
                    show(ui);
                },
            );
        } else {
            show(ui);
        }
        picked
    }
}
