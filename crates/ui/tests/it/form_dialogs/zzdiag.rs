use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{CommandForm, ImageSize, NewDocumentDialog, DocumentOptions, Param};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

fn dump(fixture: &Fixture) -> String {
    let tree = fixture.tree();
    let mut out = String::new();
    for key in tree.keys() {
        let b = tree.bounds_of(key).map(|r| format!("{:.0},{:.0},{:.0},{:.0}", r.min.x, r.min.y, r.max.x, r.max.y)).unwrap_or_default();
        out.push_str(&format!("{:?} {:?} {:?}\n", tree.role_of(key), tree.name_of(key), b));
    }
    out
}

#[trial]
fn zz_command_form() {
    let open = Rc::new(RefCell::new(true));
    let held = Rc::clone(&open);
    let mut fixture = Fixture::new(move |ui| {
        let mut o = held.borrow_mut();
        CommandForm::new("k", "Fit", Param::parse_list(r#"{"name":"","keep":bool}"#)).opened(Bind::bound(&mut *o)).show(ui);
    });
    fixture.settle();
    panic!("DIAG-CF
{}", dump(&fixture));
}

#[trial]
fn zz_image_size() {
    let mut fixture = Fixture::new(move |ui| {
        let mut o = true;
        ImageSize::new(PixelSize::new(1600, 900)).opened(Bind::bound(&mut o)).show(ui);
    });
    fixture.settle();
    panic!("DIAG-IS\n{}", dump(&fixture));
}

#[trial]
fn zz_new_doc() {
    let options = Rc::new(RefCell::new(DocumentOptions::default()));
    let held = Rc::clone(&options);
    let mut fixture = Fixture::new(move |ui| {
        let mut options = held.borrow_mut();
        NewDocumentDialog::new().options(Bind::bound(&mut *options)).show(ui);
    });
    fixture.settle();
    panic!("DIAG-ND\n{}", dump(&fixture));
}
