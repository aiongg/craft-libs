//! The Color panel in a headless egui context: the wells, the ramp, the panel menu and the focus.

use craft_theme::Srgba;
use egui::accesskit::{Node, NodeId, Role};
use egui::{Context, Event, Modifiers, OutputCommand, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{ColorPanel, ColorPanelMode, ColorPanelRamp, ColorPanelState, PanelTarget};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

struct Rig {
    ctx: Context,
    time: f64,
    state: ColorPanelState,
    colors: [Srgba; 2],
    edits: Vec<(PanelTarget, EditEvent<super::PickedColor>)>,
    opened: Vec<PanelTarget>,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new(mode: ColorPanelMode) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            state: ColorPanelState { mode, ..Default::default() },
            colors: [RED, BLUE],
            edits: Vec::new(),
            opened: Vec::new(),
            nodes: Vec::new(),
        };
        rig.settle();
        rig
    }

    fn frame_with(&mut self, modifiers: Modifiers, events: Vec<Event>) -> Vec<OutputCommand> {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(320.0, 600.0))),
            time: Some(self.time),
            events: std::iter::once(Event::ModifiersChanged(modifiers)).chain(events).collect(),
            ..Default::default()
        };
        self.time += 1.0 / 60.0;
        let numbers = LocaleNumbers::from_tag("en");
        let ctx = self.ctx.clone();
        let mut shown = None;
        let output = ctx.run_ui(input, |ui| {
            shown = Some(ColorPanel::new("panel", &mut self.state, self.colors, &numbers).show(ui));
        });
        let shown = shown.expect("shown");
        self.edits.extend(shown.edit);
        self.opened.extend(shown.open_picker);
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        let commands = output.platform_output.commands.clone();
        output.drop_without_applying_deltas();
        commands
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.frame_with(Modifiers::NONE, events);
    }

    fn settle(&mut self) {
        for _ in 0..4 {
            self.frame(vec![]);
        }
    }

    fn click(&mut self, at: Pos2, modifiers: Modifiers) {
        // Apart from an earlier click, which would make the press a double one.
        self.time += 1.0;
        self.frame_with(modifiers, vec![Event::PointerMoved(at), button(at, true, modifiers)]);
        self.frame_with(modifiers, vec![button(at, false, modifiers)]);
        self.settle();
    }

    fn bounds(&self, role: Role, name: &str) -> Rect {
        let node = self
            .nodes
            .iter()
            .map(|(_, n)| n)
            .find(|n| n.role() == role && n.label() == Some(name))
            .unwrap_or_else(|| panic!("no {role:?} named {name:?}"));
        let b = node.bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

/// A point of the background well outside the foreground one.
fn background_only(rig: &Rig) -> Pos2 {
    let (front, back) = (rig.bounds(Role::Button, "Foreground colour"), rig.bounds(Role::Button, "Background colour"));
    let corner = back.right_bottom() - vec2(3.0, 3.0);
    assert!(!front.contains(corner));
    corner
}

/// A click on the other well makes it the edited one; a click on the edited one asks for the picker.
#[test]
fn wells_switch_and_open_the_picker() {
    let mut rig = Rig::new(ColorPanelMode::RgbSliders);
    assert!(!rig.state.background);
    let back = background_only(&rig);
    rig.click(back, Modifiers::NONE);
    assert!(rig.state.background, "a click on the other well makes it the edited one");
    assert!(rig.opened.is_empty(), "and opens nothing");

    rig.click(back, Modifiers::NONE);
    assert_eq!(rig.opened, [PanelTarget::Background], "a click on the edited well asks for its picker");
    assert!(rig.state.background);
}

/// Alt on the ramp picks for the colour not edited; Shift-click cycles the ramp.
#[test]
fn ramp_alt_targets_the_other_and_shift_cycles() {
    let mut rig = Rig::new(ColorPanelMode::RgbSliders);
    let bar = rig.bounds(Role::Group, "Color Ramp");
    let at = pos2(bar.left() + bar.width() * 0.25, bar.center().y);

    rig.click(at, Modifiers::NONE);
    let picks = std::mem::take(&mut rig.edits);
    assert!(!picks.is_empty() && picks.iter().all(|(t, _)| *t == PanelTarget::Foreground), "a plain press picks for the edited colour: {picks:?}");
    let picked = picks.iter().find_map(|(_, e)| e.value()).expect("a picked colour");
    let c = picked.color;
    assert!(c.g > c.r && c.r > c.b, "a quarter across is hue 90, yellow-green: {c:?}");

    rig.click(at, Modifiers::ALT);
    let picks = std::mem::take(&mut rig.edits);
    assert!(!picks.is_empty() && picks.iter().all(|(t, _)| *t == PanelTarget::Background), "Alt picks for the other colour: {picks:?}");

    assert_eq!(rig.state.ramp, ColorPanelRamp::RgbSpectrum);
    rig.click(at, Modifiers::SHIFT);
    assert_eq!(rig.state.ramp, ColorPanelRamp::CmykSpectrum, "Shift-click cycles the ramp");
    assert!(rig.edits.is_empty(), "and picks nothing: {:?}", rig.edits);
}

/// Copy Color as HTML and Copy Color's Hex Code put color="#rrggbb" and rrggbb on the clipboard.
#[test]
fn copy_entries_use_photoshops_formats() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let mut state = ColorPanelState::default();
    let orange = Srgba::rgb(1.0, 0.5, 0.0);
    let mut run = |events: Vec<Event>, ctx: &Context| {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(320.0, 600.0))), events, ..Default::default() };
        let output = ctx.run_ui(input, |ui| ColorPanel::panel_menu(ui, &mut state, orange));
        let nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        let copied: Vec<String> = output
            .platform_output
            .commands
            .iter()
            .filter_map(|c| if let OutputCommand::CopyText(text) = c { Some(text.clone()) } else { None })
            .collect();
        output.drop_without_applying_deltas();
        (nodes, copied)
    };
    run(vec![], &ctx);
    let (nodes, _) = run(vec![], &ctx);
    let centre = |name: &str| {
        let node = nodes.iter().map(|(_, n)| n).find(|n| n.label() == Some(name)).unwrap_or_else(|| panic!("no row {name:?}"));
        let b = node.bounds().expect("bounds");
        pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
    };
    let mut copy = |name: &str| {
        let at = centre(name);
        run(vec![Event::PointerMoved(at), button(at, true, Modifiers::NONE)], &ctx);
        let (_, copied) = run(vec![button(at, false, Modifiers::NONE)], &ctx);
        copied
    };
    assert_eq!(copy("Copy Color as HTML"), ["color=\"#ff8000\""]);
    assert_eq!(copy("Copy Color's Hex Code"), ["ff8000"]);
}

/// The panel never takes the focus when shown.
#[test]
fn showing_the_panel_keeps_the_focus() {
    for mode in ColorPanelMode::ALL {
        let mut rig = Rig::new(mode);
        rig.frame(vec![]);
        assert_eq!(rig.ctx.memory(|m| m.focused()), None, "{mode:?} takes no focus");
    }
}
