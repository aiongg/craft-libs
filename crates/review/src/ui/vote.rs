//! The voting pill on a board (N5.08).

use craft_theme::ButtonVariant;
use craft_ui::controls::{Button, NumberField, Select};
use craft_ui::localized::{Labels as _, LocaleNumbers, fmt};
use craft_ui::popover::{Popover, PopoverAnchor};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::comments::text;
use super::style::ReviewStyle;
use crate::collab::{CollabRequest, CollabTransport, VoteScope};

/// Votes per person a new vote starts with.
const DEFAULT_VOTES: u32 = 3;
/// Most votes per person (FigCraft's field takes 1 to 99, `board_facilitate.rs:208`).
const MAX_VOTES: u32 = 99;

craft_ui::words! {
    /// The vote's words, English by default (FigCraft's).
    #[non_exhaustive]
    pub struct VoteLabels {
        voting_session: "vote.session" = "Voting session";
        votes_per_person: "vote.perPerson" = "Votes per person";
        vote_on: "vote.on" = "Vote on";
        all: "vote.all" = "All objects";
        stickies: "vote.stickies" = "Sticky notes";
        selection: "vote.selection" = "Selection";
        start: "vote.start" = "Start vote";
        votes_left: "vote.left" = "{n} votes left";
        hint: "vote.hint" = "Click objects to vote; click your badge to take a vote back";
        end: "vote.end" = "End vote";
        results: "vote.results" = "Results";
    }
}

/// The vote part of a board's top-right pill (FigCraft): with no vote running, the Voting
/// session button (a popover: votes per person, default 3; Vote on All objects / Sticky notes /
/// Selection; Start vote) and Results when earlier results are kept; while running, "n votes
/// left" (the hint as tooltip) and End vote. Editors only start and end votes. Buttons are Tab
/// stops; the popover's fields too, Enter starts.
pub struct VotePill<'a> {
    id_salt: IdSalt,
    transport: &'a dyn CollabTransport,
    editable: bool,
    has_selection: bool,
    labels: Option<VoteLabels>,
}

impl<'a> VotePill<'a> {
    pub fn new(id_salt: impl AsIdSalt, transport: &'a dyn CollabTransport) -> Self {
        Self { id_salt: IdSalt::new(id_salt), transport, editable: true, has_selection: false, labels: None }
    }

    pub fn editable(mut self, on: bool) -> Self {
        self.editable = on;
        self
    }

    /// Whether something is selected on the board. Vote on offers Selection only then; it is
    /// off by default.
    pub fn has_selection(mut self, on: bool) -> Self {
        self.has_selection = on;
        self
    }

    pub fn labels(mut self, labels: VoteLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Start vote, End vote and Results come back as [`CollabRequest`]s.
    pub fn show(self, ui: &mut Ui) -> VotePillOutput {
        let Self { id_salt, transport, editable, has_selection, labels } = self;
        let labels = labels.unwrap_or_else(|| VoteLabels::of(ui));
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let vote = transport.vote();
        let mut requests = Vec::new();
        let response = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = tokens.space.gap.inline;
                match vote.as_ref().filter(|vote| !vote.ended) {
                    Some(running) => {
                        let left = text(
                            ui,
                            &tokens,
                            ReviewStyle::of(&tokens).body_text,
                            tokens.color.text.normal.to_egui(),
                            fmt(&labels.votes_left, &[("n", &running.left().to_string())]),
                        );
                        left.on_hover_text(&labels.hint);
                        if editable && ui.add(Button::new(&labels.end).id_salt(id.with("end"))).clicked() {
                            requests.push(CollabRequest::EndVote);
                        }
                    }
                    None => {
                        if editable {
                            let button = ui.add(Button::new(&labels.voting_session).id_salt(id.with("session")));
                            if let Some(request) = start_popover(ui, id, &button, &labels, has_selection) {
                                requests.push(request);
                            }
                        }
                        if vote.as_ref().is_some_and(|vote| vote.has_history || vote.ended)
                            && ui.add(Button::new(&labels.results).id_salt(id.with("results"))).clicked()
                        {
                            requests.push(CollabRequest::ShowResults);
                        }
                    }
                }
            })
            .response;
        VotePillOutput { response, requests }
    }
}

/// What the vote pill did this pass.
#[must_use]
pub struct VotePillOutput {
    pub response: Response,
    /// Start vote, End vote or Results, in order.
    pub requests: Vec<CollabRequest>,
}

/// What the Voting session popover remembers between passes.
#[derive(Clone, Copy)]
struct Draft {
    open: bool,
    per_person: u32,
    scope: VoteScope,
}

/// The popover under the Voting session button: votes per person, what to vote on, Start vote.
/// The request when Start vote (or Enter in the number field) ran.
fn start_popover(ui: &mut Ui, id: egui::Id, button: &Response, labels: &VoteLabels, has_selection: bool) -> Option<CollabRequest> {
    let tokens = Tokens::of(ui);
    let state_id = id.with("draft");
    let mut draft = ui.data(|data| data.get_temp(state_id)).unwrap_or(Draft { open: false, per_person: DEFAULT_VOTES, scope: VoteScope::All });
    if button.clicked() {
        draft.open = !draft.open;
    }
    let numbers = LocaleNumbers::of(ui);
    let mut start = false;
    let mut open = draft.open;
    Popover::new(id.with("popover"), PopoverAnchor::Control(button.clone())).toggle(button.id).label(&labels.voting_session).show(
        ui,
        &mut open,
        |ui, pop| {
            ui.spacing_mut().item_spacing.y = tokens.space.gap.row;
            text(ui, &tokens, ReviewStyle::of(&tokens).strong_text, tokens.color.text.normal.to_egui(), &labels.voting_session);
            text(ui, &tokens, ReviewStyle::of(&tokens).label_text, tokens.color.text.muted.to_egui(), &labels.votes_per_person);
            let votes = NumberField::new(id.with("votes"), &labels.votes_per_person, draft.per_person, &numbers)
                .range(1..=MAX_VOTES)
                .step(1.0)
                .decimals(0..=0)
                .show(ui);
            pop.initial_focus(votes.response.id);
            if let Some(value) = votes.event.as_ref().and_then(|event| event.value()) {
                draft.per_person = value.round().clamp(1.0, f64::from(MAX_VOTES)) as u32;
            }
            text(ui, &tokens, ReviewStyle::of(&tokens).label_text, tokens.color.text.muted.to_egui(), &labels.vote_on);
            // The selection may have gone since the scope was picked: the vote then covers all.
            if !has_selection && draft.scope == VoteScope::Selection {
                draft.scope = VoteScope::All;
            }
            let select = Select::new(id.with("scope"), &labels.vote_on, &mut draft.scope)
                .option(VoteScope::All, &labels.all)
                .option(VoteScope::Stickies, &labels.stickies);
            let _ = if has_selection { select.option(VoteScope::Selection, &labels.selection) } else { select }.show(ui);
            let go = ui.add(Button::new(&labels.start).variant(ButtonVariant::Primary).id_salt(id.with("start"))).clicked();
            start = go || votes.submitted;
        },
    );
    draft.open = open;
    let request = start.then_some(CollabRequest::StartVote { per_person: draft.per_person, scope: draft.scope });
    if start {
        draft.open = false;
    }
    ui.data_mut(|data| data.insert_temp(state_id, draft));
    request
}
