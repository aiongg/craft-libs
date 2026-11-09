//! The tabs and search, the filters, the sample toolbar and the list of family cards.

#[cfg(feature = "screen-testing")]
mod stories;

use std::borrow::Cow;
use std::sync::Arc;

use craft_behavior::{ColumnWidth, DisplayOrder, Flow, ItemSource, Metrics, Proposal, Selection, SelectionChange, SpanLayout};
use craft_host::Host;
use craft_i18n::{Folder, Format};
use craft_theme::variant::{Emphasis, Tone};
use craft_ui::buttons::{Button, Link};
use craft_ui::choice::{Checkbox, Radio, RadioGroup};
use craft_ui::contract::{
    Access, ActivateOn, Bind, Collection, CollectionEvent, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, Entry,
    ItemOutput, MeasureCx, Name, NoValue, Outcome, PartsCx, Props as _, Role, Scroll, SelectMode, Show as _, ShowCollection as _, ShowContainer as _,
    Sizing, Styled as _, clicked,
};
use craft_ui::display::{Badge, Progress, Shade, Spinner, Text};
use craft_ui::form::SliderField;
use craft_ui::layout_components::{Grid, Row, Stack};
use craft_ui::list::{List, ListRow};
use craft_ui::notices::{Notice, NoticeAction, NoticeLayout};
use craft_ui::pickers::Select;
use craft_ui::primitives::TextRun;
use craft_ui::runtime::env::{Label, UnitKind};
use craft_ui::sections::Section;
use craft_ui::style::{ComponentStyle, Ink, RuleKind, Stateful, StyleCx, TextLook, TypeRole, Variant};
use craft_ui::tabs::{Tab as TabItem, TabKind, TabStrip};
use craft_ui::text::TextField;
use egui::{Id, Pos2, Rect, Response, Ui, Vec2};

use crate::index::{Category, Index, source_label};
use crate::panel::app::{
    Detail, FontsPanel, FontsPanelWords, Origin, Pending, Preset, Show, Tab, category_label, family_key, regular, source_name, style_count,
};
use crate::panel::faces::Owner;
use crate::panel::jobs::Kind;
use crate::panel::specimen::{FontSample, SampleFace};
use crate::{Catalog, FaceInfo, Source};

/// The language filter lists this many matches under its search field.
const LANGUAGE_MATCHES: usize = 8;
/// The most source problems the panel spells out.
const SOURCE_NOTICES: usize = 4;
/// How often a pass is asked for while the installed fonts are being read.
const SCAN_POLL_SECONDS: f64 = 0.1;

craft_ui::words! {
    /// The words of the panel's tabs, filters, toolbar, notes and family cards.
    pub struct FontsBrowseWords {
        tabs: "fontsBrowse.tabs" = "Fonts";
        tab_browse: "fontsBrowse.tabBrowse" = "Browse";
        tab_installed: "fontsBrowse.tabInstalled" = "Installed";
        search: "fontsBrowse.search" = "Search";
        search_catalogue: "fontsBrowse.searchCatalogue" = "Search fonts by name";
        search_installed: "fontsBrowse.searchInstalled" = "Search installed fonts";
        filters: "fontsBrowse.filters" = "Filters";
        downloads: "fontsBrowse.downloads" = "Downloads";
        allow_downloads: "fontsBrowse.allowDownloads" = "Allow downloads";
        downloads_hint: "fontsBrowse.downloadsHint" = "From each font's source, only when you install or preview a font.";
        category: "fontsBrowse.category" = "Category";
        all_categories: "fontsBrowse.allCategories" = "All categories";
        language: "fontsBrowse.language" = "Language";
        any_language: "fontsBrowse.anyLanguage" = "Any language";
        language_placeholder: "fontsBrowse.languagePlaceholder" = "Name in English or in the language";
        find_language: "fontsBrowse.findLanguage" = "Find a language";
        script: "fontsBrowse.script" = "Script";
        any_script: "fontsBrowse.anyScript" = "Any script";
        source: "fontsBrowse.source" = "Source";
        all_sources: "fontsBrowse.allSources" = "All sources";
        show: "fontsBrowse.show" = "Show";
        all_fonts: "fontsBrowse.allFonts" = "All fonts";
        installed: "fontsBrowse.installed" = "Installed";
        not_installed: "fontsBrowse.notInstalled" = "Not installed";
        clear_filters: "fontsBrowse.clearFilters" = "Clear filters";
        installed_note: "fontsBrowse.installedNote" = "Every Craft app uses the fonts in the font store. System fonts are added and removed with your system's own font tools.";
        sample: "fontsBrowse.sample" = "Sample";
        sentence: "fontsBrowse.sentence" = "Sentence";
        alphabet: "fontsBrowse.alphabet" = "Alphabet";
        numerals: "fontsBrowse.numerals" = "Numerals";
        sample_text: "fontsBrowse.sampleText" = "Sample text";
        size: "fontsBrowse.size" = "Size";
        downloads_off: "fontsBrowse.downloadsOff" = "Downloads are off";
        downloads_off_text: "fontsBrowse.downloadsOffText" = "Fonts that aren't installed show a sample once you preview them. Nothing is downloaded until you allow it.";
        cards: "fontsBrowse.cards" = "Families";
        no_match: "fontsBrowse.noMatch" = "No fonts match";
        no_match_hint: "fontsBrowse.noMatchHint" = "Try another name, or fewer filters.";
        clear_search: "fontsBrowse.clearSearch" = "Clear search and filters";
        indexes_unreadable: template "fontsBrowse.indexesUnreadable" = "The font indexes can't be read: {error}";
        sources_failed: count "fontsBrowse.sourcesFailed" = "{n} font source couldn't be loaded" | "{n} font sources couldn't be loaded";
        source_error: template "fontsBrowse.sourceError" = "{source}: {error}";
        try_again: "fontsBrowse.tryAgain" = "Try again";
        looking: "fontsBrowse.looking" = "Looking for installed fonts…";
        no_installed: "fontsBrowse.noInstalled" = "No installed fonts match";
        installed_empty: "fontsBrowse.installedEmpty" = "Fonts you install from Browse go here, for every Craft app.";
        try_name: "fontsBrowse.tryName" = "Try another name.";
        installing: "fontsBrowse.installing" = "Installing";
        removing: "fontsBrowse.removing" = "Removing";
        downloading: "fontsBrowse.downloading" = "Downloading";
        progress: template "fontsBrowse.progress" = "{what} {percent} %";
        working: template "fontsBrowse.working" = "{what}…";
        retry: "fontsBrowse.retry" = "Retry";
        failed_install: "fontsBrowse.failedInstall" = "Couldn't install";
        failed_remove: "fontsBrowse.failedRemove" = "Couldn't remove";
        failed_download: "fontsBrowse.failedDownload" = "Couldn't download";
        install: "fontsBrowse.install" = "Install";
        install_family: template "fontsBrowse.installFamily" = "Install {family}";
        preview: "fontsBrowse.preview" = "Preview";
        preview_family: template "fontsBrowse.previewFamily" = "Preview {family}";
        remove: "fontsBrowse.remove" = "Remove";
        remove_family: template "fontsBrowse.removeFamily" = "Remove {family}";
        not_curated: "fontsBrowse.notCurated" = "Not curated";
        not_curated_tip: "fontsBrowse.notCuratedTip" = "This family comes from a source you added. Craft doesn't review it.";
        card_sample: "fontsBrowse.cardSample" = "Sample";
    }
}

/// What a family card asks the panel for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CardEvent {
    /// Install the family.
    Install,
    /// Download the family for a preview.
    Preview,
    /// Remove the family from the font store.
    Remove,
    /// Try the failed job again.
    Retry,
}

/// What a card shows at its reading end.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CardState {
    /// A job runs: what it does and how far it is.
    Running(Kind, Option<f32>),
    /// The last job failed: what it did, why, and whether Retry applies.
    Failed(Kind, String, bool),
    /// The family is installed and can't be removed here.
    Installed,
    /// The family is in the font store.
    Removable,
    /// A system family.
    System,
    /// Not installed: Install, and Preview until a preview is downloaded.
    #[non_exhaustive]
    Offer {
        /// Whether Preview is offered.
        preview: bool,
    },
}

/// What a family card shows.
#[derive(Clone)]
#[non_exhaustive]
pub struct CardData {
    /// The family name.
    pub name: String,
    /// Styles, categories and source on one line.
    pub meta: String,
    /// The sample text.
    pub sample: String,
    /// The sample size in points.
    pub points: f32,
    /// False for a family from a source the user added.
    pub curated: bool,
    /// What is at the reading end.
    pub state: CardState,
    /// The face the sample is drawn in, and whose catalog holds it.
    pub face: Option<(Owner, FaceInfo, Arc<Catalog>)>,
}

/// The pieces of a card at the reading end, in reading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
    NotCurated,
    Failed,
    Retry,
    Installed,
    Preview,
    Install,
    Remove,
    RunningText,
    RunningBar,
}

impl End {
    fn event(self) -> Option<CardEvent> {
        match self {
            End::Install => Some(CardEvent::Install),
            End::Preview => Some(CardEvent::Preview),
            End::Remove => Some(CardEvent::Remove),
            End::Retry => Some(CardEvent::Retry),
            _ => None,
        }
    }
}

/// Builds the control for one piece of a card's end and runs `$body` on it as `$c`. A control is built where it is
/// measured and again where it is shown, so both come from this one list.
macro_rules! with_end {
    ($card:expr, $words:expr, $end:expr, |$c:ident| $body:expr) => {
        match $end {
            End::NotCurated => {
                let $c = Badge::new($words.not_curated.clone())
                    .variant(Variant::look(Emphasis::Soft, Tone::Warn))
                    .tooltip($words.not_curated_tip.as_str().to_owned());
                $body
            }
            End::Failed => {
                let $c = $card.failed_text(&$words);
                $body
            }
            End::Retry => {
                let $c = Button::new($words.retry.clone());
                $body
            }
            End::Installed => {
                let $c = Badge::new($words.installed.clone()).variant(Variant::look(Emphasis::Soft, Tone::Good));
                $body
            }
            End::Preview => {
                let $c = Link::new($words.preview.clone()).description($words.preview_family.fill(&[("family", $card.data.name.as_str())]));
                $body
            }
            End::Install => {
                let $c = Button::new($words.install.clone()).description($words.install_family.fill(&[("family", $card.data.name.as_str())]));
                $body
            }
            End::Remove => {
                let $c = Button::new($words.remove.clone())
                    .variant(Variant::DANGER)
                    .description($words.remove_family.fill(&[("family", $card.data.name.as_str())]));
                $body
            }
            End::RunningText => {
                let $c = Text::new(Label::from($card.running_text(&$words))).shade(Shade::Muted);
                $body
            }
            End::RunningBar => {
                let $c = match $card.fraction() {
                    Some(fraction) => Progress::new(fraction),
                    None => Progress::indeterminate(),
                }
                .label(Label::from($card.running_text(&$words)));
                $body
            }
        }
    };
}

/// The style of a family card.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FamilyCardStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The family name.
    #[style(text)]
    name: TextLook,
    /// Styles, categories and source.
    #[style(text)]
    meta: TextLook,
    /// Space round the card's contents.
    pad: f32,
    /// Space between the card's parts.
    gap: f32,
    /// The header's least height.
    line: f32,
}

/// Where the card's parts sit, from the parts step to the paint.
#[derive(Clone, Debug)]
struct Placed {
    name: Rect,
    meta: Option<Rect>,
    end: Vec<(End, Rect)>,
    sample: Rect,
    rule: (Pos2, Pos2),
}

/// The sizes of the card's parts at one width.
struct Plan {
    name: Vec2,
    end: Vec<(End, Vec2)>,
    header: f32,
    sample: Vec2,
}

/// What a family card returns.
#[non_exhaustive]
pub struct CardOut {
    /// The card's response.
    pub response: Response,
    /// What the pressed control asks for.
    pub event: Option<CardEvent>,
}

impl Outcome for CardOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<CardEvent> for CardOut {
    fn into_event(self) -> Option<CardEvent> {
        self.event
    }
}

/// One family: its name and meta line, its state at the reading end and a sample in the face below. The controls at the
/// end and the sample are child components of the card, so a press on a control never selects the card.
#[derive(craft_ui::contract::Component)]
pub struct FamilyCard {
    #[common]
    common: Common,
    data: CardData,
    placed: Option<Placed>,
    event: Option<CardEvent>,
}

impl FamilyCard {
    /// A card showing `data`.
    pub fn new(data: CardData) -> Self {
        let name = data.name.clone();
        FamilyCard { common: Common::new(), data, placed: None, event: None }.label(name)
    }

    fn fraction(&self) -> Option<f32> {
        match self.data.state {
            CardState::Running(_, fraction) => fraction,
            _ => None,
        }
    }

    fn running_text(&self, words: &FontsBrowseWords) -> String {
        let CardState::Running(kind, fraction) = &self.data.state else { return String::new() };
        let what = match kind {
            Kind::Install => words.installing.as_str(),
            Kind::Remove => words.removing.as_str(),
            Kind::Preview => words.downloading.as_str(),
        };
        match fraction {
            Some(fraction) => words.progress.fill(&[("what", what), ("percent", &((fraction * 100.0).round() as u64).to_string())]),
            None => words.working.fill(&[("what", what)]),
        }
    }

    /// The "Couldn't ..." text of a failed job, with the error as its tooltip.
    fn failed_text(&self, words: &FontsBrowseWords) -> Text {
        let (what, error) = match &self.data.state {
            CardState::Failed(kind, error, _) => {
                let what = match kind {
                    Kind::Install => &words.failed_install,
                    Kind::Remove => &words.failed_remove,
                    Kind::Preview => &words.failed_download,
                };
                (what.clone(), error.clone())
            }
            _ => (Label::Empty, String::new()),
        };
        Text::new(what).tone(Tone::Bad).tooltip(error)
    }

    /// The pieces at the reading end, in reading order.
    fn ends(&self) -> Vec<End> {
        let mut ends = Vec::new();
        if !self.data.curated {
            ends.push(End::NotCurated);
        }
        match &self.data.state {
            CardState::Running(..) => ends.extend([End::RunningText, End::RunningBar]),
            CardState::Failed(_, _, retry) => {
                ends.push(End::Failed);
                if *retry {
                    ends.push(End::Retry);
                }
            }
            CardState::Installed => ends.push(End::Installed),
            CardState::Removable => ends.push(End::Remove),
            CardState::System => {}
            CardState::Offer { preview } => {
                if *preview {
                    ends.push(End::Preview);
                }
                ends.push(End::Install);
            }
        }
        ends
    }

    /// The face the sample is drawn in: the card's own, or the interface face when there is none.
    fn face(&self) -> SampleFace<'_> {
        match &self.data.face {
            Some((owner, face, catalog)) => SampleFace::Face { catalog, owner: *owner, face },
            None => SampleFace::Interface,
        }
    }

    fn sample<'a>(&'a self, words: &FontsBrowseWords) -> FontSample<'a> {
        FontSample::new(words.card_sample.clone(), self.data.sample.as_str(), self.face(), self.data.points)
    }

    /// The sizes of the parts at `width`.
    fn plan(&self, cx: &MeasureCx<'_, Self>, width: f32) -> Plan {
        let style = cx.style;
        let words = cx.env.words::<FontsBrowseWords>();
        let name = cx.layout(&TextRun::new(self.data.name.as_str()).role(style.name.role)).size();
        let end: Vec<(End, Vec2)> = self
            .ends()
            .into_iter()
            .map(|end| (end, with_end!(self, words, end, |child| cx.child(("end", end as u8), &mut { child }, Proposal::Ideal))))
            .collect();
        let header = end.iter().map(|(_, size)| size.y).fold(name.y.max(style.line), f32::max);
        let inner = (width - 2.0 * style.pad).max(0.0);
        let mut sample = self.sample(&words);
        let sample = cx.child("sample", &mut sample, Proposal::Width(inner));
        Plan { name, end, header, sample }
    }
}

impl Component for FamilyCard {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = FamilyCardStyle;
    type Output = CardOut;

    fn style(cx: &StyleCx<'_>, _: FamilyCardStyleKey) -> FamilyCardStyle {
        FamilyCardStyle {
            metrics: Metrics::new(),
            name: TextLook::new(TypeRole::BodyStrong, Stateful::new(cx.ink(Ink::OnPanel))),
            meta: TextLook::new(TypeRole::Caption, Stateful::new(cx.color(cx.tokens.color.text.faint))),
            pad: cx.tokens.space.pad.panel,
            gap: cx.tokens.space.gap.inline,
            line: cx.control().height,
        }
    }

    /// The card is named by its family and its meta line, which the layout may elide.
    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.name(Name::Text(format!("{}, {}", self.data.name, self.data.meta)));
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let style = cx.style;
        let width = match cx.room {
            Some(room) => room,
            None => {
                let ideal = self.plan(cx, 0.0);
                2.0 * style.pad + ideal.name.x + ideal.end.iter().map(|(_, size)| size.x + style.gap).sum::<f32>()
            }
        };
        let plan = self.plan(cx, width);
        Vec2::new(width, style.pad + plan.header + style.gap + plan.sample.y + style.pad)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let rtl = cx.flow.is_rtl();
        let style = cx.style.clone();
        let plan = self.plan(&*cx, rect.width());
        let header = Rect::from_min_size(
            Pos2::new(rect.left() + style.pad, rect.top() + style.pad),
            Vec2::new((rect.width() - 2.0 * style.pad).max(0.0), plan.header),
        );
        let centred = |slot: Rect, size: Vec2| Rect::from_center_size(Pos2::new(slot.center().x, header.center().y), size);
        let mut flow = Flow::new(header, rtl);
        let mut end = Vec::new();
        for (piece, size) in plan.end.iter().rev() {
            let (slot, rest) = flow.take_end(size.x);
            end.push((*piece, centred(slot, *size)));
            flow = rest.take_end(style.gap).1;
        }
        end.reverse();
        let name_width = plan.name.x.min(flow.rect().width());
        let (name_slot, rest) = flow.take_start(name_width);
        let meta_slot = rest.take_start(style.gap).1.rect();
        let meta = (meta_slot.width() > 0.0).then_some(meta_slot);
        let sample_top = header.bottom() + style.gap;
        let sample = Rect::from_min_size(Pos2::new(header.left(), sample_top), Vec2::new(header.width(), plan.sample.y));
        self.placed = Some(Placed {
            name: Rect::from_center_size(Pos2::new(name_slot.center().x, header.center().y), Vec2::new(name_width, plan.name.y)),
            meta,
            end,
            sample,
            rule: (rect.left_bottom(), rect.right_bottom()),
        });
    }

    fn paint(&mut self, ui: &mut craft_ui::contract::PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(placed) = self.placed.as_ref() else { return };
        let style = cx.style;
        let words = cx.env.words::<FontsBrowseWords>();
        {
            let draw = ui.draw();
            draw.text(placed.name, &TextRun::new(self.data.name.as_str()).role(style.name.role).color(cx.resolve(&style.name.color)).elide());
            if let Some(meta) = placed.meta {
                draw.text(meta, &TextRun::new(self.data.meta.as_str()).role(style.meta.role).color(cx.resolve(&style.meta.color)).elide());
            }
            draw.rule(RuleKind::Divider, placed.rule.0, placed.rule.1);
        }
        for (piece, rect) in &placed.end {
            let pressed = with_end!(self, words, *piece, |child| clicked(&ui.child(("end", *piece as u8), *rect, child)));
            if pressed {
                self.event = piece.event();
            }
        }
        let sample = self.sample(&words);
        ui.child("sample", placed.sample, sample);
    }

    fn finish(self, cx: Cx<'_, Self>) -> CardOut {
        CardOut { response: cx.response.clone(), event: self.event }
    }
}

/// The style of the card list: it has no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FamilyCardsStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The family cards of one list: one Tab stop, Up and Down move the lead, Enter or a double click activates a card.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
pub struct FamilyCards {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
}

impl FamilyCards {
    /// A list named `label`.
    pub fn new(label: impl Into<Label>) -> Self {
        FamilyCards {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Fill),
            items: CollectionProps::new().select(SelectMode::Single),
        }
        .label(label)
    }
}

impl Component for FamilyCards {
    const ROLE: Role = Role::LIST;
    type Style = FamilyCardsStyle;
    type Output = CollectionResponse<usize, CardEvent>;

    fn style(_: &StyleCx<'_>, _: FamilyCardsStyleKey) -> FamilyCardsStyle {
        FamilyCardsStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for FamilyCards {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = FamilyCard;
    type ItemEvent = CardEvent;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, cx: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::sized(source, width, cx.control().height)
    }
}

/// The cards of a list in display order: each key is a position in the index or in the installed families.
pub struct CardKeys<'a> {
    keys: &'a [usize],
    name: Box<dyn Fn(usize) -> Option<&'a str> + 'a>,
}

impl<'a> CardKeys<'a> {
    /// The cards of `keys`; `name` gives a key's family name for type-ahead.
    pub fn new(keys: &'a [usize], name: impl Fn(usize) -> Option<&'a str> + 'a) -> Self {
        CardKeys { keys, name: Box::new(name) }
    }
}

impl DisplayOrder<usize> for CardKeys<'_> {
    fn len(&self) -> usize {
        self.keys.len()
    }

    fn key(&self, at: usize) -> usize {
        self.keys[at]
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        (self.name)(self.keys[at]).map(Cow::Borrowed)
    }
}

impl ItemSource<usize> for CardKeys<'_> {}

/// The tabs in display order.
struct Tabs;

const TABS: [Tab; 2] = [Tab::Catalogue, Tab::Installed];

impl DisplayOrder<Tab> for Tabs {
    fn len(&self) -> usize {
        TABS.len()
    }

    fn key(&self, at: usize) -> Tab {
        TABS[at]
    }
}

impl ItemSource<Tab> for Tabs {}

/// The words and number format a pass builds cards from.
pub(crate) struct Texts<'a> {
    pub panel: &'a FontsPanelWords,
    pub browse: &'a FontsBrowseWords,
    pub format: &'a Format,
}

impl Texts<'_> {
    fn styles(&self, n: usize) -> String {
        self.panel.styles.text(n as u64, &self.format.integer(n as u64))
    }

    fn families(&self, n: usize) -> String {
        self.panel.families.text(n as u64, &self.format.integer(n as u64))
    }
}

/// The tabs' share of the top bar beside the search field.
const TABS_SHARE: f32 = 2.0;
/// The search field's share of the top bar beside the tabs.
const SEARCH_SHARE: f32 = 1.0;
static TOP_BAR: [ColumnWidth; 2] = [ColumnWidth::fill(TABS_SHARE), ColumnWidth::fill(SEARCH_SHARE)];

impl FontsPanel {
    pub(crate) fn top_bar(&mut self, ui: &mut Ui, browse: &FontsBrowseWords, format: &Format, search: Id) {
        Grid::new(&TOP_BAR).fill_width().show(ui, |ui, _| {
            let counts = [self.catalogue.as_ref().map_or(0, |c| c.index.families().len()), self.installed.len()];
            let out = TabStrip::new(browse.tabs.clone()).kind(TabKind::Text).shown(self.tab).show_items(ui, &Tabs, &mut |tab, _| {
                let (label, count) = match tab {
                    Tab::Catalogue => (&browse.tab_browse, counts[0]),
                    Tab::Installed => (&browse.tab_installed, counts[1]),
                };
                TabItem::new(label.clone()).detail(Label::from(format.integer(count as u64)))
            });
            for event in out.response.events {
                if let CollectionEvent::Selection(SelectionChange::Set(keys)) = event
                    && let Some(tab) = keys.into_iter().last()
                {
                    self.set_tab(tab);
                }
            }
            let (text, hint) = match self.tab {
                Tab::Catalogue => (&mut self.query.text, &browse.search_catalogue),
                Tab::Installed => (&mut self.installed_text, &browse.search_installed),
            };
            TextField::search(browse.search.clone(), Bind::bound(text)).placeholder(hint.clone()).id(search).show(ui);
        });
    }

    pub(crate) fn filters(&mut self, ui: &mut Ui, texts: &Texts<'_>) {
        let browse = texts.browse;
        Stack::new().scroll(Scroll::vertical()).label(browse.filters.clone()).show(ui, |ui, _| {
            match self.tab {
                Tab::Catalogue => self.catalogue_filters(ui, texts),
                Tab::Installed => self.installed_filters(ui, texts),
            }
            if self.downloads_switch {
                Section::fixed(browse.downloads.clone()).id_salt("downloads").show(ui, |ui, _| {
                    let mut allowed = self.allowed();
                    let before = allowed;
                    Checkbox::new(Bind::bound(&mut allowed)).label(browse.allow_downloads.clone()).show(ui);
                    if allowed != before {
                        let _ = self.set_downloads_allowed(allowed);
                    }
                    Text::new(browse.downloads_hint.clone()).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
                });
            }
        });
    }

    fn catalogue_filters(&mut self, ui: &mut Ui, texts: &Texts<'_>) {
        let (browse, panel) = (texts.browse, texts.panel);
        let Ok(c) = &self.catalogue else { return };
        let index = Arc::clone(&c.index);
        let matches = c.search.languages(&self.language_text);
        let scripts: Vec<usize> = c.search.scripts().to_vec();

        let mut rows = vec![(None, browse.all_categories.clone(), self.category_counts[0])];
        rows.extend(
            Category::ALL
                .iter()
                .enumerate()
                .map(|(n, c)| (Some(*c), panel.category(*c).clone(), self.category_counts.get(n + 1).copied().unwrap_or(0))),
        );
        Section::fixed(browse.category.clone()).id_salt("category").show(ui, |ui, _| {
            radio_column(ui, browse.category.clone(), texts.format, &mut self.query.category, &rows);
        });

        Section::fixed(browse.language.clone()).id_salt("language").show(ui, |ui, _| {
            TextField::search(browse.find_language.clone(), Bind::bound(&mut self.language_text))
                .placeholder(browse.language_placeholder.clone())
                .show(ui);
            let label = |i: usize| {
                index.languages().get(i).map_or_else(String::new, |l| match l.autonym.as_deref() {
                    Some(autonym) if autonym != l.name => format!("{} · {autonym}", l.name),
                    _ => l.name.clone(),
                })
            };
            // Row 0 is "Any language"; row n + 1 is language n. The chosen language stays listed while the search hides it.
            let mut rows = vec![0usize];
            rows.extend(self.query.language.filter(|l| !matches.contains(l)).map(|l| l + 1));
            rows.extend(matches.iter().take(LANGUAGE_MATCHES).map(|l| l + 1));
            let current = Selection::single(self.query.language.map_or(0, |l| l + 1));
            let any = browse.any_language.clone();
            let listed = List::<usize>::new(browse.language.clone())
                .select(SelectMode::Single)
                .activate(ActivateOn::Click)
                .allow_empty(false)
                .header(false)
                .sizing(Sizing::Content)
                .natural_height(true)
                .selection(Bind::given(current))
                .show_items(ui, &CardKeys::new(&rows, |_| None), &mut |key, _| {
                    ListRow::new(if *key == 0 { any.clone() } else { Label::from(label(*key - 1)) })
                });
            for event in listed.events {
                if let CollectionEvent::Selection(SelectionChange::Set(keys)) = event
                    && let Some(key) = keys.into_iter().last()
                {
                    self.query.language = key.checked_sub(1);
                }
            }
        });

        let mut entries = vec![Entry::new(None, browse.any_script.clone())];
        entries.extend(scripts.iter().filter_map(|&i| index.scripts().get(i)).map(|s| Entry::new(Some(s.id.clone()), s.name.clone())));
        Section::fixed(browse.script.clone()).id_salt("script").show(ui, |ui, _| {
            Select::new(browse.script.clone(), Bind::bound(&mut self.query.script), &entries).show(ui);
        });

        if index.sources().len() > 1 {
            let mut entries = vec![Entry::new(None, browse.all_sources.clone())];
            entries.extend(index.sources().iter().map(|s| Entry::new(Some(s.id.clone()), s.name.clone())));
            Section::fixed(browse.source.clone()).id_salt("source").show(ui, |ui, _| {
                Select::new(browse.source.clone(), Bind::bound(&mut self.source), &entries).show(ui);
            });
        }

        let rows = [
            (Show::All, browse.all_fonts.clone(), self.show_counts[0]),
            (Show::Installed, browse.installed.clone(), self.show_counts[1]),
            (Show::NotInstalled, browse.not_installed.clone(), self.show_counts[2]),
        ];
        Section::fixed(browse.show.clone()).id_salt("show").show(ui, |ui, _| {
            radio_column(ui, browse.show.clone(), texts.format, &mut self.show, &rows);
        });

        let filtered = self.query.category.is_some()
            || self.query.language.is_some()
            || self.query.script.is_some()
            || self.show != Show::All
            || self.source.is_some();
        if filtered && clicked(&Link::new(browse.clear_filters.clone()).show(ui)) {
            self.clear_filters();
        }
    }

    fn clear_filters(&mut self) {
        self.query.category = None;
        self.query.language = None;
        self.query.script = None;
        self.source = None;
        self.show = Show::All;
    }

    fn installed_filters(&mut self, ui: &mut Ui, texts: &Texts<'_>) {
        let browse = texts.browse;
        let store = self.installed.iter().filter(|f| f.source == Source::Store).count();
        let rows = [
            (Origin::All, browse.all_sources.clone(), self.installed.len()),
            (Origin::Store, texts.panel.font_store.clone(), store),
            (Origin::System, texts.panel.system.clone(), self.installed.len() - store),
        ];
        Section::fixed(browse.source.clone()).id_salt("origin").show(ui, |ui, _| {
            radio_column(ui, browse.source.clone(), texts.format, &mut self.origin, &rows);
            Text::new(browse.installed_note.clone()).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
        });
    }

    pub(crate) fn main(&mut self, ui: &mut Ui, texts: &Texts<'_>, folder: &Folder, host: Option<&Arc<Host>>) {
        Stack::new().fill().show(ui, |ui, _| {
            self.toolbar(ui, texts.browse);
            if let Some(error) = self.notice.clone() {
                Notice::new(Tone::Bad, Label::from(texts.panel.setting_failed.fill(&[("error", error.as_str())]))).show(ui);
            }
            match self.tab {
                Tab::Catalogue => {
                    self.download_note(ui, texts.browse);
                    self.catalogue_list(ui, texts, host);
                }
                Tab::Installed => self.installed_list(ui, texts, folder, host),
            }
        });
    }

    fn toolbar(&mut self, ui: &mut Ui, browse: &FontsBrowseWords) {
        Row::new().fill_width().show(ui, |ui, _| {
            let presets = [
                Entry::new(Preset::Sentence, browse.sentence.clone()),
                Entry::new(Preset::Alphabet, browse.alphabet.clone()),
                Entry::new(Preset::Numerals, browse.numerals.clone()),
            ];
            let before = self.settings.preset;
            Select::new(browse.sample.clone(), Bind::bound(&mut self.settings.preset), &presets).show(ui);
            if self.settings.preset != before {
                self.settings.sample.clear();
            }
            let hint = self.sample_for(["Latn"]);
            TextField::new(browse.sample_text.clone(), Bind::bound(&mut self.settings.sample)).placeholder(Label::from(hint)).show(ui);
            let mut size = f64::from(self.settings.size);
            SliderField::new(browse.size.clone(), Bind::bound(&mut size), 12.0..=120.0).unit_kind(UnitKind::Points).show(ui);
            self.settings.size = size as f32;
        });
    }

    fn download_note(&mut self, ui: &mut Ui, browse: &FontsBrowseWords) {
        if self.allowed() || self.settings.hide_download_note {
            return;
        }
        let out = Notice::new(Tone::Info, browse.downloads_off_text.clone())
            .title(browse.downloads_off.clone())
            .action(NoticeAction::new(browse.allow_downloads.clone()))
            .dismiss()
            .show(ui);
        if out.action == Some(0) {
            let _ = self.set_downloads_allowed(true);
        }
        if out.dismissed {
            self.settings.hide_download_note = true;
        }
    }

    fn catalogue_list(&mut self, ui: &mut Ui, texts: &Texts<'_>, host: Option<&Arc<Host>>) {
        let browse = texts.browse;
        let index = match &self.catalogue {
            Ok(c) => Arc::clone(&c.index),
            Err(error) => {
                Notice::new(Tone::Bad, Label::from(browse.indexes_unreadable.fill(&[("error", error.as_str())]))).show(ui);
                return;
            }
        };
        self.source_problems(ui, texts, &index, host);
        Text::new(Label::from(texts.families(self.results.len()))).role(TypeRole::Caption).shade(Shade::Faint).show(ui);
        if self.results.is_empty() {
            self.empty(ui, browse, &browse.no_match, &browse.no_match_hint, true);
            return;
        }
        let results = self.results.clone();
        let events = {
            let me = &*self;
            let keys = CardKeys::new(&results, |i| index.families().get(i).map(|f| f.family.as_str()));
            FamilyCards::new(browse.cards.clone()).show_items(ui, &keys, &mut |key, _| FamilyCard::new(me.catalogue_card(&index, *key, texts))).events
        };
        for event in events {
            match event {
                CollectionEvent::Activate(i) => self.detail = Some(Detail::Family(i)),
                CollectionEvent::Item(i, CardEvent::Install) => self.ask(host, Pending::Install(i)),
                CollectionEvent::Item(i, CardEvent::Preview) => self.ask(host, Pending::Preview(i)),
                CollectionEvent::Item(i, CardEvent::Retry) => {
                    let retry = index
                        .families()
                        .get(i)
                        .and_then(|f| self.failed.get(&family_key(&f.source, &f.family)))
                        .and_then(|failure| failure.retry.clone());
                    if let Some(retry) = retry {
                        self.ask(host, retry);
                    }
                }
                _ => {}
            }
        }
    }

    /// Sources that couldn't be loaded, one note each above the list; the others show as usual.
    fn source_problems(&mut self, ui: &mut Ui, texts: &Texts<'_>, index: &Index, host: Option<&Arc<Host>>) {
        let browse = texts.browse;
        let problems = index.problems();
        if problems.is_empty() {
            return;
        }
        let retry = self.allowed() && !self.fetching_sources();
        let title = browse.sources_failed.text(problems.len() as u64, &texts.format.integer(problems.len() as u64));
        let mut again = false;
        for (at, problem) in problems.iter().take(SOURCE_NOTICES).enumerate() {
            let message = browse.source_error.fill(&[("source", problem.name.as_str()), ("error", problem.error.as_str())]);
            let mut notice = Notice::new(Tone::Warn, Label::from(message)).id_salt(at);
            if at == 0 {
                notice = notice.title(Label::from(title.clone()));
            }
            if retry {
                notice = notice.action(NoticeAction::new(browse.try_again.clone()));
            }
            again |= notice.show(ui).action == Some(0);
        }
        if again {
            self.fetch_sources(host);
        }
    }

    fn installed_list(&mut self, ui: &mut Ui, texts: &Texts<'_>, folder: &Folder, host: Option<&Arc<Host>>) {
        let browse = texts.browse;
        if !self.catalog.is_ready() {
            Row::new().show(ui, |ui, _| {
                Spinner::new().label(browse.looking.clone()).show(ui);
                Text::new(browse.looking.clone()).shade(Shade::Muted).show(ui);
            });
            craft_ui::runtime::frame::request_repaint_after(ui.ctx(), SCAN_POLL_SECONDS);
            return;
        }
        let shown = self.installed_shown(folder);
        Text::new(Label::from(texts.families(shown.len()))).role(TypeRole::Caption).shade(Shade::Faint).show(ui);
        if shown.is_empty() {
            let hint = if self.origin == Origin::Store && self.installed_text.trim().is_empty() { &browse.installed_empty } else { &browse.try_name };
            self.empty(ui, browse, &browse.no_installed, hint, false);
            return;
        }
        let events = {
            let me = &*self;
            let keys = CardKeys::new(&shown, |i| me.installed.get(i).map(|f| f.name.as_str()));
            FamilyCards::new(browse.cards.clone()).show_items(ui, &keys, &mut |key, _| FamilyCard::new(me.installed_card(*key, texts))).events
        };
        for event in events {
            match event {
                CollectionEvent::Activate(i) => {
                    if let Some(family) = self.installed.get(i).cloned() {
                        let in_index =
                            self.index().and_then(|index| index.families().iter().position(|f| f.family.eq_ignore_ascii_case(&family.name)));
                        self.detail = Some(match in_index {
                            Some(i) if family.source == Source::Store => Detail::Family(i),
                            _ => Detail::Installed(family.name.clone(), family.source),
                        });
                    }
                }
                CollectionEvent::Item(i, CardEvent::Remove) => {
                    if let Some(family) = self.installed.get(i).cloned() {
                        self.remove(host, &family.name, &family.faces, true);
                    }
                }
                _ => {}
            }
        }
    }

    fn empty(&mut self, ui: &mut Ui, browse: &FontsBrowseWords, title: &Label, hint: &Label, clear: bool) {
        let mut notice = Notice::new(Tone::Info, hint.clone()).title(title.clone()).layout(NoticeLayout::Centred);
        if clear {
            notice = notice.action(NoticeAction::new(browse.clear_search.clone()));
        }
        if notice.show(ui).action == Some(0) {
            self.query.text.clear();
            self.clear_filters();
        }
    }

    /// What a catalogue family's card shows.
    fn catalogue_card(&self, index: &Index, i: usize, texts: &Texts<'_>) -> CardData {
        let Some(family) = index.families().get(i) else { return CardData::empty() };
        let key = family_key(&family.source, &family.family);
        let installed = self.installed_by_name.get(&family.family.to_lowercase()).and_then(|f| regular(f)).cloned();
        let previewed = self.previewed.get(&key).and_then(|f| regular(f)).cloned();
        let state = if let Some(job) = self.jobs.of(&family.source, &family.family) {
            CardState::Running(job.kind, job.fraction())
        } else if let Some(failure) = self.failed.get(&key) {
            CardState::Failed(failure.kind, failure.error.clone(), failure.retry.is_some())
        } else if installed.is_some() {
            CardState::Installed
        } else {
            CardState::Offer { preview: !self.previewed.contains_key(&key) }
        };
        let (source, curated) = source_label(index, family);
        let meta = [texts.styles(style_count(family)), category_label(texts.panel, family), source.to_owned()].join(" · ");
        let face = match (installed, previewed) {
            (Some(face), _) => Some((Owner::Installed, face, Arc::clone(&self.catalog))),
            (None, Some(face)) => Some((Owner::Preview, face, Arc::clone(&self.previews))),
            (None, None) => None,
        };
        CardData {
            name: family.family.clone(),
            meta,
            sample: self.sample_for(family.scripts.iter().map(String::as_str)),
            points: self.settings.size,
            curated,
            state,
            face,
        }
    }

    /// What an installed family's card shows.
    fn installed_card(&self, i: usize, texts: &Texts<'_>) -> CardData {
        let Some(family) = self.installed.get(i) else { return CardData::empty() };
        let meta = [texts.styles(family.faces.len()), source_name(texts.panel, family.source).as_str().to_owned()].join(" · ");
        let state = match self.jobs.of("", &family.name) {
            Some(job) => CardState::Running(job.kind, job.fraction()),
            None => match self.failed.get(&family_key("", &family.name)) {
                Some(failure) => CardState::Failed(failure.kind, failure.error.clone(), false),
                None if family.removable() => CardState::Removable,
                None => CardState::System,
            },
        };
        let scripts = self.index().and_then(|index| index.family(&family.name).map(|f| f.scripts.clone())).unwrap_or_default();
        CardData {
            name: family.name.clone(),
            meta,
            sample: self.sample_for(scripts.iter().map(String::as_str)),
            points: self.settings.size,
            curated: true,
            state,
            face: regular(&family.faces).cloned().map(|face| (Owner::Installed, face, Arc::clone(&self.catalog))),
        }
    }
}

impl CardData {
    /// A card for a key the index no longer holds.
    fn empty() -> CardData {
        CardData { name: String::new(), meta: String::new(), sample: String::new(), points: 0.0, curated: true, state: CardState::System, face: None }
    }
}

/// A group of radio buttons over `current`, each row with its count after the name.
fn radio_column<T: Clone + PartialEq + Send + Sync + 'static>(
    ui: &mut Ui,
    label: Label,
    format: &Format,
    current: &mut T,
    rows: &[(T, Label, usize)],
) {
    RadioGroup::new(label, Bind::bound(current)).show(ui, |ui, _| {
        Stack::new().show(ui, |ui, _| {
            for (value, name, count) in rows {
                let count = format.integer(*count as u64);
                Row::new().show(ui, |ui, _| {
                    Radio::new(value.clone()).label(name.clone()).description(count.clone()).show(ui);
                    Text::new(count.clone()).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
                });
            }
        });
    });
}
