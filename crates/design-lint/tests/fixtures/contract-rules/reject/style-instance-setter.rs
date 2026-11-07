impl Chip {
    pub fn with_style(self, style: ChipStyle) -> Self {
        self.apply(style)
    }

    pub fn custom(self, look: ChipLook) -> Self {
        self.apply(look)
    }
}
