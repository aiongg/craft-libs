//! The Color panel in a headless egui context: the wells, the ramp, the panel menu and the focus.

use craft_theme::Srgba;
use egui::accesskit::{Node, NodeId, Role};
use egui::{Context, Event, Key, Modifiers, OutputCommand, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{ColorPanel, ColorPanelMode, ColorPanelRamp, ColorPanelState, PanelTarget};
use craft_ui::dialog::ColorRequest;
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
    opened: Vec<ColorRequest>,
    added: Vec<PanelTarget>,
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
            added: Vec::new(),
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
        self.edits.extend(shown.edits);
        self.opened.extend(shown.pick_color);
        self.added.extend(shown.add_to_swatches);
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

    /// The bounds of the node named `name`, whatever its role, if the panel shows one.
    fn named(&self, name: &str) -> Option<Rect> {
        let node = self.nodes.iter().map(|(_, n)| n).find(|n| n.label() == Some(name))?;
        let b = node.bounds()?;
        Some(Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32)))
    }

    /// Replaces the text of the field named `name` with `text` and commits it with Enter.
    fn type_into(&mut self, name: &str, text: &str) {
        let at = self.named(name).unwrap_or_else(|| panic!("no node named {name:?}")).center();
        self.click(at, Modifiers::NONE);
        let key = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
        self.frame_with(Modifiers::COMMAND, vec![key(Key::A, Modifiers::COMMAND)]);
        self.frame(vec![Event::Text(text.to_owned())]);
        self.frame(vec![key(Key::Enter, Modifiers::NONE)]);
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
    let (front, back) = (rig.bounds(Role::Button, "Foreground color"), rig.bounds(Role::Button, "Background color"));
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
    let request = ColorRequest::new(PanelTarget::Background.key(), rig.colors[1]).title("Color Picker (Background Color)");
    assert_eq!(rig.opened, [request], "a click on the edited well asks for its picker, keyed for the well");
    assert_eq!(PanelTarget::from_key(PanelTarget::Background.key()), Some(PanelTarget::Background));
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
    let mut state = ColorPanelState::default();
    let orange = Srgba::rgb(1.0, 0.5, 0.0);
    let mut entries = Vec::new();
    let frame = |ctx: &Context, run: &mut dyn FnMut(&mut egui::Ui)| {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(320.0, 600.0))), ..Default::default() };
        let output = ctx.run_ui(input, |ui| run(ui));
        let copied: Vec<String> = output
            .platform_output
            .commands
            .iter()
            .filter_map(|c| if let OutputCommand::CopyText(text) = c { Some(text.clone()) } else { None })
            .collect();
        output.drop_without_applying_deltas();
        copied
    };
    frame(&ctx, &mut |ui| entries = ColorPanel::panel_menu(ui.ctx(), "panel", &ColorPanelState::default()));
    let labels: Vec<&str> = entries.iter().filter_map(|e| e.item()).map(|i| i.label()).collect();
    assert_eq!(labels.len(), 9 + 2 + 4 + 1);
    assert_eq!(labels[9..11], ["Copy Color as HTML", "Copy Color's Hex Code"]);
    assert_eq!(labels.last(), Some(&"Make Ramp Web Safe"));
    let command = |label: &str| entries.iter().filter_map(|e| e.item()).find(|i| i.label() == label).and_then(|i| i.command()).cloned().expect("row");
    let (html, hex) = (command("Copy Color as HTML"), command("Copy Color's Hex Code"));
    assert_eq!(frame(&ctx, &mut |ui| assert!(ColorPanel::run_command(ui.ctx(), "panel", &mut state, &html, orange))), ["color=\"#ff8000\""]);
    assert_eq!(frame(&ctx, &mut |ui| assert!(ColorPanel::run_command(ui.ctx(), "panel", &mut state, &hex, orange))), ["ff8000"]);

    let wheel = command("Color Wheel");
    frame(&ctx, &mut |ui| assert!(ColorPanel::run_command(ui.ctx(), "panel", &mut state, &wheel, orange)));
    assert_eq!(state.mode, ColorPanelMode::ColorWheel);
    let safe = command("Make Ramp Web Safe");
    frame(&ctx, &mut |ui| assert!(ColorPanel::run_command(ui.ctx(), "panel", &mut state, &safe, orange)));
    assert!(state.web_safe_ramp);
}

/// The Add to Swatches button under the wheel asks the app to add the edited colour.
#[test]
fn add_to_swatches_reports_the_edited_well() {
    let mut rig = Rig::new(ColorPanelMode::ColorWheel);
    let b = rig.bounds(Role::Button, "Add to Swatches");
    rig.click(b.center(), Modifiers::NONE);
    assert_eq!(rig.added, [PanelTarget::Foreground]);
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

/// Under the cubes the R, G and B fields sit beside the hex field, and an edit of one sets that
/// channel of the edited colour (PhotoCraft's colour readout).
#[test]
fn cube_modes_have_red_green_and_blue_fields() {
    for mode in [ColorPanelMode::HueCube, ColorPanelMode::BrightnessCube] {
        let mut rig = Rig::new(mode);
        for name in ["Hex color", "Red", "Green", "Blue"] {
            assert!(rig.named(name).is_some(), "{mode:?}: no field named {name:?}");
        }
        rig.type_into("Green", "128");
        let picked = rig.edits.iter().rev().find_map(|(t, e)| e.value().map(|v| (*t, v.color))).expect("an edit of the colour");
        assert_eq!(picked.0, PanelTarget::Foreground, "{mode:?}");
        assert_eq!(picked.1.to_rgba().0[..3], [255, 128, 0], "{mode:?}: only green changes");
    }
}

/// The cubes show the out-of-gamut chip, whose click takes the nearest printable colour for the
/// edited well, and no web-colour chip (PhotoCraft #8672).
#[test]
fn cube_modes_warn_when_the_colour_cannot_print() {
    const GAMUT: &str = "Out of gamut for printing: click for the nearest printable color";
    const NOT_WEB: &str = "Not a web color: click for the nearest web color";
    let green = Srgba::rgb(0.0, 1.0, 0.0);
    for mode in [ColorPanelMode::HueCube, ColorPanelMode::BrightnessCube] {
        let mut rig = Rig::new(mode);
        rig.colors = [green, BLUE];
        rig.settle();
        assert!(rig.named(NOT_WEB).is_none(), "{mode:?}: the web-colour chip is for Web Color");
        let chip = rig.named(GAMUT).unwrap_or_else(|| panic!("{mode:?}: pure green can't print")).center();
        rig.click(chip, Modifiers::NONE);
        let picked = rig.edits.iter().find_map(|(t, e)| e.value().map(|v| (*t, v.color))).expect("the nearest colour");
        assert_eq!(picked.0, PanelTarget::Foreground, "{mode:?}");
        assert_ne!(picked.1.to_rgba(), green.to_rgba(), "{mode:?}");

        // A printable colour shows no chip.
        let mut rig = Rig::new(mode);
        rig.colors = [Srgba::rgb(0.5, 0.5, 0.5), BLUE];
        rig.settle();
        assert!(rig.named(GAMUT).is_none(), "{mode:?}");

        // With the background well edited, the click edits the background.
        let mut rig = Rig::new(mode);
        rig.colors = [Srgba::rgb(0.5, 0.5, 0.5), green];
        rig.state.background = true;
        rig.settle();
        let chip = rig.named(GAMUT).expect("the background can't print").center();
        rig.click(chip, Modifiers::NONE);
        assert!(rig.edits.iter().any(|(t, _)| *t == PanelTarget::Background), "{mode:?}: {:?}", rig.edits);
        assert!(rig.edits.iter().all(|(t, _)| *t == PanelTarget::Background), "{mode:?}: the foreground is left alone");
    }
}
