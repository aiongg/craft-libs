fn completion(ui: &mut Ui, items: &[String]) {
    for item in items {
        ui.label(item.as_str());
    }
    ui.heading("x");
    let _ = egui::RichText::new("x");
}
