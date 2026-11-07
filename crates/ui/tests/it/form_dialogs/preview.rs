//! The preview pane's pan as AccessKit sees it: the scroll actions of the axes it pans on, and the same pan as the
//! arrow keys when one arrives.

use craft_behavior::CanvasTransform;
use craft_ui::contract::Show as _;
use craft_ui::patterns::form_dialogs::PreviewPane;
use craft_ui::testing::access::Declared;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, trial};
use egui::load::SizedTexture;
use egui::{ColorImage, Key, Modifiers, TextureHandle, TextureOptions, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A picture much wider than the well and much shorter than it: the pane pans along the horizontal axis only.
const WIDE: [usize; 2] = [4000, 8];

struct Rig {
    fixture: Fixture,
    view: Rc<Cell<CanvasTransform>>,
}

fn rig() -> Rig {
    let view = Rc::new(Cell::new(CanvasTransform::new(1.0, Vec2::ZERO)));
    let handle: RefCell<Option<TextureHandle>> = RefCell::new(None);
    let held = Rc::clone(&view);
    let mut fixture = Fixture::new(move |ui| {
        let mut handle = handle.borrow_mut();
        let texture =
            handle.get_or_insert_with(|| ui.ctx().load_texture("preview-pan", ColorImage::filled(WIDE, Default::default()), TextureOptions::LINEAR));
        let sized = SizedTexture::new(texture.id(), Vec2::new(WIDE[0] as f32, WIDE[1] as f32));
        if let Some(next) = PreviewPane::new("Preview", sized, held.get()).show(ui).view {
            held.set(next);
        }
    });
    fixture.settle();
    Rig { fixture, view }
}

/// A picture wider than the well declares ScrollLeft and ScrollRight on the pane, and no vertical scroll action.
#[trial]
fn a_pane_declares_the_scroll_actions_of_the_axes_it_pans_on() {
    let rig = rig();
    let pane = rig.fixture.by_label("Preview").expect("the pane has a node");
    let tree = rig.fixture.tree();
    assert!(
        tree.declares_key(pane.node, Declared::ScrollLeft) && tree.declares_key(pane.node, Declared::ScrollRight),
        "the horizontal pan is declared"
    );
    assert!(
        !tree.declares_key(pane.node, Declared::ScrollUp) && !tree.declares_key(pane.node, Declared::ScrollDown),
        "a picture that fits vertically does not pan vertically"
    );
}

/// AccessKit ScrollRight pans the view as ArrowRight does.
#[trial]
fn scroll_right_pans_as_the_right_arrow_does() {
    let mut keyed = rig();
    keyed.fixture.key(Key::Tab, Modifiers::NONE);
    keyed.fixture.idle(2);
    keyed.fixture.key(Key::ArrowRight, Modifiers::NONE);
    keyed.fixture.idle(2);
    let mut scrolled = rig();
    let pane = scrolled.fixture.by_label("Preview").expect("the pane has a node");
    scrolled.fixture.access(AccessActionKind::ScrollRight, &pane);
    scrolled.fixture.idle(2);
    assert!(keyed.view.get().offset.x < 0.0, "the arrow pans");
    assert_eq!(scrolled.view.get(), keyed.view.get(), "the scroll action pans the same way");
}
