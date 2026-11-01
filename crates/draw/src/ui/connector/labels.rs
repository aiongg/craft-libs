//! The words of the Connector tool and its bar, English by default.

craft_ui::words! {
    /// What a screen reader reads for the Connector tool's parts.
    #[non_exhaustive]
    pub struct ConnectorToolLabels {
        group: "connectorTool.group" = "Connector";
        /// The start end glued to `{target}` (the app's object name).
        start_attached: "connectorTool.startAttached" = "Start, attached to {target}";
        start_free: "connectorTool.startFree" = "Start, free";
        end_attached: "connectorTool.endAttached" = "End, attached to {target}";
        end_free: "connectorTool.endFree" = "End, free";
        /// A path handle: `{index}` from 1 of `{count}`.
        bend: "connectorTool.bend" = "Bend {index} of {count}";
        /// The "+" buttons' group.
        add_connected: "connectorTool.addConnected" = "Add a connected shape";
    }
}

craft_ui::words! {
    /// The words of [`ConnectorBar`](super::ConnectorBar) (FigCraft's; US spelling).
    #[non_exhaustive]
    pub struct ConnectorBarLabels {
        /// The routing buttons' group name.
        routing: "connectorBar.routing" = "Connector type";
        straight: "connectorBar.straight" = "Straight";
        elbow: "connectorBar.elbow" = "Elbow";
        curved: "connectorBar.curved" = "Curved";
        start: "connectorBar.start" = "Start point";
        end: "connectorBar.end" = "End point";
        line_style: "connectorBar.lineStyle" = "Stroke style";
        solid: "connectorBar.solid" = "Solid";
        dashed: "connectorBar.dashed" = "Dash";
        /// A weight row: `{weight}` in points.
        weight: "connectorBar.weight" = "Stroke weight {weight}";
    }
}
