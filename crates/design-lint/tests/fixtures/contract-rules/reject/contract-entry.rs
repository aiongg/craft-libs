pub struct Swatch {
    color: Color,
}

impl Swatch {
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.response()
    }
}

pub fn colour_row(ui: &mut Ui, colors: &[Color]) -> EditResponse<Color> {
    todo!()
}
