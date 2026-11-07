fn paint(response: &Response, tokens: &Palette) -> Color32 {
    if response.hovered() {
        tokens.hover_color
    } else {
        tokens.rest_color
    }
}
