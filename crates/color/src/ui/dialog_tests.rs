//! The Full picker in a real dialog window, as PhotoCraft's Color Picker: its size against
//! Photoshop's window, the warnings beside the swatch, the strip's room, and the hex field's
//! focus on open.

use craft_theme::Srgba;
use egui::accesskit::{Node, NodeId, Role};
use egui::{Context, Id, Pos2, RawInput, Rect, pos2, vec2};

use super::{ColorArea, ColorPicker, ColorPickerLayout, ColorStrip, ColorWarning};
use craft_ui::dialog::{ButtonRole, Dialog, DialogState, Footer, FooterButton, InitialFocus, dialog_id};
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, Tokens, TokensOf, install};

/// How the picker's body is hosted.
#[derive(Clone, Copy, PartialEq)]
enum Host {
    /// A [`Dialog`] with this initial focus.
    Dialog(InitialFocus),
    /// A plain egui window.
    Window,
}

fn window_id() -> Id {
    Id::unique(("color-tests", "window"))
}

/// A Full picker with a compare swatch, warnings and side actions, in a window.
struct Shown {
    ctx: Context,
    numbers: LocaleNumbers,
    time: f64,
    host: Host,
    state: DialogState,
    picker: Id,
    window: Rect,
    picker_rect: Rect,
    nodes: Vec<(NodeId, Node)>,
}

impl Shown {
    fn new(host: Host) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut shown = Self {
            ctx,
            numbers: LocaleNumbers::from_tag("en"),
            time: 0.0,
            host,
            state: DialogState::new(),
            picker: Id::NULL,
            window: Rect::NOTHING,
            picker_rect: Rect::NOTHING,
            nodes: Vec::new(),
        };
        for _ in 0..6 {
            shown.frame();
        }
        shown
    }

    fn frame(&mut self) {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))), time: Some(self.time), ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let (host, numbers) = (self.host, &self.numbers);
        let picked = std::cell::Cell::new((Id::NULL, Rect::NOTHING));
        let body = |ui: &mut egui::Ui| {
            let color = Srgba::rgb(0.2, 0.4, 0.6);
            let shown = ColorPicker::new("picker", "Fill colour", color, numbers)
                .layout(ColorPickerLayout::Full)
                .compare(color)
                .warning(ColorWarning::new("Out of gamut", Srgba::rgb(0.2, 0.4, 0.5)))
                .warning(ColorWarning::web_colors("Not a web colour", Srgba::rgb(0.2, 0.4, 0.6)))
                .web_colors(&mut false)
                .eyedropper("Pick screen color")
                .side_actions([
                    FooterButton::new(ButtonRole::Default, "OK"),
                    FooterButton::new(ButtonRole::Cancel, "Cancel"),
                    FooterButton::new(ButtonRole::Alternate, "Add to Swatches"),
                ])
                .show(ui);
            picked.set((shown.edit.response.id, shown.edit.response.rect));
        };
        let output = ctx.run_ui(input, |ui| match host {
            Host::Dialog(focus) => {
                let ctx = ui.ctx().clone();
                Dialog::new("picker", "Color Picker")
                    .initial_focus(focus)
                    .footer(Footer::none())
                    .show(&ctx, &mut self.state, &mut (), |ui, ()| body(ui));
            }
            Host::Window => {
                egui::Window::new("Color Picker").id(window_id()).show(ui.ctx(), |ui| body(ui));
            }
        });
        (self.picker, self.picker_rect) = picked.get();
        self.window =
            self.ctx.memory(|m| m.area_rect(if matches!(host, Host::Window) { window_id() } else { dialog_id("picker") })).unwrap_or(Rect::NOTHING);
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
    }

    fn rect(&self, role: Role, name: &str) -> Rect {
        let node = self.nodes.iter().map(|(_, n)| n).find(|n| n.role() == role && n.label() == Some(name)).unwrap_or_else(|| panic!("no {name}"));
        let b = node.bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }

    fn hex_focused(&self) -> bool {
        self.ctx.memory(|m| m.focused()) == Some(super::hex::hex_id(self.picker))
    }
}

#[test]
fn the_picker_with_its_actions_fits_photoshops_window() {
    let shown = Shown::new(Host::Dialog(InitialFocus::None));
    // Photoshop's Color Picker is 534 x 376 points; PhotoCraft's test allows 384 high.
    assert!(
        shown.window.width() <= 534.0 && shown.window.height() <= 384.0,
        "Color Picker window {} x {} points",
        shown.window.width(),
        shown.window.height()
    );
    // The actions are in the picker's width, above the fields and beside the swatch.
    let (ok, add) = (shown.rect(Role::Button, "OK"), shown.rect(Role::Button, "Add to Swatches"));
    let hue = shown.rect(Role::SpinButton, "Hue");
    let new = shown.rect(Role::Image, "New colour");
    assert!(shown.picker_rect.contains_rect(add), "{add:?} in {:?}", shown.picker_rect);
    assert!(ok.left() > new.right() && add.bottom() < hue.top(), "{ok:?}, {add:?}, {hue:?}");
    assert!((ok.min.x - add.min.x).abs() < 0.5 && ok.width() == add.width(), "one column, one width");
    assert!(ok.right() >= shown.rect(Role::TextInput, "Hex colour").right(), "at the trailing edge of the grid");
}

#[test]
fn warnings_sit_beside_the_swatch_level_with_its_top() {
    let shown = Shown::new(Host::Dialog(InitialFocus::None));
    let new = shown.rect(Role::Image, "New colour");
    let (gamut, web) = (shown.rect(Role::Button, "Out of gamut"), shown.rect(Role::Button, "Not a web colour"));
    assert!(gamut.left() >= new.right() && web.left() >= new.right(), "beside the swatch: {gamut:?} {web:?} {new:?}");
    assert!((gamut.top() - (new.top() + 2.0)).abs() < 1.5, "level with the swatch's top: {gamut:?} {new:?}");
    assert!(web.top() > gamut.bottom(), "the second below the first");
}

#[test]
fn focus_on_open_reaches_the_hex_field_in_a_dialog_and_a_window() {
    for host in [Host::Dialog(InitialFocus::None), Host::Dialog(InitialFocus::DefaultButton), Host::Window] {
        assert!(Shown::new(host).hex_focused(), "the picker's hex field takes the focus when its window opens");
    }
}

#[test]
fn the_strips_pointer_room_is_its_own_not_the_areas() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let numbers = LocaleNumbers::from_tag("en");
    let run = || {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 600.0))), time: Some(0.0), ..Default::default() };
        let mut widths = None;
        let output = ctx.run_ui(input, |ui| {
            let tokens = Tokens::of(ui);
            let style = ColorArea::style(&tokens);
            ui.allocate_ui(vec2(300.0, 0.0), |ui| {
                let _ = ColorPicker::new("picker", "Fill colour", Srgba::rgb(0.2, 0.4, 0.6), &numbers).layout(ColorPickerLayout::Compact).show(ui);
            });
            widths = Some((style, craft_ui::primitives::Gap::Inline.points(&tokens)));
        });
        let nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
        (nodes, widths.expect("style"))
    };
    run();
    let (nodes, (style, gap)) = run();
    let find = |name: &str| {
        let node = nodes.iter().map(|(_, n)| n).find(|n| n.label() == Some(name)).unwrap_or_else(|| panic!("no {name}"));
        let b = node.bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    };
    let (area, strip) = (find("Saturation and brightness"), find("Hue"));
    let room = style.edge.width + style.pointer.x;
    assert!((strip.left() - area.right() - gap).abs() < 0.5, "the area is a gap from the strip: {area:?} {strip:?}");
    assert!((strip.width() - style.strip).abs() < 0.5);
    assert!((strip.right() + room - 300.0 - area.left()).abs() < 0.5, "the strip's pointer room ends the row: {area:?} {strip:?}");
    assert!(ColorStrip::outer_width(&style, style.strip) == style.strip + room);
}
