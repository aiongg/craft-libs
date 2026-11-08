//! Stories of the people bar.

use super::PeopleBar;
use crate::collab::{CollabTransport, JoinState, PEOPLE_COLORS, Participant, ParticipantId, VoteSession};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

/// A session of `count` people, you the first.
pub(crate) struct Room(pub Vec<Participant>);

impl Room {
    pub(crate) fn of(names: &[&str]) -> Self {
        let person = |(at, name): (usize, &&str)| Participant {
            id: ParticipantId(at as u64 + 1),
            name: (*name).into(),
            color: PEOPLE_COLORS[at % PEOPLE_COLORS.len()],
            cursor: None,
            selection: Vec::new(),
            chat: None,
            viewer: false,
            spotlight: false,
        };
        Room(names.iter().enumerate().map(person).collect())
    }
}

impl CollabTransport for Room {
    fn me(&self) -> ParticipantId {
        ParticipantId(1)
    }

    fn participants(&self) -> Vec<Participant> {
        self.0.clone()
    }

    fn vote(&self) -> Option<VoteSession> {
        None
    }

    fn join_state(&self) -> JoinState {
        JoinState::Joined
    }
}

#[story(PeopleBar<'static>, covers = [Default, Many], drive = Drive::Never)]
fn people_bar_seven(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let room = Room::of(&["Ana Reyes", "Bo Chen", "Cy Dahl", "Di Evans", "Ed Fox", "Flo Gray", "Gus Hall"]);
    PeopleBar::new(&room).following(ParticipantId(3)).show(ui).into()
}

#[story(PeopleBar<'static>, covers = [One], drive = Drive::Never)]
fn people_bar_alone(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    PeopleBar::new(&Room::of(&["Ana Reyes"])).show(ui).into()
}
