//! Built-in words of the panels family.

craft_ui::words! {
    /// The words of the panel patterns; each id is prefixed by its module.
    pub struct PanelWords {
        inspector_empty: "panels.inspectorEmpty" = "No properties";
        palette_search: "panels.paletteSearch" = "Search commands";
        palette_placeholder: "panels.palettePlaceholder" = "Search commands, tools and panels…";
        palette_list: "panels.paletteList" = "Commands";
        palette_empty: "panels.paletteEmpty" = "No matching commands";
        palette_results: count "panels.paletteResults" = "{n} result" | "{n} results";
        tour_back: "panels.tourBack" = "Back";
        tour_next: "panels.tourNext" = "Next";
        tour_done: "panels.tourDone" = "Done";
        tour_skip: "panels.tourSkip" = "Skip";
        tour_count: template "panels.tourCount" = "{n} of {total}";
    }
}
