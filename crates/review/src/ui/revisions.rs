//! The revisions panel: a compared copy's changes to accept or reject (N5.10).

use craft_theme::ButtonVariant;
use craft_ui::controls::{Button, Link};
use craft_ui::localized::{Labels as _, Words, fmt};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, corner_radius};
use egui::accesskit::Role;
use egui::{Align, AsIdSalt, Color32, Frame, IdSalt, Layout, Margin, Response, Sense, Ui, Vec2, WidgetInfo};

use super::ReviewStyle;
use super::comments::text;
use crate::revisions::{Revision, RevisionAction, RevisionKind};

/// The detail lines a card shows.
const DETAIL_LINES: usize = 8;

/// The panel's count words, for the facade's `ui::COUNTS`.
pub mod counts {
    use craft_ui::localized::words::Counted;

    pub const ACCEPT_ALL: Counted = Counted::new("revisions.acceptAll", "Accept All ({n})", "Accept All ({n})");
    pub const REJECT_ALL: Counted = Counted::new("revisions.rejectAll", "Reject All ({n})", "Reject All ({n})");

    /// Every count word of the Revisions panel.
    pub const ALL: &[Counted] = &[ACCEPT_ALL, REJECT_ALL];
}

craft_ui::words! {
    /// The panel's words, English by default (DeckCraft's, PowerPoint's).
    #[non_exhaustive]
    pub struct RevisionsLabels {
        compared_with: "revisions.comparedWith" = "Compared with {name}";
        by_position: "revisions.byPosition" = "Not a copy of this presentation: slides are matched by position.";
        /// The English plural of [`counts::ACCEPT_ALL`].
        accept_all: "revisions.acceptAll" = "Accept All ({n})";
        /// The English plural of [`counts::REJECT_ALL`].
        reject_all: "revisions.rejectAll" = "Reject All ({n})";
        inserted: "revisions.inserted" = "Inserted";
        deleted: "revisions.deleted" = "Deleted";
        changed: "revisions.changed" = "Changed";
        /// A detail line of a change, after its bullet.
        detail_line: "revisions.detailLine" = "\u{2022} {detail}";
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
        let mut actions = Vec::new();
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
                        actions.push(RevisionAction::EndReview);
                    }
                    return;
                }
                let count = u64::try_from(revisions.len()).unwrap_or(u64::MAX);
                let words = Words::of(ui);
                let accept_all = words.counted(ui.ctx(), counts::ACCEPT_ALL, &labels.accept_all, count);
                let reject_all = words.counted(ui.ctx(), counts::REJECT_ALL, &labels.reject_all, count);
                ui.horizontal(|ui| {
                    if ui.add(Button::new(accept_all).id_salt(id.with("accept-all"))).clicked() {
                        actions.push(RevisionAction::AcceptAll);
                    }
                    if ui.add(Button::new(reject_all).id_salt(id.with("reject-all"))).clicked() {
                        actions.push(RevisionAction::RejectAll);
                    }
                });
                for (index, revision) in revisions.iter().enumerate() {
                    card(ui, &tokens, &style, &labels, (id, index), revision, &mut actions);
                }
            })
            .response;
        RevisionsPanelOutput { response, actions }
    }
}

/// What the Revisions panel did this pass.
#[must_use]
pub struct RevisionsPanelOutput {
    pub response: Response,
    /// In the order they happened this pass.
    pub actions: Vec<RevisionAction>,
}

/// The colour bar of `kind`.
fn kind_color(style: &ReviewStyle, kind: RevisionKind) -> Color32 {
    match kind {
        RevisionKind::Inserted => style.revision_inserted,
        RevisionKind::Deleted => style.revision_deleted,
        RevisionKind::Changed => style.revision_changed,
    }
}

/// The word of `kind`, the bar's accessible name.
fn kind_word(labels: &RevisionsLabels, kind: RevisionKind) -> &str {
    match kind {
        RevisionKind::Inserted => &labels.inserted,
        RevisionKind::Deleted => &labels.deleted,
        RevisionKind::Changed => &labels.changed,
    }
}

/// One change: bar, heading link, Accept and Reject, then its detail lines.
fn card(
    ui: &mut Ui,
    tokens: &craft_ui::theme::UiTokens,
    style: &ReviewStyle,
    labels: &RevisionsLabels,
    (id, index): (egui::Id, usize),
    revision: &Revision,
    actions: &mut Vec<RevisionAction>,
) {
    let frame = Frame::new()
        .fill(tokens.color.surface.raised.to_egui())
        .corner_radius(corner_radius(tokens.shape.radius.control))
        .inner_margin(Margin::same(tokens.space.pad.control as i8));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = tokens.space.gap.row;
        ui.horizontal(|ui| {
            let (bar, response) = ui.allocate_exact_size(Vec2::new(style.revision_bar_width, style.revision_bar_height), Sense::hover());
            response.widget_info(|| WidgetInfo::labeled(Role::Label, true, kind_word(labels, revision.kind)));
            ui.painter().rect_filled(bar, 0.0, kind_color(style, revision.kind));
            if ui.add(Link::new(&revision.heading).id_salt(id.with(("go", index)))).clicked() {
                actions.push(RevisionAction::Go(index));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(Button::new(&labels.reject).id_salt(id.with(("reject", index)))).clicked() {
                    actions.push(RevisionAction::Reject(index));
                }
                if ui.add(Button::new(&labels.accept).variant(ButtonVariant::Primary).id_salt(id.with(("accept", index)))).clicked() {
                    actions.push(RevisionAction::Accept(index));
                }
            });
        });
        let muted = tokens.color.text.muted.to_egui();
        for detail in revision.details.iter().take(DETAIL_LINES) {
            text(ui, tokens, style.caption_text, muted, fmt(&labels.detail_line, &[("detail", detail)]));
        }
    });
}
