#[derive(Props)]
pub struct Chip {
    count: u32,
    note: Option<String>,
}

impl Chip {
    pub fn count(mut self, n: u32) -> Self {
        self.count = n;
        self
    }

    pub fn note(mut self, text: impl Into<String>) -> Self {
        self.note = Some(text.into());
        self
    }
}
