//! Commands and part activations as a component reads them: `MeasureCx::resolve_command`, `Cx::fire` and
//! `Cx::part_activated`.

use super::kit::{AppCommand, FixedStyle, Pane, PaneCommand};
use craft_behavior::{NavShape, PartKey, Roving, Sense};
use craft_ui::contract::{Common, Component, Cx, Input, MeasureCx, PartsCx, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::runtime::commands::{CommandInput, CommandRef, CommandState, Commands as _, FireSource};
use craft_ui::runtime::focus::{ActivateKeys, KeyUse};
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Response, Ui, Vec2};
use std::cell::RefCell;

/// A leaf that records how it resolves one command in `measure` and what `Cx::fire` returns for another in `update`.
#[derive(Component)]
pub struct Asker<'a> {
    #[common]
    common: Common,
    resolve: Option<CommandRef>,
    fire: Option<(CommandRef, FireSource)>,
    resolved: &'a RefCell<Option<(String, bool)>>,
    fired: &'a RefCell<Option<bool>>,
}

impl<'a> Asker<'a> {
    fn new(resolved: &'a RefCell<Option<(String, bool)>>, fired: &'a RefCell<Option<bool>>) -> Self {
        Asker { common: Common::new(), resolve: None, fire: None, resolved, fired }
    }

    fn resolve(mut self, command: impl Into<CommandRef>) -> Self {
        self.resolve = Some(command.into());
        self
    }

    fn fire(mut self, command: impl Into<CommandRef>, source: FireSource) -> Self {
        self.fire = Some((command.into(), source));
        self
    }
}

impl Component for Asker<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = FixedStyle;
    type Output = Response;

    fn style(_: &craft_ui::style::StyleCx<'_>, _: AskerStyleKey) -> FixedStyle {
        FixedStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        if let Some(command) = &self.resolve {
            *self.resolved.borrow_mut() = cx.resolve_command(command.clone()).map(|resolved| (resolved.label.to_string(), resolved.chord.is_some()));
        }
        Vec2::new(40.0, 20.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some((command, source)) = &self.fire {
            *self.fired.borrow_mut() = Some(cx.fire(command.clone(), *source));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A scoped command resolves with the title its instance published and the chord bound to it; outside any
/// instance of its scope it does not resolve.
#[trial]
fn resolve_command_reads_the_instance_above_the_component() {
    let mut frames = Frames::new();
    let inside = RefCell::new(None);
    let outside = RefCell::new(None);
    let fired = RefCell::new(None);
    let mut show = |ui: &mut Ui| {
        Pane::new().show(ui, |ui, cx| {
            cx.publish(PaneCommand::ZoomIn, CommandState::new().title("Zoom Further"));
            Asker::new(&inside, &fired).resolve(PaneCommand::ZoomIn).show(ui);
        });
        Asker::new(&outside, &fired).id_salt("outside").resolve(PaneCommand::ZoomIn).show(ui);
    };
    frames.idle(4, &mut show);
    assert_eq!(*inside.borrow(), Some(("Zoom Further".to_owned(), true)), "the instance's title and the bound chord");
    assert_eq!(*outside.borrow(), None, "no pane above the component");
}

/// `Cx::fire` queues an enabled command with the source it was given and refuses a disabled one.
#[trial]
fn cx_fire_checks_enabled_and_passes_the_source() {
    let mut frames = Frames::new();
    let resolved = RefCell::new(None);
    let fired = RefCell::new(None);
    let mut show = |ui: &mut Ui| {
        Asker::new(&resolved, &fired).fire(AppCommand::Save, FireSource::Access).show(ui);
    };
    frames.commands(CommandInput::new().state(AppCommand::Save, CommandState::new().enabled(false)));
    frames.idle(3, &mut show);
    assert_eq!(*fired.borrow(), Some(false), "a disabled command is not fired");
    assert!(take_output(&frames.ctx).fired.is_empty(), "nothing ran");

    frames.commands(CommandInput::new().state(AppCommand::Save, CommandState::new()));
    frames.idle(3, &mut show);
    assert_eq!(*fired.borrow(), Some(true));
    let out = take_output(&frames.ctx).fired;
    assert!(!out.is_empty() && out.iter().all(|f| f.command.id == "trial.save" && f.source == FireSource::Access), "the given source: {out:?}");
}

const LEFT: PartKey = PartKey::new("left");
const RIGHT: PartKey = PartKey::new("right");

/// A leaf of two adjoining focusable parts that records `part_activated` for both on every pass.
#[derive(Component)]
pub struct Pair<'a> {
    #[common]
    common: Common,
    seen: &'a RefCell<Vec<(Option<FireSource>, Option<FireSource>)>>,
}

impl Component for Pair<'_> {
    const ROLE: Role = Role::RADIO_GROUP;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_activate(ActivateKeys::Space);
    type Style = FixedStyle;
    type Output = Response;

    fn style(_: &craft_ui::style::StyleCx<'_>, _: PairStyleKey) -> FixedStyle {
        FixedStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(80.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let half = |from: f32| Rect::from_min_size(rect.min + Vec2::new(from, 0.0), Vec2::new(40.0, 20.0));
        cx.part(LEFT, half(0.0)).sense(Sense::CLICK | Sense::FOCUS).focusable().selected();
        cx.part(RIGHT, half(40.0)).sense(Sense::CLICK | Sense::FOCUS).focusable();
        cx.roving(Roving::new(NavShape::Flat));
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.seen.borrow_mut().push((cx.part_activated(LEFT), cx.part_activated(RIGHT)));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Space on the active part is a key activation of that part only; a click is a pointer activation of the part
/// under it only.
#[trial]
fn part_activated_names_the_part_and_the_source() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Vec::new());
    let origin = RefCell::new(Pos2::ZERO);
    let mut show = |ui: &mut Ui| {
        *origin.borrow_mut() = Pair { common: Common::new(), seen: &seen }.show(ui).rect.min;
    };
    frames.idle(3, &mut show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    seen.borrow_mut().clear();
    frames.frame(vec![key_press(Key::Space, Modifiers::NONE)], &mut show);
    frames.frame(vec![], &mut show);
    assert!(seen.borrow().contains(&(Some(FireSource::Key), None)), "Space on the active left part: {:?}", seen.borrow());
    assert!(seen.borrow().iter().all(|(_, right)| right.is_none()), "never the right part: {:?}", seen.borrow());

    seen.borrow_mut().clear();
    let at = *origin.borrow() + Vec2::new(60.0, 10.0);
    click(&mut frames, at, &mut show);
    assert!(seen.borrow().contains(&(None, Some(FireSource::Pointer))), "a click on the right part: {:?}", seen.borrow());
    assert!(seen.borrow().iter().all(|(left, _)| left.is_none()), "never the left part: {:?}", seen.borrow());
}
