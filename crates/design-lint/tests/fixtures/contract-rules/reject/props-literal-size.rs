#[derive(Props)]
pub struct Bar {
    /// Width in points.
    #[prop]
    width: f32,
    /// Size in points.
    #[prop]
    size_hint: Option<Vec2>,
}
