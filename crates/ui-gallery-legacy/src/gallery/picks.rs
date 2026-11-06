//! What is selected where: the Overview's selected component, level and tile stories, each
//! component tab's selected story, and the centre tab the side panels follow.
use std::collections::HashMap;

use craft_ui::docking::{Layout, PanelState};
use craft_ui::story::{Level, Story};

use super::catalog::default_story_of;
use crate::layout::PanelId;
use crate::persist::Saved;

pub(super) struct Picks {
    /// The component the Overview has selected: its tile has the ring, and the side panels follow
    /// it while the Overview is the front centre tab.
    pub(super) overview_selected: Option<&'static str>,
    /// The level the Overview shows; selecting a component sets it to the component's.
    pub(super) overview_level: Option<Level>,
    /// The centre tab that was last brought to the front; the side panels follow it.
    pub(super) centre: PanelId,
    /// The story id each Overview tile shows, by component; a component without an entry shows
    /// its default story.
    pub(super) overview_story: HashMap<&'static str, &'static str>,
    /// The selected story id of each component tab; a tab without an entry has its default story
    /// selected.
    pub(super) tab_story: HashMap<&'static str, &'static str>,
    /// The tile to scroll to, until it has been drawn: a component in the Overview, a story id in
    /// a component tab.
    pub(super) scroll_to: Option<&'static str>,
}

impl Picks {
    /// The selections `saved` holds that name a story the gallery has, with `layout` as the
    /// components dock they live in.
    pub(super) fn load(saved: &Saved, stories: &[Story], layout: &Layout<PanelId>) -> Self {
        let component = |name: &str| stories.iter().find(|s| s.component == name).map(|s| s.component);
        let story = |component: &str, id: &str| stories.iter().find(|s| s.component == component && s.id == id).map(|s| (s.component, s.id));
        let mut overview_selected = saved.overview_selected.as_deref().and_then(component);
        let mut overview_story: HashMap<&'static str, &'static str> = saved.overview_story.iter().filter_map(|(c, id)| story(c, id)).collect();
        // A state from before the Overview kept its own selection holds one selected story.
        if overview_selected.is_none()
            && let Some(old) = saved.selected.as_deref().and_then(|id| stories.iter().find(|s| s.id == id))
        {
            overview_selected = Some(old.component);
            overview_story.entry(old.component).or_insert(old.id);
        }
        let tab_story: HashMap<&'static str, &'static str> = saved.tab_story.iter().filter_map(|(c, id)| story(c, id)).collect();
        let overview_level = saved
            .overview_level
            .as_deref()
            .and_then(|slug| Level::ALL.into_iter().find(|l| l.slug() == slug))
            .or_else(|| overview_selected.and_then(|c| stories.iter().find(|s| s.component == c)).map(|s| s.level));
        let tab_story = tab_story.into_iter().filter(|(c, _)| layout.contains(&PanelId::Component(c))).collect();
        Self { overview_selected, overview_level, centre: front_centre(layout), overview_story, tab_story, scroll_to: None }
    }

    /// The centre tab that was last brought to the front, if `layout` still holds it: the
    /// Overview, or a component's tab.
    pub(super) fn front_tab(&self, layout: &Layout<PanelId>) -> PanelId {
        if layout.contains(&self.centre) { self.centre } else { PanelId::View }
    }

    /// The story of `component` that `picks` names, or its default story.
    pub(super) fn story_in(stories: &[Story], picks: &HashMap<&'static str, &'static str>, component: &str) -> Option<Story> {
        picks.get(component).and_then(|id| stories.iter().find(|s| s.id == *id)).or_else(|| default_story_of(stories, component)).copied()
    }

    /// The story the side panels follow when `front` is the front centre tab: the Overview's
    /// selected tile's, or the component tab's selected story. None while the Overview has no
    /// selection.
    pub(super) fn target(&self, stories: &[Story], front: PanelId) -> Option<Story> {
        match front {
            PanelId::Component(component) => Self::story_in(stories, &self.tab_story, component),
            _ => self.overview_selected.and_then(|component| Self::story_in(stories, &self.overview_story, component)),
        }
    }

    /// Selects `story` in the Overview, and in its component's tab when that is open.
    pub(super) fn select(&mut self, story: Story, tab_open: bool) {
        self.overview_selected = Some(story.component);
        self.overview_level = Some(story.level);
        self.overview_story.insert(story.component, story.id);
        if tab_open {
            self.tab_story.insert(story.component, story.id);
        }
    }

    /// Selects `component` in the Overview, and asks to scroll to its tile.
    pub(super) fn select_component(&mut self, stories: &[Story], component: &'static str) {
        self.overview_selected = Some(component);
        self.overview_level = default_story_of(stories, component).map(|s| s.level).or(self.overview_level);
        self.scroll_to = Some(component);
    }

    /// A pick in the Stories panel: in a component tab it selects that story's tile, in the
    /// Overview it changes what the component's tile shows.
    pub(super) fn pick_story(&mut self, front: PanelId, component: &'static str, id: &'static str) {
        if matches!(front, PanelId::Component(_)) {
            self.tab_story.insert(component, id);
            self.scroll_to = Some(id);
        } else {
            self.overview_story.insert(component, id);
        }
    }

    /// A click on the Overview's tile of `component`.
    pub(super) fn pick_overview(&mut self, component: &'static str) {
        self.overview_selected = Some(component);
        self.centre = PanelId::View;
    }

    /// A click on the tile of story `id` in `component`'s tab.
    pub(super) fn pick_tab(&mut self, component: &'static str, id: &'static str) {
        self.tab_story.insert(component, id);
        self.centre = PanelId::Component(component);
    }

    /// Drops the selected story of each component tab that is no longer in `layout`.
    pub(super) fn forget_closed_tabs(&mut self, layout: &Layout<PanelId>) {
        self.tab_story.retain(|c, _| layout.contains(&PanelId::Component(c)));
    }
}

/// The centre tab that shows in `layout`: the Overview when it is in front, else the first
/// component tab that is.
fn front_centre(layout: &Layout<PanelId>) -> PanelId {
    if layout.panel_state(&PanelId::View) == PanelState::Front {
        return PanelId::View;
    }
    layout
        .panels()
        .into_iter()
        .copied()
        .find(|p| matches!(p, PanelId::Component(_)) && layout.panel_state(p) == PanelState::Front)
        .unwrap_or(PanelId::View)
}
