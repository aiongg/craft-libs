//! The layout components `Stack`, `Row` and `Grid`: containers with no surface, header or footer whose body
//! lays its children out (6.3).

use crate::contract::{Common, Component, Container, ContainerProps, Cx, Gap, Pad, Role, Scroll, Sizing};
use crate::layout::{BodyLayout, RowAlign};
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{ColumnWidth, Metrics};
use egui::Response;

#[cfg(feature = "testing")]
mod stories;

/// The style of the layout components: no padding, no minimum size.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
pub struct LayoutStyle {
    #[style(metrics)]
    metrics: Metrics,
}

impl LayoutStyle {
    fn new() -> Self {
        LayoutStyle { metrics: Metrics::new() }
    }
}

macro_rules! layout_component {
    ($(#[$doc:meta])* $name:ident, $key:ident) => {
        $(#[$doc])*
        #[derive(crate::contract::Component)]
        #[component(container)]
        pub struct $name {
            #[common]
            common: Common,
            #[container]
            props: ContainerProps,
        }

        impl $name {
            /// The spacing between the children.
            pub fn gap(mut self, gap: Gap) -> Self {
                self.props = self.props.body_gap(gap);
                self
            }

            /// The padding tier around the children.
            pub fn pad(mut self, pad: Pad) -> Self {
                self.props = self.props.pad(pad);
                self
            }

            /// Scrolls the children.
            pub fn scroll(mut self, scroll: Scroll) -> Self {
                self.props = self.props.scroll(scroll);
                self
            }

            /// Takes the available rect instead of the size of the children.
            pub fn fill(mut self) -> Self {
                self.props = self.props.sizing(Sizing::Fill);
                self
            }
        }

        impl Component for $name {
            const ROLE: Role = Role::None;
            type Style = LayoutStyle;
            type Output = Response;

            fn style(_: &StyleCx<'_>, _: $key) -> LayoutStyle {
                LayoutStyle::new()
            }

            fn finish(self, cx: Cx<'_, Self>) -> Response {
                cx.core.response
            }
        }

        impl Container for $name {}
    };
}

layout_component!(
    /// Children one under another.
    Stack,
    StackStyleKey
);
layout_component!(
    /// Children side by side.
    Row,
    RowStyleKey
);
layout_component!(
    /// Children row-major over a column list: a child in flow takes the next cell.
    Grid,
    GridStyleKey
);

impl Stack {
    /// A stack that takes the size of its children.
    pub fn new() -> Self {
        Stack { common: Common::new(), props: ContainerProps::new().sizing(Sizing::Content).layout(BodyLayout::stack()) }
    }
}

impl Default for Stack {
    fn default() -> Self {
        Stack::new()
    }
}

impl Row {
    /// A row that takes the size of its children.
    pub fn new() -> Self {
        Row { common: Common::new(), props: ContainerProps::new().sizing(Sizing::Content).layout(BodyLayout::row()) }
    }

    /// Whether children continue on a new line.
    pub fn wrap(mut self, on: bool) -> Self {
        self.props.layout = std::mem::replace(&mut self.props.layout, BodyLayout::Stack).wrap(on);
        self
    }

    /// Where the block of children sits along the row.
    pub fn align(mut self, align: RowAlign) -> Self {
        self.props.layout = std::mem::replace(&mut self.props.layout, BodyLayout::Stack).align(align);
        self
    }
}

impl Default for Row {
    fn default() -> Self {
        Row::new()
    }
}

impl Grid {
    /// A grid over `columns` that takes the size of its cells.
    pub fn new(columns: &'static [ColumnWidth]) -> Self {
        Grid { common: Common::new(), props: ContainerProps::new().sizing(Sizing::Content).layout(BodyLayout::grid(columns)) }
    }
}
