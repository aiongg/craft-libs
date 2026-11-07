//! Built-in words of the bars family.

craft_ui::words! {
    /// The words of bars and bar items; each id is prefixed by its type.
    pub struct BarWords {
        tool: "bars.barTool" = "Toolbar";
        options: "bars.barOptions" = "Options";
        status: "bars.barStatus" = "Status";
        items: "bars.itemsActions" = "Actions";
        more: "bars.itemsMore" = "More";
        enter_key: "bars.barEnter" = "Enter";
        escape_key: "bars.barEscape" = "Esc";
    }
}

craft_ui::words! {
    /// The words of the status readout.
    pub struct StatusReadoutWords {
        zoom: "statusReadout.zoom" = "Zoom";
        menu: "statusReadout.menu" = "Show";
        document_sizes: "statusReadout.documentSizes" = "Document Sizes";
        document_profile: "statusReadout.documentProfile" = "Document Profile";
        document_dimensions: "statusReadout.documentDimensions" = "Document Dimensions";
        measurement_scale: "statusReadout.measurementScale" = "Measurement Scale";
        scratch_sizes: "statusReadout.scratchSizes" = "Scratch Sizes";
        efficiency: "statusReadout.efficiency" = "Efficiency";
        timing: "statusReadout.timing" = "Timing";
        current_tool: "statusReadout.currentTool" = "Current Tool";
        layer_count: "statusReadout.layerCount" = "Layer Count";
        layers: count "statusReadout.layers" = "{n} Layer" | "{n} Layers";
        doc_prefix: template "statusReadout.docPrefix" = "Doc: {flat}/{layered}";
        scratch_prefix: template "statusReadout.scratchPrefix" = "Scratch: {used}/{available}";
        efficiency_text: template "statusReadout.efficiencyText" = "Efficiency: {n}%";
        timing_text: template "statusReadout.timingText" = "Timing: {n} s";
        size_kilo: template "statusReadout.sizeKilo" = "{n}K";
        size_mega: template "statusReadout.sizeMega" = "{n}M";
        size_giga: template "statusReadout.sizeGiga" = "{n}G";
    }
}
craft_ui::words! {
    /// The words of the align bar.
    pub struct AlignWords {
        align: "align.align" = "Align";
        distribute: "align.distribute" = "Distribute";
        more: "align.more" = "Align and distribute";
        tooltip: template "align.tooltip" = "{group}: {action}";
        left_edges: "align.leftEdges" = "Left Edges";
        horizontal_centers: "align.horizontalCenters" = "Horizontal Centers";
        right_edges: "align.rightEdges" = "Right Edges";
        top_edges: "align.topEdges" = "Top Edges";
        vertical_centers: "align.verticalCenters" = "Vertical Centers";
        bottom_edges: "align.bottomEdges" = "Bottom Edges";
        horizontally: "align.horizontally" = "Horizontally";
        vertically: "align.vertically" = "Vertically";
    }
}
craft_ui::words! {
    /// The words of the breadcrumb.
    pub struct BreadcrumbWords {
        hidden: "breadcrumb.hidden" = "Show the whole path";
    }
}
