#[non_exhaustive]
pub struct ChipStyle {
    pub fill: Stateful<StyleColor>,
    pub radius: f32,
}

fn style(cx: &StyleCx, key: ChipKey) -> ChipStyle {
    let group = cx.token_group::<ChipColors>();
    let tokens = cx.tokens();
    ChipStyle { fill: Stateful::new(cx.color(group.fill)), radius: tokens.radius.control }
}

fn plain_colour(colour: Rgba) -> Color32 {
    colour.to_egui()
}

fn kind_width(kind: ChipKind) -> f32 {
    match kind {
        ChipKind::Wide => 2.0,
        ChipKind::Narrow => 1.0,
    }
}

fn has_focus(response: &Response) -> bool {
    if response.has_focus() { true } else { response.hovered() }
}

impl Chip {
    pub fn style_name(&self) -> &str {
        "chip"
    }

    pub fn style(&self) -> &ChipStyle {
        &self.style
    }

    pub fn tone(self, tone: Tone) -> Self {
        self.with(tone)
    }
}

#[cfg(test)]
mod tests {
    fn t(ui: &mut Ui) {
        let _ = Tokens::of(ui);
    }
}
