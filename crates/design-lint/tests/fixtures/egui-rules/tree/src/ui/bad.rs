fn f(ui: &mut Ui) {
    ui.button("x");
    // design: allow[contract/egui-widget](legacy canvas toolbar)
    ui.checkbox(&mut on, "x");
}
