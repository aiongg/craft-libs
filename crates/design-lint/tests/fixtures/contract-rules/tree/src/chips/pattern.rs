use super::Inspector;

impl Pattern for Inspector {}

impl Inspector {
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.response()
    }
}
