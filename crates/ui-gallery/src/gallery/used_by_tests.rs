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
    shapes: Vec<egui::epaint::ClippedShape>,
}

impl Fixture {
    fn new() -> Self {
        let ctx = Context::default();
        ctx.enable_accesskit();
        Self { ctx, gallery: Gallery::new(None), time: 0.0, shapes: Vec::new() }
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
        self.shapes = output.shapes.clone();
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
        for _ in 0..2 * total + 3 {
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
    f.frames(8);
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

#[test]
fn nothing_is_scanned_while_no_token_is_selected_or_the_list_is_hidden() {
    let mut f = Fixture::new();
    f.frames(3);
    assert_eq!(f.gallery.scan_progress(), None);
    f.gallery.set_workspace(GalleryWorkspace::Theme);
    f.frames(3);
    assert_eq!(f.gallery.scan_progress(), None);
    // A token is selected but the Used by list is on another workspace.
    f.gallery.select_token(ACCENT);
    f.gallery.set_workspace(GalleryWorkspace::Components);
    f.frames(6);
    let hidden = f.gallery.scan_progress().map_or(0, |(done, _)| done);
    assert_eq!(hidden, 0, "a scan step ran with the list hidden");
    f.gallery.set_workspace(GalleryWorkspace::Theme);
    f.frames(6);
    assert!(f.gallery.scan_progress().is_some_and(|(done, _)| done > 0));
}

#[test]
fn a_change_of_direction_restarts_the_scan() {
    let mut f = theme_workspace();
    f.frames(8);
    let (before, _) = f.gallery.scan_progress().unwrap();
    assert!(before >= 3);
    f.gallery.appearance_mut().rtl = true;
    f.frame(Vec::new());
    let (after, _) = f.gallery.scan_progress().unwrap();
    assert!(after < before, "{after} after {before}");
}

#[test]
#[ignore = "needs probe frame fix (W4c2)"]
fn the_outline_sits_on_the_primary_buttons_fill() {
    fn walk(s: &egui::Shape, out: &mut Vec<egui::epaint::RectShape>) {
        match s {
            egui::Shape::Rect(r) => out.push(r.clone()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut f = Fixture::new();
    f.gallery.select_component("Button");
    f.gallery.select_token(ACCENT);
    let tree = f.frames(4);
    let button = tree
        .walk(tree.root())
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| tree.name(*id).as_deref() == Some("Button") && tree.node(*id).is_some_and(|n| n.role() == Role::Button))
        .and_then(|id| tree.node(id).and_then(|n| n.bounds()))
        .expect("the first Button in the tile");
    let ppp = f.ctx.pixels_per_point();
    let fill =
        Rect::from_min_max(egui::pos2(button.x0 as f32 / ppp, button.y0 as f32 / ppp), egui::pos2(button.x1 as f32 / ppp, button.y1 as f32 / ppp));
    let mut rects = Vec::new();
    for clipped in &f.shapes {
        walk(&clipped.shape, &mut rects);
    }
    let outlines: Vec<Rect> = rects
        .iter()
        .filter(|r| r.stroke_kind == egui::StrokeKind::Outside && r.fill == egui::Color32::TRANSPARENT && r.rect.intersects(fill.expand(4.0)))
        .map(|r| r.rect)
        .collect();
    assert_eq!(outlines.len(), 1, "{outlines:?}");
    let o = outlines[0];
    assert!((o.min - fill.min).abs().max_elem() < 0.6 && (o.max - fill.max).abs().max_elem() < 0.6, "{o:?} against {fill:?}");
}
