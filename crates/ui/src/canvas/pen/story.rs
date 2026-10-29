//! The pen draft's story, and the host the story and the tests keep their paths and draft in.

use egui::{Key, Modifiers, Ui};
use kurbo::Point;

use super::{PenDraft, PenNames, PenResponse};
use crate::canvas::bezier::story::sample_paths;
use crate::canvas::{BezierNames, BezierOverlay, BezierTool, EditPath, EditSubpath, KnotRef, PathSelection, RulerUnits};
use crate::edit::EditEvent;
use crate::localized::LocaleNumbers;
use crate::story::{Args, Level, Story, Surface};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Pen draft"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story::new("primitives/pen-draft/draw", Level::Primitives, "Pen draft", "Draw", crate::canvas::story::args, show_pen).on(Surface::Canvas)]
}

/// A host's finished paths and open draft, as the story and the tests keep them: it keeps the
/// draft from the events, files a committed one as a path and ends the draft when the pen
/// finishes it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PenHost {
    pub paths: Vec<EditPath>,
    pub draft: Option<EditSubpath>,
    /// The key the draft finishes under when it continues a path.
    resumed: Option<u64>,
    next: u64,
}

impl PenHost {
    pub(crate) fn new(paths: Vec<EditPath>) -> Self {
        let next = paths.iter().map(|p| p.key).max().unwrap_or(0) + 1;
        Self { paths, draft: None, resumed: None, next }
    }

    /// The end points of the open subpaths, for [`PenDraft::ends`].
    pub(crate) fn ends(&self) -> Vec<(u64, KnotRef, Point)> {
        let mut ends = Vec::new();
        for path in &self.paths {
            for (s, sub) in path.subpaths.iter().enumerate() {
                if sub.closed || sub.knots.len() < 2 {
                    continue;
                }
                for k in [0, sub.knots.len() - 1] {
                    ends.push((path.key, KnotRef::new(s, k), sub.knots[k].anchor));
                }
            }
        }
        ends
    }

    /// Applies a pass's events and the path it resumes.
    pub(crate) fn apply(&mut self, r: &PenResponse) {
        match &r.event {
            Some(EditEvent::Update { value, .. }) => self.draft = Some(value.clone()),
            // An empty draft removes the path it was: here a resumed one, already out of `paths`.
            Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) if value.knots.is_empty() => {
                self.resumed = None;
                self.draft = None;
            }
            Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                let key = self.resumed.take().unwrap_or_else(|| {
                    self.next += 1;
                    self.next - 1
                });
                self.paths.push(EditPath { key, subpaths: vec![value.clone()], color: None });
                self.draft = None;
            }
            Some(EditEvent::Cancel { .. }) => self.draft = None,
            Some(EditEvent::Begin { .. }) | None => {}
        }
        if r.finished {
            self.draft = None;
            self.resumed = None;
        }
        if let Some((key, at)) = r.resumed
            && let Some(i) = self.paths.iter().position(|p| p.key == key)
        {
            let mut sub = self.paths.remove(i).subpaths.remove(at.subpath);
            // The end pressed is the last knot.
            if at.knot == 0 {
                sub.knots.reverse();
                for k in &mut sub.knots {
                    std::mem::swap(&mut k.in_ctrl, &mut k.out_ctrl);
                }
            }
            self.draft = Some(sub);
            self.resumed = Some(key);
        }
    }
}

/// The story's finished paths: the bezier story's closed shape and an open path.
pub(crate) fn finished_paths() -> Vec<EditPath> {
    let mut paths = sample_paths();
    paths[1].color = None;
    paths
}

/// Draws a path with the Pen on the story document: click for a corner anchor, drag for smooth
/// handles (⇧ at 45°, ⌥ breaks the pair); press on the last anchor to retract its handle, ⌥ on a
/// handle to move it alone; click the first anchor to close; press on an open path's end to carry
/// on with it. Enter or Esc ends the path, Backspace removes the last anchor, ⌘Z does too.
fn show_pen(ui: &mut Ui, args: &mut Args) {
    let key = ui.scope_id().with("pen-story");
    let mut host: PenHost = ui.data(|d| d.get_temp(key)).unwrap_or_else(|| PenHost::new(finished_paths()));
    let canvas = crate::canvas::story::document(ui, args, "Document");
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
    let selection = PathSelection::default();
    let names = BezierNames::new("Paths", "In handle", "Out handle", "Selection", |i, n| format!("Anchor {i} of {n}"));
    let shown = BezierOverlay::new("story-pen-paths", &host.paths, &selection, &px, &px, names).tool(BezierTool::Show).show(
        ui,
        &canvas.response,
        &canvas.view,
    );
    let _ = shown;
    if host.draft.is_some() && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
        PenDraft::remove_last(ui.ctx(), ui.make_persistent_id("story-pen"));
    }
    let ends = host.ends();
    let names = PenNames::new("Path being drawn", |i, n| format!("Anchor {i} of {n}"));
    let response = PenDraft::new("story-pen", host.draft.as_ref(), names).ends(&ends).units(&px, &px).show(ui, &canvas.response, &canvas.view);
    host.apply(&response);
    ui.data_mut(|d| d.insert_temp(key, host));
}
