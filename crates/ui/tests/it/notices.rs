//! The notices family: the settings notices, the toast cards and the notice center.

use craft_behavior::PartKey;
use craft_theme::variant::Tone;
use craft_ui::buttons::Button;
use craft_ui::contract::Bind;
use craft_ui::contract::ShowContainer as _;
use craft_ui::contract::{CommandSet, Props as _, Show as _};
use craft_ui::notices::{
    Notice, NoticeAction, NoticeCenter, NoticeLayout, PostedNotice, SaveResult, SettingsLoad, SettingsNoticeState, SettingsStatus, ToastStack,
};
use craft_ui::popover::Popover;
use craft_ui::runtime::commands::FireSource;
use craft_ui::runtime::focus::Initial;
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::runtime::frame::take_output;
use craft_ui::runtime::toast::{self, Toast, ToastAction, ToastEnd};
use craft_ui::testing::input::key_press;
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Id, Key, Modifiers, Ui};
use std::cell::Cell;
use std::rc::Rc;

/// A command a notice action runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "noticetrial")]
#[non_exhaustive]
pub enum NoticeCommand {
    #[command(label = "Open")]
    Open,
}

fn failed() -> SaveResult {
    SaveResult::Failed("disk full".to_owned())
}

/// The first failure speaks in the status line, the third adds the notice, and the next success clears it and says so.
#[trial]
fn repeated_save_failures_escalate_and_a_success_clears_them() {
    let ctx = egui::Context::default();
    let mut state = SettingsNoticeState::new();
    state.ticked(&ctx, &failed());
    assert_eq!(state.take_status(), Some(SettingsStatus::SaveFailed("disk full".to_owned())), "the first failure reaches the status line");
    assert!(!state.has_notice(), "one failure shows no notice");
    state.ticked(&ctx, &failed());
    assert_eq!(state.take_status(), None, "a second failure adds no status");
    state.ticked(&ctx, &failed());
    assert!(state.has_notice(), "the third failure in a row shows the notice");
    state.ticked(&ctx, &SaveResult::Saved);
    assert!(!state.has_notice(), "a success removes the notice");
    let status = state.take_status().expect("a success after failures says so");
    assert_eq!(status.tone(), Tone::Good);
}

/// A success with no failure before it is silent, and stored settings that were reset show their notice until a save.
#[trial]
fn a_plain_success_is_silent_and_a_save_clears_the_reset_notice() {
    let ctx = egui::Context::default();
    let mut state = SettingsNoticeState::new();
    state.loaded(&ctx, &SettingsLoad::new().reset(vec!["tools.brushSize".to_owned()]).moved_aside("prefs.bad"));
    assert!(state.has_notice(), "reset settings show a notice");
    assert_eq!(state.take_status(), Some(SettingsStatus::MovedAside("prefs.bad".to_owned())));
    state.ticked(&ctx, &SaveResult::Saved);
    assert!(!state.has_notice(), "the save wrote the defaults");
    assert_eq!(state.take_status(), None, "no failure came first, so there is nothing to report");
}

fn messages(frames: &Frames) -> Vec<String> {
    toast::cards(&frames.ctx).iter().map(|card| card.toast.message.as_str().to_owned()).collect()
}

/// A fourth card drops the oldest one that has no action; a card with an action outlives older plain ones.
#[trial]
fn the_queue_keeps_three_cards_and_drops_the_oldest_without_an_action() {
    let mut frames = Frames::new();
    frames.idle(2, |ui| ToastStack::show(ui));
    for text in ["a", "b", "c", "d"] {
        toast::post(&frames.ctx, Toast::new(text));
    }
    assert_eq!(messages(&frames), ["b", "c", "d"], "the oldest plain card went");

    let mut frames = Frames::new();
    frames.idle(2, |ui| ToastStack::show(ui));
    toast::post(&frames.ctx, Toast::new("a").action(ToastAction::new("Undo")));
    for text in ["b", "c", "d"] {
        toast::post(&frames.ctx, Toast::new(text));
    }
    assert_eq!(messages(&frames), ["a", "c", "d"], "the card with an action stays over an older plain one");
}

fn host() -> Fixture {
    let mut fixture = Fixture::new(|ui| ToastStack::show(ui));
    fixture.settle();
    fixture
}

/// Clicking a card's action ends the toast with that action's index.
#[trial]
fn clicking_a_toast_action_reports_the_action() {
    let mut fixture = host();
    let id = toast::post(fixture.ctx(), Toast::new("Saved").action(ToastAction::new("Undo")).timeout(None));
    fixture.idle(4);
    let action = fixture.by_label("Undo").expect("the action has a node named by its label");
    fixture.click(&action);
    fixture.idle(2);
    assert_eq!(toast::end(fixture.ctx(), id), Some(ToastEnd::Action(0)));
    assert!(toast::cards(fixture.ctx()).is_empty(), "the card left");
}

/// Escape on a focused card closes it and reports a dismissal.
#[trial]
fn escape_on_a_toast_card_reports_a_dismissal() {
    let mut fixture = host();
    let id = toast::post(fixture.ctx(), Toast::new("Saved").action(ToastAction::new("Undo")).timeout(None));
    fixture.idle(4);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(toast::end(fixture.ctx(), id), Some(ToastEnd::Dismissed));
}

fn open_notice(ui: &mut Ui) -> Id {
    Notice::new(Tone::Info, "Saved").action(NoticeAction::command(NoticeCommand::Open)).id_salt("notice").show(ui).response.id
}

/// An action activated by Enter fires its command with the key as its source.
#[trial]
fn a_notice_action_activated_by_enter_fires_its_command_with_the_key_source() {
    let id = Cell::new(None);
    let mut show = |ui: &mut Ui| id.set(Some(open_notice(ui)));
    let mut frames = Frames::new();
    frames.idle(2, &mut show);
    request(&frames.ctx, FocusTarget::Part(id.get().expect("the notice showed"), PartKey::new("action").at(0)));
    frames.frame(vec![], &mut show);
    let _ = take_output(&frames.ctx);
    frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut show);
    let fired = take_output(&frames.ctx).fired;
    assert_eq!(fired.iter().map(|f| f.command.id.as_str()).collect::<Vec<_>>(), ["noticetrial.open"]);
    assert_eq!(fired[0].source, FireSource::Key);
}

/// Two actions wider than the centred column stack: the second sits below the first.
#[trial]
fn centred_actions_that_do_not_fit_side_by_side_stack() {
    let id = Rc::new(Cell::new(None));
    let shown = id.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = Notice::new(Tone::Neutral, "Nothing here yet.")
            .layout(NoticeLayout::Centred)
            .title("No files")
            .action(NoticeAction::new("Choose files from your computer or a network location"))
            .action(NoticeAction::new("Open a recent project from the list of projects"))
            .id_salt("centred")
            .show(ui);
        shown.set(Some(out.response.id));
    });
    fixture.settle();
    let id = id.get().expect("the notice showed");
    let first = fixture.part(id, PartKey::new("action").at(0)).expect("the first action has a node");
    let second = fixture.part(id, PartKey::new("action").at(1)).expect("the second action has a node");
    assert!(second.rect.top() >= first.rect.bottom(), "the second action sits below the first");
}

/// A notice posted through the center keeps its tone on the card, and one with a dismiss key offers "Don't Show Again".
#[trial]
fn a_posted_notice_keeps_its_tone_and_offers_dont_show_again() {
    let mut frames = Frames::new();
    frames.idle(2, |ui| ToastStack::show(ui));
    let notice = PostedNotice::new("Heads up").lines(vec!["ICO is limited".to_owned()]).tone(Tone::Warn).dismiss_key("warn.ico");
    NoticeCenter::new().post(&frames.ctx, &notice, &[]).expect("not silenced");
    let cards = toast::cards(&frames.ctx);
    assert_eq!(cards[0].toast.tone, Tone::Warn);
    assert_eq!(cards[0].toast.actions.len(), 1, "the dismiss key adds the Don't Show Again action");
    assert!(NoticeCenter::new().post(&frames.ctx, &notice, &["warn.ico".to_owned()]).is_none(), "a silenced key is not posted");
}

/// The action marked default holds the focus when its layer opens with `Initial::Default`, and Enter takes it.
#[trial]
fn the_default_action_of_a_notice_takes_the_focus_of_an_opening_layer() {
    let shown = Rc::new(Cell::new((None, None)));
    let seen = shown.clone();
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Tour").show(ui);
        Popover::new("Tour", Bind::given(true)).opened_by(&opener).initial_focus(Initial::Default).show(ui, |ui, _| {
            let notice = Notice::new(Tone::Neutral, "Step one")
                .layout(NoticeLayout::Inline)
                .action(NoticeAction::new("Skip"))
                .action(NoticeAction::new("Next").default_action())
                .id_salt("tour-notice")
                .show(ui);
            seen.set((Some(notice.response.id), notice.action.or(seen.get().1)));
        });
    });
    fixture.settle();
    let (id, _) = shown.get();
    let id = id.expect("the notice showed");
    assert_eq!(craft_ui::testing::focus::active_part(fixture.ctx(), id), Some(PartKey::new("action").at(1)), "the default action holds focus");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(shown.get().1, Some(1), "Enter takes the default action");
}
