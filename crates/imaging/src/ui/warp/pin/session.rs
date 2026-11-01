//! [`PinWarpSession`]: the pins of a Puppet Warp while it is open, and the [`PinSolver`] the app
//! plugs in (task TR-T6).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T6)")]

use kurbo::Point;

use craft_ui::dialog::LocalHistory;
use craft_ui::edit::EditEvent;

/// One pin: where it attaches on the rest mesh and where it holds that point now, in document
/// units (PhotoCraft `PuppetPin { src, dst, rotate, depth }`, VectorCraft `rest`, `at`, `angle`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pin {
    pub rest: Point,
    pub at: Point,
    /// The fixed rotation of the mesh around the pin, in degrees; `None` is Auto (LC15).
    pub angle: Option<f64>,
    /// Where the mesh overlaps itself, triangles nearest a higher pin draw on top (LC16).
    pub depth: i32,
}

/// How stiff the mesh is (Photoshop's Mode).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PinMode {
    Rigid,
    #[default]
    Normal,
    Distort,
}

/// How many points the mesh has (Photoshop's Density).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PinDensity {
    Fewer,
    #[default]
    Normal,
    More,
}

/// What the solver reads besides the pins: mode, density and expansion.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PinWarpOptions {
    pub mode: PinMode,
    pub density: PinDensity,
    /// Grows (or, negative, shrinks) the mesh beyond the opaque pixels, in document pixels; 2 by
    /// default, −50 to 100 in the bar (PhotoCraft).
    pub expansion: f64,
}

impl Default for PinWarpOptions {
    fn default() -> Self {
        Self { mode: PinMode::default(), density: PinDensity::default(), expansion: 2.0 }
    }
}

/// Puppet Warp's persisted section: the options and Show Mesh, kept between runs (Photoshop; LC17).
/// Unknown fields are ignored, missing ones take their defaults.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PinWarpSettings {
    #[serde(flatten)]
    pub options: PinWarpOptions,
    /// Show Mesh; on by default.
    pub show_mesh: bool,
}

impl Default for PinWarpSettings {
    fn default() -> Self {
        Self { options: PinWarpOptions::default(), show_mesh: true }
    }
}

/// The app's pin-warp solver (PhotoCraft's `PuppetSolver`, VectorCraft's ARAP): the session calls
/// it whenever the pins or the options change, and the overlay and the preview read its mesh. The
/// solver itself stays in the app until the engine step (step 7 plan, question 1).
pub trait PinSolver {
    /// Pins were added or removed, or a pin's rotation was fixed or freed: rebuild what depends on
    /// the pin set, then solve.
    fn rebind(&mut self, pins: &[Pin], options: &PinWarpOptions);

    /// Pin positions, angles or depths changed: solve again from the rest mesh, never warm-started,
    /// so the result does not depend on the pointer's history.
    fn solve(&mut self, pins: &[Pin], options: &PinWarpOptions);

    /// Density or expansion changed: rebuild the mesh. `Err` keeps the previous mesh; the text is
    /// shown as a notice ("Could not use Puppet Warp: no opaque pixels at this density and
    /// expansion").
    fn remesh(&mut self, options: &PinWarpOptions) -> Result<(), String>;

    /// The mesh as it is now.
    fn mesh(&self) -> PinMeshView<'_>;
}

/// The solver's mesh, lent for one pass: the rest and deformed vertices, the triangles, and the
/// order to draw the triangles in (back to front).
#[derive(Clone, Copy, Debug)]
pub struct PinMeshView<'a> {
    pub rest: &'a [Point],
    pub deformed: &'a [Point],
    pub triangles: &'a [[u32; 3]],
    pub order: &'a [u32],
}

impl PinMeshView<'_> {
    /// The rest position of the deformed mesh's point `at`, where a new pin attaches: the first
    /// triangle containing `at` searched from the top of the draw order, by barycentric
    /// coordinates (degenerate triangles skipped). `None` off the mesh (LC18).
    pub fn rest_of(&self, at: Point) -> Option<Point> {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:165 (`rest_of`).
        todo!()
    }

    /// Every triangle edge once, as vertex index pairs (smaller first), for the mesh painter.
    pub fn edges(&self) -> Vec<[u32; 2]> {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:368 (each edge once).
        todo!()
    }

    /// The middle of the rest mesh's bounds: where Insert places a pin when the pointer is
    /// outside the canvas. `None` for an empty mesh.
    pub fn centre(&self) -> Option<Point> {
        // new: the keyboard path of LC14 (Insert).
        todo!()
    }
}

/// An edit of the pin session, from [`PinWarp`](super::PinWarp) or [`PinWarpBar`](super::PinWarpBar).
#[derive(Clone, Debug, PartialEq)]
pub enum PinEdit {
    /// The pins as a whole: a drag (Begin, Update, Commit, Cancel) or a one-shot change (Set).
    /// Placing a pin begins a drag whose value already holds the new pin, so the placement and the
    /// drag that follows are one step; moving several selected pins, an ⌥-drag turn, a delete, a
    /// depth or angle change and Remove All Pins all report the new pin list.
    Pins(EditEvent<Vec<Pin>>),
    /// The selection, as indices into the pins. Not an undo step.
    Select(Vec<usize>),
    /// Mode, density or expansion. Density and expansion remesh; a refused remesh restores the
    /// previous options.
    Options(PinWarpOptions),
    /// Show Mesh. Not an undo step.
    ShowMesh(bool),
}

/// The pins of an open Puppet Warp, their selection, the options and the session's undo steps.
///
/// - Changes only through [`apply`](Self::apply), which also tells the solver: a new pin count or
///   a pin's rotation fixed or freed is a [`rebind`](PinSolver::rebind), any other pin change a
///   [`solve`](PinSolver::solve), density or expansion a [`remesh`](PinSolver::remesh).
/// - One undo step per drag that moved something, per one-shot change, per run of arrow nudges
///   within the [nudge pause](Self::nudge_pause). Undo and Redo keep the selection where the
///   selected pins still exist (PhotoCraft `restore`).
#[derive(Clone, Debug)]
pub struct PinWarpSession {
    pins: Vec<Pin>,
    selected: Vec<usize>,
    options: PinWarpOptions,
    show_mesh: bool,
    history: LocalHistory<Vec<Pin>>,
    before: Option<Vec<Pin>>,
    nudge_pause: Option<f64>,
    last_nudge: Option<f64>,
}

impl PinWarpSession {
    /// A session with no pins and `settings`' options.
    pub fn new(settings: &PinWarpSettings) -> Self {
        Self::from_pins(Vec::new(), settings)
    }

    /// A session on pins the app kept (VectorCraft's pins live in the document).
    pub fn from_pins(pins: Vec<Pin>, settings: &PinWarpSettings) -> Self {
        Self {
            history: LocalHistory::new(&pins),
            pins,
            selected: Vec::new(),
            options: settings.options,
            show_mesh: settings.show_mesh,
            before: None,
            nudge_pause: None,
            last_nudge: None,
        }
    }

    /// The app's Nudge Bundle Pause in seconds (as `TransformSession::nudge_pause`).
    pub fn nudge_pause(mut self, seconds: Option<f64>) -> Self {
        self.nudge_pause = seconds;
        self
    }

    pub fn pins(&self) -> &[Pin] {
        &self.pins
    }

    /// The selected pins' indices.
    pub fn selected(&self) -> &[usize] {
        &self.selected
    }

    pub fn options(&self) -> &PinWarpOptions {
        &self.options
    }

    pub fn show_mesh(&self) -> bool {
        self.show_mesh
    }

    /// The persisted section as the session has it now.
    pub fn settings(&self) -> PinWarpSettings {
        PinWarpSettings { options: self.options, show_mesh: self.show_mesh }
    }

    /// Whether a pin drag is running.
    pub fn dragging(&self) -> bool {
        self.before.is_some()
    }

    /// The pins to commit; `None` when every pin is where it attached and has no turn (a turn of a
    /// whole number of full turns counts as none), so the app skips the command.
    pub fn result(&self) -> Option<&[Pin]> {
        // pc: crates/algo/src/puppet.rs:116 (`is_identity`); crates/ui-egui/src/tools/puppet_ui.rs:236
        // (`commit`).
        todo!()
    }

    /// Applies one edit at `now` (seconds), telling `solver` what changed. `Err` carries a refused
    /// remesh's text for the bar's notice; the options are then as before.
    pub fn apply(&mut self, edit: PinEdit, solver: &mut dyn PinSolver, now: f64) -> Result<(), String> {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:293 (`pointer`), :123 (`remesh_or_restore`),
        // :135 (`rebind`), :143 (`solve`), :86 (`delete_selected_pin`).
        todo!()
    }

    /// Records an undo step when the pins have come to rest changed (as
    /// `TransformSession::settle`).
    pub fn settle(&mut self, now: f64, busy: bool) {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:53 (`record`: one step per drag that moved
        // something).
        todo!()
    }

    /// Steps back one undo step and rebinds `solver`; false while a drag runs or with nothing to
    /// undo.
    pub fn undo(&mut self, solver: &mut dyn PinSolver) -> bool {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:70 (`undo`), :61 (`restore`).
        todo!()
    }

    /// Steps forward one undone step and rebinds `solver`.
    pub fn redo(&mut self, solver: &mut dyn PinSolver) -> bool {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:78 (`redo`).
        todo!()
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
}
