//! Call sites as an app writes them: components declared with the derives, installed with a style replacement
//! and a custom variant, and shown inside `Context::run_ui`. This is an integration binary, an external crate
//! as an app is, so the derives' `::craft_ui` paths resolve as they do there.

use craft_behavior::Proposal;
use craft_theme::Variant;
use craft_ui::contract::{Looks, Props as _, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::runtime::{InstallSpec, install};
use craft_ui::testing::contract::clicked;
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Response, Vec2};

mod app {
    use craft_behavior::{DisplayOrder, ItemSource, Metrics, PartKey, Proposal, Selection, Sense, SpanLayout};
    use craft_theme::{Rgba, States, Tokens, Variant, VariantLook};
    use craft_ui::contract::{
        Collection, CollectionProps, Common, Component, Container, ContainerProps, Cx, MeasureCx, PaintUi, PartsCx, Props, Role,
    };
    use craft_ui::runtime::env::{AppValues, EnvValue};
    use craft_ui::style::{ComponentStyle, StyleCx};
    use egui::{Rect, Response, Vec2};

    /// A look for the app's own components.
    #[derive(Clone, Debug, PartialEq, ComponentStyle)]
    pub struct PlainLook {
        #[style(metrics)]
        metrics: Metrics,
    }

    impl PlainLook {
        pub fn new(min: Vec2) -> Self {
            PlainLook { metrics: Metrics::new().min_size(min) }
        }
    }

    fn record_look(_: &Tokens) -> VariantLook {
        let (fill, text) = (Rgba::rgb(200, 30, 30), Rgba::rgb(255, 255, 255));
        VariantLook::new(States::new(fill, fill, fill, fill), States::new(text, text, text, text))
    }

    /// The app's own variant for `Badge`, built in a `const`.
    pub const RECORD: craft_theme::variant::CustomVariant = craft_theme::variant::CustomVariant::new::<BadgeStyleKey>("app.record", record_look);

    /// An app value a container sets for what it shows.
    #[derive(Clone, Debug, PartialEq, Hash, EnvValue)]
    #[env(style, root = Indent(0))]
    pub struct Indent(pub u32);

    /// A leaf with setters.
    #[derive(Component, Props)]
    pub struct Badge {
        #[common]
        common: Common,
        /// The count shown beside the label.
        #[prop]
        count: Option<u32>,
        /// Working: shown dimmed.
        #[prop(flag)]
        busy: bool,
    }

    impl Badge {
        pub fn new() -> Self {
            Badge { common: Common::new(), count: None, busy: false }
        }
    }

    impl Component for Badge {
        const ROLE: Role = Role::GROUP;
        type Style = PlainLook;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: BadgeStyleKey) -> PlainLook {
            PlainLook::new(Vec2::new(24.0, 16.0))
        }

        fn sense(&self) -> Sense {
            if self.busy { Sense::empty() } else { Sense::CLICK | Sense::FOCUS }
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    const FACE: PartKey = PartKey::new("face");

    /// A composite: a part that wears a badge's look.
    #[derive(Component)]
    pub struct Tag {
        #[common]
        common: Common,
    }

    impl Tag {
        pub fn new() -> Self {
            Tag { common: Common::new() }
        }
    }

    impl Component for Tag {
        const ROLE: Role = Role::GROUP;
        type Style = PlainLook;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: TagStyleKey) -> PlainLook {
            PlainLook::new(Vec2::ZERO)
        }

        fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
            cx.child("face", &mut Badge::new(), Proposal::Ideal)
        }

        fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
            cx.part(FACE, rect).sense(Sense::CLICK).pointer_only("Enter activates the component");
        }

        fn paint(&mut self, ui: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
            ui.face(FACE, Badge::new().count(3).variant(Variant::Custom(RECORD)));
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    /// A container that sets an app value for its body.
    #[derive(Component, Props)]
    #[component(container)]
    pub struct Card {
        #[common]
        common: Common,
        #[container]
        props: ContainerProps,
        /// Whether the card shows a close control.
        #[prop(flag)]
        closable: bool,
    }

    impl Card {
        pub fn new(title: &'static str) -> Self {
            Card { common: Common::new(), props: ContainerProps::new(), closable: false }.label(title)
        }
    }

    impl Component for Card {
        const ROLE: Role = Role::GROUP;
        type Style = PlainLook;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: CardStyleKey) -> PlainLook {
            PlainLook::new(Vec2::new(60.0, 30.0))
        }

        fn content_env(&self, _: &Cx<'_, Self>, values: &mut AppValues) {
            values.set(Indent(12));
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    impl Container for Card {}

    /// A source of `n` keys.
    pub struct Count(pub usize);

    impl DisplayOrder<usize> for Count {
        fn len(&self) -> usize {
            self.0
        }

        fn key(&self, at: usize) -> usize {
            at
        }
    }

    impl ItemSource<usize> for Count {}

    /// A collection of badges.
    #[derive(Component)]
    #[component(collection)]
    pub struct Strip {
        #[common]
        common: Common,
        #[container]
        props: ContainerProps,
        items: CollectionProps,
    }

    impl Strip {
        pub fn new() -> Self {
            Strip { common: Common::new(), props: ContainerProps::new(), items: CollectionProps::new() }
        }
    }

    impl Component for Strip {
        const ROLE: Role = Role::LIST;
        type Style = PlainLook;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: StripStyleKey) -> PlainLook {
            PlainLook::new(Vec2::ZERO)
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    impl Collection for Strip {
        type Key = usize;
        type ItemLayout = SpanLayout;
        type Selection = Selection<usize>;
        type Item<'r> = Badge;
        type ItemEvent = ();

        fn items(&self) -> &CollectionProps {
            &self.items
        }

        fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, _: &StyleCx<'_>) -> SpanLayout {
            SpanLayout::new(source, width, 20.0)
        }
    }
}

use app::{Badge, Card, Count, RECORD, Strip, Tag};

#[trial]
fn every_call_site_shows() {
    let mut frames = Frames::uninstalled();
    // An app-wide style replacement, frozen at install. Variants need no registration.
    let mut looks = Looks::new();
    looks.style::<Badge>(|_, _| app::PlainLook::new(Vec2::new(30.0, 18.0)));
    assert!(install(&frames.ctx, craft_ui::testing::install_spec().looks(looks)).is_ok(), "one replacement per type");
    let help = String::from("help");
    frames.frame(vec![], |ui| {
        // A leaf with setters, a rich tooltip that borrows app state, and a measure call.
        let response: Response = Badge::new().count(2).busy().variant(Variant::PRIMARY).show_with_tip(ui, |ui| {
            ui.label(&help);
        });
        let _clicked = clicked(&response);
        Badge::new().variant(Variant::Custom(RECORD)).show(ui);
        let _size = Badge::new().measure_in(ui, Proposal::Ideal);
        // A part wearing another component's look.
        Tag::new().show(ui);
        // A container with a body result; the output is reached through `Deref`.
        let inner = Card::new("Card").closable().show(ui, |ui, cx| {
            Badge::new().show(ui);
            cx.rect
        });
        let _body_ran: Option<Rect> = inner.inner;
        let _response: &Response = &inner;
        // A collection with one item component per visible key.
        let _strip = Strip::new().show_items(ui, &Count(3), &mut |_, _| Badge::new());
    });
}
