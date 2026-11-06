pub fn install(ui: &mut egui::Ui) {
    ui.style_mut();
}

pub fn size() -> egui::Vec2 {
    egui::Vec2::new(10.0, 20.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn size() {
        let _ = egui::Vec2::new(10.0, 20.0);
    }
}
