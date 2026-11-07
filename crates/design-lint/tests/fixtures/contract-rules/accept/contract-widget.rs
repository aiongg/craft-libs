impl egui::DroppedFile for DroppedPath {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl egui::widgets::Label for Plain {}
