//! The pen draft's story, and the host the story and the tests keep their paths and draft in.

use craft_ui::localized::Labels as _;
use egui::{Key, Modifiers, Ui};
use kurbo::Point;

use super::bezier::story::sample_paths;
use super::{PathOptionsPopover, PathOptionsSettings, PenDraft, PenMode, PenNames, PenResponse};
use crate::ui::pen::bezier::{BezierNames, BezierOverlay, BezierTool, EditPath, EditSubpath, Knot, KnotRef, PathSelection};
use craft_ui::canvas::RulerUnits;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::story::{Level, Story, StoryArgs, Surface};
use craft_ui::testing::canvas_story::{PlainArgs, ViewArgs};
use craft_ui::typed_story;
use egui::Color32;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Pen draft", "Path options"];

#[derive(StoryArgs, Debug, PartialEq)]
struct CurvatureArgs {
    #[arg(flatten)]
    view: ViewArgs,
    closed: bool,
    #[arg(name = "corner anchors")]
    corners: bool,
    #[arg(name = "rubber band")]
    rubber_band: bool,
}

impl Default for CurvatureArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), closed: false, corners: false, rubber_band: true }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("primitives/pen-draft/draw", Level::Primitives, "Pen draft", "Draw", PlainArgs::default(), show_pen).on(Surface::Canvas),
        typed_story!("primitives/pen-draft/curvature", Level::Primitives, "Pen draft", "Curvature", CurvatureArgs::default(), show_curvature)
            .on(Surface::Canvas),
        typed_story!("primitives/pen-draft/auto-add-delete", Level::Primitives, "Pen draft", "Auto add/delete", PlainArgs::default(), show_auto)
            .on(Surface::Canvas),
        typed_story!("primitives/path-options/popover", Level::Primitives, "Path options", "Popover", PlainArgs::default(), show_options)
            .on(Surface::Canvas),
    ]
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
            Some(_) | None => {}
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
fn show_pen(ui: &mut Ui, args: &mut PlainArgs) {
    let key = ui.scope_id().with("pen-story");
    let mut host: PenHost = ui.data(|d| d.get_temp(key)).unwrap_or_else(|| PenHost::new(finished_paths()));
    let document_name = <craft_ui::story::StoryLabels as craft_ui::localized::Labels>::of(ui).document;
    let canvas = craft_ui::testing::canvas_story::document(ui, args.view, &document_name);
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
    let selection = PathSelection::default();
    let names = BezierNames::of(ui);
    let shown = BezierOverlay::new("story-pen-paths", &host.paths, &selection, &px, &px, names).tool(BezierTool::Show).show(
        ui,
        &canvas.response,
        &canvas.view,
    );
    let _ = shown;
    let undo = host.draft.is_some() && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z));
    let ends = host.ends();
    let names = PenNames::of(ui);
    let response =
        PenDraft::new("story-pen", host.draft.as_ref(), names).ends(&ends).units(&px, &px).remove_last(undo).show(ui, &canvas.response, &canvas.view);
    host.apply(&response);
    ui.data_mut(|d| d.insert_temp(key, host));
}

/// A draft of `points`: smooth anchors the Curvature Pen's handles flow through, the second one a
/// corner when `corner`.
fn curvature_draft(points: &[(f64, f64)], closed: bool, corner: bool) -> EditSubpath {
    let knots = points
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            let at = Point::new(x, y);
            if corner && i == 1 { Knot::corner(at) } else { Knot::smooth(at, at) }
        })
        .collect();
    let mut draft = EditSubpath { knots, closed };
    draft.reflow();
    draft
}

/// The Curvature Pen on a draft of five anchors (the arguments choose a closed draft, a corner
/// anchor and the rubber band): click for a smooth anchor, double-click an anchor to toggle it,
/// drag an anchor to move it, click the first anchor to close, Backspace or ⌘Z removes the last.
/// The preview curve to the pointer shows whatever the rubber band setting. Changing the draft's
/// arguments starts it again.
fn show_curvature(ui: &mut Ui, args: &mut CurvatureArgs) {
    const POINTS: [(f64, f64); 5] = [(110.0, 280.0), (180.0, 150.0), (280.0, 110.0), (380.0, 200.0), (470.0, 140.0)];
    let key = ui.scope_id().with("pen-curvature-story");
    let shape = (args.closed, args.corners);
    let (mut host, seen): (PenHost, (bool, bool)) = ui.data(|d| d.get_temp(key)).filter(|(_, seen)| *seen == shape).unwrap_or_else(|| {
        let mut host = PenHost::new(Vec::new());
        host.draft = Some(curvature_draft(&POINTS, args.closed, args.corners));
        (host, shape)
    });
    let options = PathOptionsSettings { rubber_band: args.rubber_band, ..PathOptionsSettings::default() };
    run_pen(ui, &mut host, args.view, "story-pen-curvature", PenMode::Curvature, false, Some(&options));
    ui.data_mut(|d| d.insert_temp(key, (host, seen)));
}

/// The Pen with Auto Add/Delete on an open draft: hover a segment of the draft to see the add
/// look and click to add an anchor, hover an inner anchor to see the delete look and click to
/// remove it; the first and last anchors are left to the pen, and ⇧ turns the behaviour off.
fn show_auto(ui: &mut Ui, args: &mut PlainArgs) {
    let key = ui.scope_id().with("pen-auto-story");
    let mut host: PenHost = ui.data(|d| d.get_temp(key)).unwrap_or_else(|| {
        let mut host = PenHost::new(Vec::new());
        host.draft = Some(EditSubpath {
            knots: vec![
                Knot::corner(Point::new(100.0, 270.0)),
                Knot::smooth(Point::new(220.0, 130.0), Point::new(280.0, 130.0)),
                Knot::smooth(Point::new(380.0, 260.0), Point::new(440.0, 260.0)),
                Knot::corner(Point::new(520.0, 120.0)),
            ],
            closed: false,
        });
        host
    });
    run_pen(ui, &mut host, args.view, "story-pen-auto", PenMode::Pen, true, None);
    ui.data_mut(|d| d.insert_temp(key, host));
}

/// The colours the story's stand-in for the app's colour picker cycles through.
const PICKS: [Color32; 3] = [Color32::from_rgb(230, 110, 20), Color32::from_rgb(40, 140, 220), Color32::from_rgb(60, 170, 90)];

/// The Path Options gear above a canvas strip with a draft and a finished path: the thickness,
/// colour and rubber band reach both. The colour swatch cycles through a few colours, where an
/// app opens its picker.
fn show_options(ui: &mut Ui, args: &mut PlainArgs) {
    let key = ui.scope_id().with("pen-options-story");
    let (mut host, mut settings, mut open, mut picks): (PenHost, PathOptionsSettings, bool, usize) =
        ui.data(|d| d.get_temp(key)).unwrap_or_else(|| {
            let mut host = PenHost::new(finished_paths());
            host.draft = Some(EditSubpath {
                knots: vec![
                    Knot::corner(Point::new(80.0, 120.0)),
                    Knot::smooth(Point::new(160.0, 60.0), Point::new(210.0, 60.0)),
                    Knot::corner(Point::new(280.0, 110.0)),
                ],
                closed: false,
            });
            (host, PathOptionsSettings { rubber_band: true, ..PathOptionsSettings::default() }, false, 0)
        });
    let response = PathOptionsPopover::new("story-path-options", &settings).show(ui, &mut open);
    if let Some(value) = response.event.as_ref().and_then(EditEvent::value) {
        settings = *value;
    }
    if response.pick_color.is_some() {
        let pick = PICKS[picks % PICKS.len()];
        settings.color = Some([pick.r(), pick.g(), pick.b()]);
        picks += 1;
    }
    run_pen(ui, &mut host, args.view, "story-pen-options", PenMode::Pen, false, Some(&settings));
    ui.data_mut(|d| d.insert_temp(key, (host, settings, open, picks)));
}

/// One pass of the pen over the host's finished paths: the paths in the Show tool, then the draft
/// in `mode`, as an app runs them. The keys that change the draft go to it first.
fn run_pen(ui: &mut Ui, host: &mut PenHost, view: ViewArgs, salt: &str, mode: PenMode, auto: bool, options: Option<&PathOptionsSettings>) {
    let document_name = <craft_ui::story::StoryLabels as craft_ui::localized::Labels>::of(ui).document;
    let canvas = craft_ui::testing::canvas_story::document(ui, view, &document_name);
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
    let selection = PathSelection::default();
    let names = BezierNames::of(ui);
    let mut shown = BezierOverlay::new((salt, "paths"), &host.paths, &selection, &px, &px, names).tool(BezierTool::Show);
    if let Some(options) = options {
        shown = shown.path_options(options);
    }
    let _ = shown.show(ui, &canvas.response, &canvas.view);
    let undo = host.draft.is_some() && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z));
    let ends = host.ends();
    let names = PenNames::of(ui);
    let mut pen = PenDraft::new(salt, host.draft.as_ref(), names).mode(mode).auto_add_delete(auto).ends(&ends).units(&px, &px).remove_last(undo);
    if let Some(options) = options {
        pen = pen.path_options(options);
    }
    let response = pen.show(ui, &canvas.response, &canvas.view);
    host.apply(&response);
}
