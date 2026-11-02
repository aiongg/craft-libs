//! [`ScopePanelLabels`]: every word of the scope panel, its menus and the vectorscope (`scopes.*`).

craft_ui::words! {
    /// The scope panel's words, English by default (Photoshop's and Camera Raw's). Pass a
    /// translated copy with [`ScopePanel::labels`](super::ScopePanel::labels); the menus read the
    /// installed words. Channel and source names are the host's ([`ScopeChannel::label`](super::ScopeChannel::label),
    /// [`ScopeSource::label`](super::ScopeSource::label)).
    #[non_exhaustive]
    pub struct ScopePanelLabels {
        /// The plot's accessible name.
        histogram: "scopes.histogram" = "Histogram";
        /// The plot's value text: `{channel}` and `{source}` are the host's labels.
        histogram_value: "scopes.histogramValue" = "{channel} channel, {source}";
        /// The source named in the value text when the host passes none.
        entire_image: "scopes.entireImage" = "Entire Image";
        /// The Channel row's caption.
        channel: "scopes.channel" = "Channel:";
        /// The Channel list's accessible name.
        channel_name: "scopes.channelName" = "Channel";
        /// The Channel list's entry that overlays the colour channels.
        colors: "scopes.colors" = "Colors";
        /// The Source row's caption.
        source: "scopes.source" = "Source:";
        /// The Source list's accessible name.
        source_name: "scopes.sourceName" = "Source";
        /// The panel menu row and the refresh button's name.
        uncached_refresh: "scopes.uncachedRefresh" = "Uncached Refresh";
        /// The cached-data mark's tooltip and name.
        refresh_hint: "scopes.refreshHint" = "Click to refresh histogram with uncached data";
        compact: "scopes.compact" = "Compact View";
        expanded: "scopes.expanded" = "Expanded View";
        all_channels: "scopes.allChannels" = "All Channels View";
        show_statistics: "scopes.showStatistics" = "Show Statistics";
        show_channels_in_color: "scopes.showChannelsInColor" = "Show Channels in Color";
        /// The statistics grid's accessible name.
        statistics: "scopes.statistics" = "Statistics";
        mean: "scopes.mean" = "Mean:";
        standard_deviation: "scopes.standardDeviation" = "Std Dev:";
        median: "scopes.median" = "Median:";
        pixels: "scopes.pixels" = "Pixels:";
        level: "scopes.level" = "Level:";
        count: "scopes.count" = "Count:";
        percentile: "scopes.percentile" = "Percentile:";
        cache_level: "scopes.cacheLevel" = "Cache Level:";
        /// The scope menu's rows.
        lab_readouts: "scopes.labReadouts" = "Show Lab Color Readouts";
        show_vectorscope: "scopes.showVectorscope" = "Show Vectorscope";
        selected_region: "scopes.selectedRegion" = "Show Selected Region";
        red_right: "scopes.redRight" = "Show Red at 3 o'clock";
        skin_tone: "scopes.skinTone" = "Show Skin Tone Indicator";
        /// The clipping buttons' names and tooltips.
        shadow_warning: "scopes.shadowWarning" = "Shadow Clipping Warning (U)";
        highlight_warning: "scopes.highlightWarning" = "Highlight Clipping Warning (O)";
        /// The tone zones' group name.
        tone_zones: "scopes.toneZones" = "Tone zones";
        blacks: "scopes.blacks" = "Blacks";
        shadows: "scopes.shadows" = "Shadows";
        exposure: "scopes.exposure" = "Exposure";
        highlights: "scopes.highlights" = "Highlights";
        whites: "scopes.whites" = "Whites";
        /// A zone's tooltip: `{zone}` its name, `{value}` the host's value text.
        zone_tooltip: "scopes.zoneTooltip" = "{zone}  {value}";
        /// The vectorscope's accessible name.
        vectorscope: "scopes.vectorscope" = "Vectorscope";
        /// The vectorscope's value text ([`VectorscopeSummary`](crate::scopes::VectorscopeSummary)).
        vectorscope_value: "scopes.vectorscopeValue" = "Peak saturation {saturation} %, mean hue {hue} degrees";
        /// The hover badge, RGB form: `{r}`, `{g}`, `{b}` in 0..=255, no decimals.
        readout_rgb: "scopes.readoutRgb" = "R {r}  G {g}  B {b}";
        /// The hover badge, Lab form, one decimal.
        readout_lab: "scopes.readoutLab" = "L {l}  a {a}  b {b}";
    }
}
