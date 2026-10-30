//! Pick from preview.
use craft_theme::TokenRef;
use craft_ui::inspect::AccessTree;
use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::Gallery;
use crate::layout::{GalleryWorkspace, PanelId, PreviewView};

/// A gallery in a 1280 x 1200 window with AccessKit on, in the Theme workspace.
struct Fixture {
    ctx: Context,
    gallery: Gallery,
    time: f64,
}

impl Fixture {
    fn new(preview: PreviewView) -> Self {
        let ctx = Context::default();
        ctx.enable_accesskit();
        // Tooltips show as soon as the pointer is on a control.
        ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
        let mut gallery = Gallery::new(None);
        gallery.set_workspace(GalleryWorkspace::Theme);
        gallery.set_preview(preview);
        let mut f = Self { ctx, gallery, time: 0.0 };
        f.frames(3);
        f
    }

    fn frame_with(&mut self, events: Vec<Event>) -> Option<AccessTree> {
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 1200.0))),
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
            self.frame_with(Vec::new());
        }
        self.frame_with(Vec::new()).expect("an AccessKit tree")
    }

    fn bounds_of(&mut self, name: &str, role: Role) -> Rect {
        let tree = self.frames(2);
        let id = tree
            .walk(tree.root())
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| tree.name(*id).as_deref() == Some(name) && tree.node(*id).is_some_and(|n| n.role() == role))
            .unwrap_or_else(|| panic!("a {role:?} named {name:?}"));
        tree.bounds(id).expect("bounds")
    }

    fn hover(&mut self, pos: Pos2) {
        self.time += 1.0;
        self.frame_with(vec![Event::PointerMoved(pos)]);
        self.frame_with(Vec::new());
    }

    /// A real primary click: pressed and released in one frame, after the pointer moved there.
    fn click(&mut self, pos: Pos2, modifiers: Modifiers) {
        self.hover(pos);
        self.time += 1.0;
        let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers };
        self.frame_with(vec![button(true), button(false)]);
        self.frame_with(Vec::new());
    }

    fn press(&mut self, key: egui::Key) {
        self.time += 1.0;
        let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
        self.frame_with(vec![event(true), event(false)]);
        self.frame_with(Vec::new());
    }
}

fn path(p: &str) -> TokenRef {
    TokenRef::from_path(p).unwrap_or_else(|| panic!("{p} is a token"))
}

#[test]
fn a_click_on_the_selected_layers_row_selects_state_selected_and_leaves_the_mock_alone() {
    let mut f = Fixture::new(PreviewView::App);
    let row = f.bounds_of("Shapes", Role::ListBoxOption);
    let before = f.gallery.app.mock.doc.selected.clone();
    f.gallery.select_token(path("accent.solid"));
    f.gallery.set_picking(true);
    f.gallery.activate(PanelId::Preview);
    f.hover(row.center());
    assert_eq!(f.gallery.picked_hover(), Some(Some(path("state.selected"))));
    f.click(row.center(), Modifiers::NONE);
    assert_eq!(f.gallery.selected_token(), Some(path("state.selected")));
    assert_eq!(f.gallery.app.mock.doc.selected, before, "the click did not reach the mock app");
    assert!(f.gallery.picking(), "picking stays on");
}

#[test]
fn escape_ends_picking() {
    let mut f = Fixture::new(PreviewView::App);
    f.gallery.set_picking(true);
    f.frames(2);
    assert!(f.gallery.picking());
    f.press(egui::Key::Escape);
    assert!(!f.gallery.picking());
}

#[test]
fn a_computed_colour_names_no_token_and_a_click_on_it_selects_nothing() {
    let mut f = Fixture::new(PreviewView::App);
    f.gallery.set_picking(true);
    // The document's own pixels are not theme colours.
    let doc = f.bounds_of("Document", Role::Canvas);
    f.click(doc.center(), Modifiers::NONE);
    assert_eq!(f.gallery.selected_token(), None);
    assert!(matches!(f.gallery.picked_hover(), None | Some(None)), "{:?}", f.gallery.picked_hover());
}

/// The box of the checked "Snap to grid" checkbox in the Components preview, once every tile has
/// been probed in the pass that `alt` asks for.
fn checked_box(f: &mut Fixture, alt: bool) -> Pos2 {
    f.gallery.set_picking(true);
    if alt {
        f.time += 1.0;
        f.frame_with(vec![Event::ModifiersChanged(Modifiers::ALT)]);
    }
    // Probing is one tile per frame.
    f.frames(45);
    f.bounds_of("Snap to grid (checked)", Role::CheckBox).min + vec2(10.0, 12.0)
}

#[test]
fn in_the_components_preview_a_click_picks_the_tile_shape_and_does_not_press_the_tile() {
    let mut f = Fixture::new(PreviewView::Components);
    let pos = checked_box(&mut f, false);
    f.hover(pos);
    assert!(matches!(f.gallery.picked_hover(), Some(Some(TokenRef::Color(_)))), "{:?}", f.gallery.picked_hover());
    f.click(pos, Modifiers::NONE);
    assert!(matches!(f.gallery.selected_token(), Some(TokenRef::Color(_))), "{:?}", f.gallery.selected_token());
    f.bounds_of("Snap to grid (checked)", Role::CheckBox);
}

#[test]
fn an_alt_click_picks_the_component_field() {
    let mut f = Fixture::new(PreviewView::Components);
    let pos = checked_box(&mut f, true);
    f.click(pos, Modifiers::ALT);
    assert!(matches!(f.gallery.selected_token(), Some(TokenRef::Component(_))), "{:?}", f.gallery.selected_token());
}
