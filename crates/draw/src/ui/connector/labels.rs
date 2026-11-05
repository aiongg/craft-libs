//! The words of the Connector tool and its bar, English by default.

craft_ui::words! {
    /// What a screen reader reads for the Connector tool's parts.
    #[non_exhaustive]
    pub struct ConnectorToolLabels {
        #[allow(missing_docs)]
        group: "connectorTool.group" = "Connector";
        /// The start end glued to `{target}` (the app's object name).
        start_attached: "connectorTool.startAttached" = "Start, attached to {target}";
        #[allow(missing_docs)]
        start_free: "connectorTool.startFree" = "Start, free";
        #[allow(missing_docs)]
        end_attached: "connectorTool.endAttached" = "End, attached to {target}";
        #[allow(missing_docs)]
        end_free: "connectorTool.endFree" = "End, free";
        /// A path handle: `{index}` from 1 of `{count}`.
        bend: "connectorTool.bend" = "Bend {index} of {count}";
    }
}

craft_ui::words! {
    /// The words of [`ConnectorBar`](super::ConnectorBar) (FigCraft's; US spelling).
    #[non_exhaustive]
    pub struct ConnectorBarLabels {
        /// The routing buttons' group name.
        routing: "connectorBar.routing" = "Connector type";
        #[allow(missing_docs)]
        straight: "connectorBar.straight" = "Straight";
        #[allow(missing_docs)]
        elbow: "connectorBar.elbow" = "Elbow";
        #[allow(missing_docs)]
        curved: "connectorBar.curved" = "Curved";
        #[allow(missing_docs)]
        start: "connectorBar.start" = "Start point";
        #[allow(missing_docs)]
        end: "connectorBar.end" = "End point";
        #[allow(missing_docs)]
        line_style: "connectorBar.lineStyle" = "Stroke style";
        #[allow(missing_docs)]
        solid: "connectorBar.solid" = "Solid";
        #[allow(missing_docs)]
        dashed: "connectorBar.dashed" = "Dash";
        /// A weight row: `{weight}` in points.
        weight: "connectorBar.weight" = "Stroke weight {weight}";
    }
}
