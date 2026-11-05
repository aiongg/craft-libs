//! The revisions panel: a compared copy's changes to accept or reject (N5.10).

use craft_theme::ButtonVariant;
use craft_ui::controls::{Button, Link};
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, corner_radius};
use egui::{Align, AsIdSalt, Color32, Frame, IdSalt, Layout, Margin, Response, Sense, Ui, Vec2};

use super::ReviewStyle;
use super::comments::text;
use crate::revisions::{Revision, RevisionAction, RevisionKind};

/// The detail lines a card shows.
const DETAIL_LINES: usize = 8;

craft_ui::words! {
    /// The panel's words, English by default (DeckCraft's, PowerPoint's).
    #[non_exhaustive]
    pub struct RevisionsLabels {
        compared_with: "revisions.comparedWith" = "Compared with {name}";
        by_position: "revisions.byPosition" = "Not a copy of this presentation: slides are matched by position.";
        accept_all: "revisions.acceptAll" = "Accept All ({n})";
        reject_all: "revisions.rejectAll" = "Reject All ({n})";
        accept: "revisions.accept" = "Accept";
        reject: "revisions.reject" = "Reject";
        none_left: "revisions.noneLeft" = "No differences left.";
        end_review: "revisions.endReview" = "End Review";
    }
}

/// PowerPoint's Compare pane (DeckCraft): "Compared with {name}" (and the by-position note),
/// Accept All (n) and Reject All (n), then a card per change: a colour bar
/// (`review.revision_bar_*` in `review.revision_inserted`, `_deleted` or `_changed` of
/// [`ReviewStyle`](super::ReviewStyle)), the heading as a [`Link`](craft_ui::controls::Link)
/// (goes there), Accept and Reject at the right, up to eight detail lines with bullets. With no
/// changes: "No differences left." and End Review. Every link and button is a Tab stop in reading
/// order; counts through [`Counted`](craft_ui::localized::words::Counted) words.
pub struct RevisionsPanel<'a> {
    id_salt: IdSalt,
    compared_with: &'a str,
    by_position: bool,
    revisions: &'a [Revision],
    labels: Option<RevisionsLabels>,
}

impl<'a> RevisionsPanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, compared_with: &'a str, revisions: &'a [Revision]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), compared_with, by_position: false, revisions, labels: None }
    }

    /// The copy didn't match by identity; changes are by position.
    pub fn by_position(mut self, on: bool) -> Self {
        self.by_position = on;
        self
    }

    pub fn labels(mut self, labels: RevisionsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // deckcraft: crates/ui-egui/src/panels/comments.rs:5 `revisions`.
    pub fn show(self, ui: &mut Ui) -> RevisionsPanelOutput {
        let Self { id_salt, compared_with, by_position, revisions, labels } = self;
        let labels = labels.unwrap_or_else(|| RevisionsLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let muted = tokens.color.text.muted.to_egui();
        let mut action = None;
        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = tokens.space.gap.stack;
                text(ui, &tokens, style.body_text, muted, fmt(&labels.compared_with, &[("name", compared_with)]));
                if by_position {
                    text(ui, &tokens, style.caption_text, muted, &labels.by_position);
                }
                if revisions.is_empty() {
                    text(ui, &tokens, style.body_text, tokens.color.text.normal.to_egui(), &labels.none_left);
                    if ui.add(Button::new(&labels.end_review).id_salt(id.with("end"))).clicked() {
                        action = Some(RevisionAction::EndReview);
                    }
                    return;
                }
                let count = revisions.len().to_string();
                ui.horizontal(|ui| {
                    if ui.add(Button::new(fmt(&labels.accept_all, &[("n", &count)])).id_salt(id.with("accept-all"))).clicked() {
                        action = Some(RevisionAction::AcceptAll);
                    }
                    if ui.add(Button::new(fmt(&labels.reject_all, &[("n", &count)])).id_salt(id.with("reject-all"))).clicked() {
                        action = Some(RevisionAction::RejectAll);
                    }
                });
                for (index, revision) in revisions.iter().enumerate() {
                    if let Some(chosen) = card(ui, &tokens, &style, &labels, id, index, revision) {
                        action = Some(chosen);
                    }
                }
            })
            .response;
        RevisionsPanelOutput { response, action }
    }
}

/// What the Revisions panel did this pass.
#[must_use]
pub struct RevisionsPanelOutput {
    pub response: Response,
    pub action: Option<RevisionAction>,
}

/// The colour bar of `kind`.
fn kind_color(style: &ReviewStyle, kind: RevisionKind) -> Color32 {
    match kind {
        RevisionKind::Inserted => style.revision_inserted,
        RevisionKind::Deleted => style.revision_deleted,
        RevisionKind::Changed => style.revision_changed,
    }
}

/// One change: bar, heading link, Accept and Reject, then its detail lines.
fn card(
    ui: &mut Ui,
    tokens: &craft_ui::theme::UiTokens,
    style: &ReviewStyle,
    labels: &RevisionsLabels,
    id: egui::Id,
    index: usize,
    revision: &Revision,
) -> Option<RevisionAction> {
    let mut action = None;
    let frame = Frame::new()
        .fill(tokens.color.surface.raised.to_egui())
        .corner_radius(corner_radius(tokens.shape.radius.control))
        .inner_margin(Margin::same(tokens.space.pad.control as i8));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = tokens.space.gap.row;
        ui.horizontal(|ui| {
            let (bar, _) = ui.allocate_exact_size(Vec2::new(style.revision_bar_width, style.revision_bar_height), Sense::hover());
            ui.painter().rect_filled(bar, 0.0, kind_color(style, revision.kind));
            if ui.add(Link::new(&revision.heading).id_salt(id.with(("go", index)))).clicked() {
                action = Some(RevisionAction::Go(index));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(Button::new(&labels.reject).id_salt(id.with(("reject", index)))).clicked() {
                    action = Some(RevisionAction::Reject(index));
                }
                if ui.add(Button::new(&labels.accept).variant(ButtonVariant::Primary).id_salt(id.with(("accept", index)))).clicked() {
                    action = Some(RevisionAction::Accept(index));
                }
            });
        });
        let muted = tokens.color.text.muted.to_egui();
        for detail in revision.details.iter().take(DETAIL_LINES) {
            text(ui, tokens, style.caption_text, muted, format!("\u{2022} {detail}"));
        }
    });
    action
}
