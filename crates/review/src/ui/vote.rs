//! The voting pill on a board (N5.08).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use crate::collab::CollabTransport;

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
    transport: &'a mut dyn CollabTransport,
    editable: bool,
    labels: Option<VoteLabels>,
}

impl<'a> VotePill<'a> {
    pub fn new(id_salt: impl AsIdSalt, transport: &'a mut dyn CollabTransport) -> Self {
        Self { id_salt: IdSalt::new(id_salt), transport, editable: true, labels: None }
    }

    pub fn editable(mut self, on: bool) -> Self {
        self.editable = on;
        self
    }

    pub fn labels(mut self, labels: VoteLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Calls the transport's `start_vote` and `end_vote`; `true` in the bool when Results was
    /// clicked (the app shows them).
    // figcraft: crates/ui-egui/src/canvas/board_facilitate.rs:153 `vote_part`, `:185` `vote_popup`.
    pub fn show(self, ui: &mut Ui) -> (Response, bool) {
        todo!()
    }
}
