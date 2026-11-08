//! The appearance page over a suite held in memory.

use craft_theme::{ThemeDef, ThemeId, Variants, builtins};
use craft_theme_store::ui::AppearancePage;
use craft_theme_store::{SelectionChange, SuiteClient};
use craft_ui::contract::{Bind, Role};
use craft_ui::testing::{Fixture, Target, trial};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An appearance page, with what it reported.
struct Page {
    fixture: Fixture,
    client: Rc<SuiteClient>,
    draft: Rc<RefCell<SelectionChange>>,
    changed: Rc<Cell<bool>>,
    toggled: Rc<RefCell<Option<ThemeId>>>,
}

impl Page {
    fn new(favorites: Vec<ThemeId>) -> Self {
        let client = Rc::new(SuiteClient::memory());
        let draft = Rc::new(RefCell::new(SelectionChange::default()));
        let changed = Rc::new(Cell::new(false));
        let toggled = Rc::new(RefCell::new(None));
        let (suite, held, flagged, flipped) = (Rc::clone(&client), Rc::clone(&draft), Rc::clone(&changed), Rc::clone(&toggled));
        let mut fixture = Fixture::new(move |ui| {
            let mut draft = held.borrow_mut();
            let out = AppearancePage::new(&suite, Bind::bound(&mut *draft)).favorites(&favorites).show(ui);
            flagged.set(flagged.get() || out.changed);
            if out.favorite_toggled.is_some() {
                *flipped.borrow_mut() = out.favorite_toggled;
            }
        });
        fixture.settle();
        Page { fixture, client, draft, changed, toggled }
    }

    /// The topmost node of `role` named `name`: the light row's chip, which comes before the dark row's.
    fn node(&self, role: Role, name: &str) -> Target {
        let mut found: Vec<Target> =
            self.fixture.by_role(role).into_iter().filter(|target| self.fixture.tree().name_of(target.node) == Some(name)).collect();
        found.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
        *found.first().expect("a node of that name")
    }
}

/// The first two built-in themes that have a light variant.
fn light_themes() -> Vec<&'static ThemeDef> {
    builtins().iter().filter(|theme| theme.variants != Variants::DarkOnly).take(2).collect()
}

/// Picking a chip writes the light or dark slot into the draft only; the client is unchanged until the app applies.
#[trial]
fn picking_a_chip_edits_the_draft_only() {
    let themes = light_themes();
    let mut page = Page::new(Vec::new());
    let chip = page.node(Role::RADIO_BUTTON, &themes[1].name);
    page.fixture.click(&chip);
    page.fixture.settle();
    let draft = page.draft.borrow();
    assert_eq!(draft.theme_light.as_ref(), Some(&themes[1].id));
    assert_eq!(draft.theme_dark, None);
    assert!(page.changed.get());
    assert_eq!(page.client.shown().theme_light, None);
}

/// A click on the star badge reports the toggled theme and does not pick it.
#[trial]
fn star_click_toggles_favorite_without_picking() {
    let themes = light_themes();
    let mut page = Page::new(vec![themes[1].id.clone()]);
    let star = page.node(Role::BUTTON, &format!("Remove from Favorites: {}", themes[1].name));
    page.fixture.click(&star);
    page.fixture.settle();
    assert_eq!(page.toggled.borrow().as_ref(), Some(&themes[1].id));
    assert_eq!(*page.draft.borrow(), SelectionChange::default());
}
