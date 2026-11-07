#[derive(Component)]
pub struct Select {
    #[common]
    common: Common,
}

impl Component for Select {
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {}
}
