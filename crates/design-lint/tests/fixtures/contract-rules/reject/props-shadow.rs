#[derive(Props)]
pub struct Chip {
    #[prop]
    selected: bool,
    #[prop(name = "label")]
    text: String,
}
