//! Command scopes of a layout panel and of a focused leaf, and the defaults of a command that differ by platform.

use super::kit::FixedStyle;
use craft_behavior::Sense;
use craft_host::target::Os;
use craft_ui::buttons::Button;
use craft_ui::contract::{CommandSet, Common, Component, Cx, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::layout_components::Stack;
use craft_ui::runtime::commands::{CommandScope, Commands as _, resolve};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::style::StyleCx;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Response, Vec2};
use std::cell::Cell;
use std::rc::Rc;

/// The commands of the panel scope. `FindNext` is F3 everywhere but on macOS, where it is Cmd+G.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "panel", scope = "trial.panel")]
#[non_exhaustive]
pub enum PanelCommand {
    #[command(label = "Toggle", default = "Cmd+Alt+P")]
    Toggle,
    #[command(label = "Find Next", default = "F3", default(mac = "Cmd+G"))]
    FindNext,
}

const PANEL: CommandScope = <PanelCommand as CommandSet>::SCOPE;

fn toggle(fixture: &mut Fixture) {
    fixture.key(Key::P, Modifiers::COMMAND | Modifiers::ALT);
    fixture.settle();
}

/// A focusable leaf that counts the panel commands it takes.
#[derive(Component)]
struct Stop {
    #[common]
    common: Common,
    taken: Rc<Cell<u32>>,
}

impl Stop {
    fn new(taken: &Rc<Cell<u32>>) -> Self {
        Stop { common: Common::new(), taken: Rc::clone(taken) }
    }
}

impl Component for Stop {
    const ROLE: Role = Role::BUTTON;
    type Style = FixedStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: StopStyleKey) -> FixedStyle {
        FixedStyle::default()
    }

    fn measure(&mut self, _: &craft_ui::contract::MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        while cx.take_command::<PanelCommand>().is_some() {
            self.taken.set(self.taken.get() + 1);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Two instances of one command scope, as stacks and as focusable leaves: the chord reaches the one that holds focus,
/// and not the other.
#[trial]
fn a_scope_command_reaches_only_the_instance_that_holds_focus() {
    let counts = [Rc::new(Cell::new(0)), Rc::new(Cell::new(0))];
    let ids = [Rc::new(Cell::new(None)), Rc::new(Cell::new(None))];
    let (seen, stops) = (counts.clone(), ids.clone());
    let mut fixture = Fixture::new(move |ui| {
        for (at, name) in ["First", "Second"].into_iter().enumerate() {
            let taken = &seen[at];
            Stack::new().command_scope(PANEL).id_salt(name).show(ui, |ui, cx| {
                while cx.take_command::<PanelCommand>().is_some() {
                    taken.set(taken.get() + 1);
                }
                stops[at].set(Some(Button::new(name).show(ui).id));
            });
        }
    });
    fixture.settle();
    for (focused, expected) in [(1, (0, 1)), (0, (1, 1))] {
        request(fixture.ctx(), FocusTarget::Id(ids[focused].get().expect("the button ran")));
        fixture.settle();
        toggle(&mut fixture);
        assert_eq!((counts[0].get(), counts[1].get()), expected, "only the stack holding focus ({focused}) takes the command");
    }

    let counts = [Rc::new(Cell::new(0)), Rc::new(Cell::new(0))];
    let ids = [Rc::new(Cell::new(None)), Rc::new(Cell::new(None))];
    let (seen, stops) = (counts.clone(), ids.clone());
    let mut fixture = Fixture::new(move |ui| {
        for (at, name) in ["one", "two"].into_iter().enumerate() {
            stops[at].set(Some(Stop::new(&seen[at]).command_scope(PANEL).id_salt(name).label(name).show(ui).id));
        }
    });
    fixture.settle();
    for (focused, expected) in [(1, (0, 1)), (0, (1, 1))] {
        request(fixture.ctx(), FocusTarget::Id(ids[focused].get().expect("the leaf ran")));
        fixture.settle();
        toggle(&mut fixture);
        assert_eq!((counts[0].get(), counts[1].get()), expected, "only the focused leaf ({focused}) takes the command");
    }
}

/// A platform default replaces the others on its platform.
#[trial]
fn a_platform_default_replaces_the_default_on_its_platform() {
    let mut fixture = Fixture::new(|_| {});
    let key = |fixture: &mut Fixture, os: Os| {
        fixture.os(os).frame(vec![]);
        resolve(fixture.ctx(), PanelCommand::FindNext).chord.map(|chord| chord.key)
    };
    let (mac, windows, linux) = (key(&mut fixture, Os::Mac), key(&mut fixture, Os::Windows), key(&mut fixture, Os::Linux));
    assert!(mac.is_some() && windows.is_some(), "every platform has a chord");
    assert_ne!(mac, windows, "the macOS default differs from the others");
    assert_eq!(windows, linux, "the other platforms share the default");
}
