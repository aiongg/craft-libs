//! The Theme panel's Used by list and the tile highlight.
use craft_theme::{ColorToken, ComponentField, ModeChoice, TokenRef};
use craft_ui::inspect::AccessTree;
use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, RawInput, Rect, vec2};

use super::Gallery;
use crate::layout::{GalleryWorkspace, PanelId};

/// A gallery in a 1280 x 2400 window, tall enough that the detail shows its sections without scrolling, with AccessKit on.
struct Fixture {
    ctx: Context,
    gallery: Gallery,
    time: f64,
}

impl Fixture {
    fn new() -> Self {
        let ctx = Context::default();
        ctx.enable_accesskit();
        Self { ctx, gallery: Gallery::new(None), time: 0.0 }
    }

    fn frame(&mut self, events: Vec<Event>) -> Option<AccessTree> {
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 2400.0))),
            events,
            time: Some(self.time),
            ..RawInput::default()
        };
        let output = self.ctx.run_ui(raw, |ui| self.gallery.show(ui));
        let tree = output.platform_output.accesskit_update.as_ref().and_then(AccessTree::from_update);
        output.drop_without_applying_deltas();
        tree
    }

    fn frames(&mut self, count: usize) -> AccessTree {
        for _ in 1..count {
            self.frame(Vec::new());
        }
        self.frame(Vec::new()).expect("an AccessKit tree")
    }

    /// Frames until the scan has probed every component.
    fn scan_to_end(&mut self) {
        let total = self.gallery.scan_progress().map_or(0, |(_, total)| total);
        for _ in 0..total + 3 {
            self.frame(Vec::new());
        }
        let (done, total) = self.gallery.scan_progress().expect("a scan");
        assert_eq!(done, total);
    }

    /// A real primary click on the centre of the node: pressed and released in one frame.
    fn click_link(&mut self, name: &str) {
        let tree = self.frames(2);
        let node = tree
            .walk(tree.root())
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| tree.name(*id).as_deref() == Some(name) && tree.node(*id).is_some_and(|n| n.role() == Role::Link))
            .unwrap_or_else(|| panic!("a link named {name:?}"));
        let b = tree.node(node).and_then(|n| n.bounds()).expect("bounds");
        let ppp = self.ctx.pixels_per_point();
        let pos = egui::pos2(((b.x0 + b.x1) / 2.0) as f32 / ppp, ((b.y0 + b.y1) / 2.0) as f32 / ppp);
        self.time += 1.0;
        let button = |pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![button(true), button(false)]);
        self.frame(Vec::new());
    }
}

const ACCENT: TokenRef = TokenRef::Color(ColorToken::AccentSolid);

fn theme_workspace() -> Fixture {
    let mut f = Fixture::new();
    f.gallery.set_workspace(GalleryWorkspace::Theme);
    f.gallery.select_token(ACCENT);
    f
}

#[test]
fn the_scan_lists_button_as_a_painter_of_accent_solid_and_its_link_opens_the_component() {
    let mut f = theme_workspace();
    f.frame(Vec::new());
    let (done, total) = f.gallery.scan_progress().expect("selecting a token starts the scan");
    assert!(done < total);
    f.scan_to_end();
    let painters = f.gallery.painters(ACCENT);
    assert_eq!(painters.iter().filter(|(c, _)| *c == "Button").count(), 1);
    f.click_link("Button");
    assert_eq!(f.gallery.workspace(), GalleryWorkspace::Components);
    assert_eq!(f.gallery.open_tabs(), ["Button"]);
    assert_eq!(f.gallery.front_tab(), PanelId::Component("Button"));
}

#[test]
fn read_by_lists_the_fields_that_default_to_the_token_and_selects_one_from_its_link() {
    let mut f = theme_workspace();
    let field = ComponentField::ALL.iter().find(|f| f.path() == "button.primary.fill.rest").copied().expect("the primary button fill");
    f.click_link("button.primary.fill.rest");
    assert_eq!(f.gallery.selected_token(), Some(TokenRef::Component(field)));
}

#[test]
fn a_change_of_appearance_restarts_the_scan() {
    let mut f = theme_workspace();
    f.frames(4);
    let (before, _) = f.gallery.scan_progress().unwrap();
    assert!(before >= 3);
    f.gallery.appearance_mut().mode = ModeChoice::Dark;
    f.frame(Vec::new());
    let (after, _) = f.gallery.scan_progress().unwrap();
    assert!(after < before, "{after} after {before}");
}

#[test]
fn tiles_that_paint_the_selected_token_are_outlined_and_none_is_without_a_selection() {
    let mut f = Fixture::new();
    f.gallery.select_component("Button");
    f.frames(3);
    assert!(f.gallery.highlighted_tiles().is_empty());
    f.gallery.select_token(ACCENT);
    f.frames(3);
    assert!(f.gallery.highlighted_tiles().contains(&"Button"), "{:?}", f.gallery.highlighted_tiles());
}
