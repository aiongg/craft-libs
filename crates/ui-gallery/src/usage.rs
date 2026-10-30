//! Which components paint with a token: the scan behind the Theme panel's "Used by" list, and the
//! probe lookups that outline a token's shapes in the tiles.
//!
//! The scan probes the default story of each component at rest, one component per frame
//! ([`Probe`] takes about 50 ms per story in a debug build), in both passes: pass A names the
//! semantic tokens, pass B the component colour fields. Results are cached by the probe's
//! [`Key`], which holds the appearance but not the theme's edited values, so editing a colour
//! never scans again.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use craft_theme::{Mode, ThemeDef, TokenRef};
use craft_ui::story::{Args, Story};
use egui::{Context, Rect, Vec2, vec2};

use crate::appearance::Appearance;
use crate::gallery::default_story_of;
use crate::probe::{Cache, Key, Pass, Probe, Usage};

/// The size the scan draws each story at.
const SCAN_SIZE: Vec2 = vec2(480.0, 640.0);

/// The height tiles probe their story at; the width is the tile's.
pub(crate) const TILE_HEIGHT: f32 = 640.0;

/// The pass that names `token`.
fn pass_of(token: TokenRef) -> Pass {
    match token {
        TokenRef::Color(_) => Pass::Semantic,
        TokenRef::Component(_) => Pass::Component,
    }
}

/// What a scan was started for.
#[derive(Clone, PartialEq)]
struct ScanContext {
    appearance: Appearance,
    mode: Mode,
}

/// One scan: the stories still to probe and what the ones done painted.
struct Run {
    context: ScanContext,
    stories: Vec<Story>,
    next: usize,
    /// For each token, the shapes each component painted with it.
    painters: HashMap<TokenRef, BTreeMap<&'static str, usize>>,
}

/// The scan and the probe cache the tiles share. At most one probe runs per frame, whoever asks
/// first.
#[derive(Default)]
pub(crate) struct Scan {
    cache: Cache,
    run: Option<Run>,
    /// Whether this frame's probe has not been used yet.
    budget: bool,
}

impl Scan {
    /// Starts a frame: one more probe may run.
    pub(crate) fn begin_frame(&mut self) {
        self.budget = true;
    }

    /// Starts a scan for `appearance` in `mode` unless one for it exists, restarting when either
    /// changed.
    pub(crate) fn ensure(&mut self, stories: &[Story], appearance: &Appearance, mode: Mode) {
        let context = ScanContext { appearance: appearance.clone(), mode };
        if self.run.as_ref().is_some_and(|r| r.context == context) {
            return;
        }
        let mut components: Vec<&'static str> = Vec::new();
        for story in stories {
            if !components.contains(&story.component) {
                components.push(story.component);
            }
        }
        let stories = components.into_iter().filter_map(|c| default_story_of(stories, c).copied()).collect();
        self.run = Some(Run { context, stories, next: 0, painters: HashMap::new() });
    }

    /// Probes the next component if this frame's probe is unused, and asks for another frame
    /// while any are left.
    pub(crate) fn step(&mut self, ctx: &Context, theme: &ThemeDef) {
        let Some(run) = self.run.as_mut() else { return };
        let Some(story) = run.stories.get(run.next).copied() else { return };
        if !std::mem::take(&mut self.budget) {
            ctx.request_repaint();
            return;
        }
        let ScanContext { appearance, mode } = &run.context;
        for pass in [Pass::Semantic, Pass::Component] {
            let key = key_of(&story, &story.defaults(), appearance, *mode, pass, SCAN_SIZE);
            let usage = self.cache.get_or_run(key, || Probe::story(theme, appearance, *mode, pass, SCAN_SIZE, &story, &mut story.defaults()));
            for used in &usage.uses {
                if let Some(token) = used.token.filter(|t| pass_of(*t) == pass) {
                    *run.painters.entry(token).or_default().entry(story.component).or_default() += 1;
                }
            }
        }
        run.next += 1;
        ctx.request_repaint();
    }

    /// How many components have been probed and how many there are; `None` before a scan starts.
    pub(crate) fn progress(&self) -> Option<(usize, usize)> {
        self.run.as_ref().map(|r| (r.next, r.stories.len()))
    }

    /// The components that painted with `token` so far, each with its shape count, most shapes
    /// first.
    pub(crate) fn painters(&self, token: TokenRef) -> Vec<(&'static str, usize)> {
        let mut painters: Vec<_> =
            self.run.as_ref().and_then(|r| r.painters.get(&token)).map(|m| m.iter().map(|(c, n)| (*c, *n)).collect()).unwrap_or_default();
        painters.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        painters
    }
}

fn key_of(story: &Story, args: &Args, appearance: &Appearance, mode: Mode, pass: Pass, size: Vec2) -> Key {
    Key { what: story.id.to_owned(), args: format!("{args:?}"), appearance: appearance.clone(), mode, pass, size: [size.x as u32, size.y as u32] }
}

/// The token whose shapes the tiles outline, with what probing a tile needs.
pub(crate) struct Highlight<'a> {
    pub(crate) scan: &'a mut Scan,
    pub(crate) token: TokenRef,
    pub(crate) theme: &'a ThemeDef,
    pub(crate) mode: Mode,
}

impl Highlight<'_> {
    /// Where `story` with `args` paints with the token when drawn `size` large, in the drawing's
    /// coordinates. `None` while the story has not been probed: a probe runs now if this frame's
    /// is unused, else the next frame asks again.
    pub(crate) fn rects(&mut self, ctx: &Context, appearance: &Appearance, story: &Story, args: &Args, size: Vec2) -> Option<Vec<Rect>> {
        let pass = pass_of(self.token);
        let key = key_of(story, args, appearance, self.mode, pass, size);
        let usage: Arc<Usage> = match self.scan.cache.get(&key) {
            Some(usage) => usage,
            None if std::mem::take(&mut self.scan.budget) => {
                let theme = self.theme;
                let mode = self.mode;
                self.scan.cache.get_or_run(key, || Probe::story(theme, appearance, mode, pass, size, story, &mut args.clone()))
            }
            None => {
                ctx.request_repaint();
                return None;
            }
        };
        Some(usage.rects(self.token))
    }
}
