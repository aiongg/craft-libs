//! The About dialog's layout rules.

use craft_ui::contract::{Bind, Role};
use craft_ui::patterns::form_dialogs::{AboutDialog, AboutInfo, AboutTab, Contributor, Credits, ModelCredit};
use craft_ui::testing::{Fixture, relations, trial};

/// Whichever tab shows, the table that fills the dialog leaves the OK button inside the screen and below the table.
#[trial]
fn the_footer_stays_visible_under_a_table_that_fills_the_dialog() {
    let info = AboutInfo::new("PhotoCraft", "An image editor.", "0.1");
    let people: Vec<Contributor> = (0..60).map(|at| Contributor::new(format!("person{at}")).counts(1, 10 + at)).collect();
    let models: Vec<ModelCredit> = (0..30).map(|at| ModelCredit::new("Acme", format!("Model {at}"), "1").work(5, 100, 10)).collect();
    let credits = Credits::new(people, models, 1_000);
    for tab in [AboutTab::Contributors, AboutTab::Models] {
        let (info, credits) = (info.clone(), credits.clone());
        let mut fixture = Fixture::new(move |ui| {
            let mut open = true;
            AboutDialog::new(&info, &credits).tab(tab).opened(Bind::bound(&mut open)).show(ui);
        });
        fixture.settle();
        let screen = fixture.ctx().content_rect();
        let table = fixture.by_role(Role::TABLE).first().copied().expect("the table showed");
        let ok = fixture.by_label("OK").expect("the footer button is shown");
        assert!(screen.contains_rect(ok.rect), "{tab:?}: OK is inside the screen: {:?} in {screen:?}", ok.rect);
        assert!(relations::above(table.rect, ok.rect), "{tab:?}: OK is below the table: {:?} under {:?}", ok.rect, table.rect);
    }
}
