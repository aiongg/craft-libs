//! The stop rules of a gradient bar, as pure functions the bar and an on-canvas gradient line
//! share. Every edit returns the new stops and the index of the stop it touched, so a selection
//! follows a stop across reorders; `None` when the rules refuse the edit.
//!
//! Stops keep their own midpoints and tags when they move. Locations are clamped to 0..=1; the
//! bar rounds pointer locations with [`round_location`] before it gives them to the rules, so a
//! typed location keeps its decimals.

use std::ops::RangeInclusive;

use craft_theme::Srgba;

use super::editor::StopRef;
use super::value::{ColorStop, GradientStops, OpacityStop, StopRow, StopRows};

/// Gives the colour, with alpha, of a gradient at a location 0..=1. Draws the ramp and colours
/// new stops; the default is [`default_sampler`].
pub type Sampler = dyn Fn(&GradientStops, f32) -> Srgba;

/// What a host allows of the stops of one row.
#[derive(Clone, Debug, PartialEq)]
pub struct StopRules {
    /// The row layout.
    pub rows: StopRows,
    /// Fewest stops a row keeps (PhotoCraft 2, EffectCraft 1).
    pub min: usize,
    /// Most stops a row holds (PhotoCraft 1024, its Gradient Map 64).
    pub max: usize,
    /// The range a midpoint may take (PhotoCraft 0.05..=0.95; InDesign and Illustrator 0.13..=0.87).
    pub midpoint: RangeInclusive<f32>,
}

impl Default for StopRules {
    fn default() -> Self {
        Self { rows: StopRows::Split, min: 2, max: 1024, midpoint: 0.05..=0.95 }
    }
}

/// `location` clamped to 0..=1 and rounded to 0.001, as pointer locations are (PhotoCraft's
/// Gradient Map).
pub fn round_location(location: f32) -> f32 {
    (clamp_location(location) * 1000.0).round() / 1000.0
}

/// `location` clamped to 0..=1; 0 when it isn't a number.
fn clamp_location(location: f32) -> f32 {
    if location.is_finite() { location.clamp(0.0, 1.0) } else { 0.0 }
}

/// What the two stop types have in common for the rules.
pub(crate) trait Stop: Clone + PartialEq {
    fn location(&self) -> f32;
    fn set_location(&mut self, location: f32);
    fn midpoint(&self) -> f32;
    fn set_midpoint(&mut self, midpoint: f32);
    fn tag(&self) -> u64;
}

impl Stop for ColorStop {
    fn location(&self) -> f32 {
        self.location
    }
    fn set_location(&mut self, location: f32) {
        self.location = location;
    }
    fn midpoint(&self) -> f32 {
        self.midpoint
    }
    fn set_midpoint(&mut self, midpoint: f32) {
        self.midpoint = midpoint;
    }
    fn tag(&self) -> u64 {
        self.tag
    }
}

impl Stop for OpacityStop {
    fn location(&self) -> f32 {
        self.location
    }
    fn set_location(&mut self, location: f32) {
        self.location = location;
    }
    fn midpoint(&self) -> f32 {
        self.midpoint
    }
    fn set_midpoint(&mut self, midpoint: f32) {
        self.midpoint = midpoint;
    }
    fn tag(&self) -> u64 {
        self.tag
    }
}

/// Whether stop `k` of `new` is stop `i` of `old`: both exist and carry the same tag. A gradient
/// line's carried-on drag finds its stop by this rule. Stops all have tag 0 unless the host sets
/// tags, so stops an equal-length reorder swapped can't be told apart; a host that reorders stops
/// from another control during a drag should tag them.
pub(crate) fn same_stop<T: Stop>(old: &[T], i: usize, new: &[T], k: usize) -> bool {
    matches!((old.get(i), new.get(k)), (Some(a), Some(b)) if a.tag() == b.tag())
}

/// A stop of a row as an operation names it, found again on the stops of the operation's turn
/// ([`find`](StopKey::find)), which another control may have changed since: a stop with a tag by
/// its tag, an untagged stop by its location (or, when another control moved it, as
/// [`find`](StopKey::find) describes).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StopKey {
    pub row: StopRow,
    pub tag: u64,
    /// The stop's location when the key was made.
    pub location: f32,
    /// The locations of the row's untagged stops when the key was made.
    pub untagged: Vec<f32>,
    /// The locations of the untagged stops next to this one in the row, when the key was made.
    pub neighbours: [Option<f32>; 2],
    /// Which of the untagged stops at `location` the stop was, in row order: stops can share a
    /// location.
    pub nth: usize,
}

impl StopKey {
    /// The key of stop `i` of `row` (of [`GradientStops::opacity_stops`] for the opacity row);
    /// `None` when there is no such stop.
    pub fn of(stops: &GradientStops, row: StopRow, i: usize) -> Option<Self> {
        match row {
            StopRow::Color => Self::in_row(&stops.color, row, i),
            StopRow::Opacity => Self::in_row(&stops.opacity_stops(), row, i),
        }
    }

    fn in_row<T: Stop>(list: &[T], row: StopRow, i: usize) -> Option<Self> {
        let stop = list.get(i)?;
        let (tag, location) = (stop.tag(), stop.location());
        let untagged = list.iter().filter(|s| s.tag() == 0).map(Stop::location).collect();
        let nth = list[..i].iter().filter(|s| s.tag() == 0 && s.location() == location).count();
        let before = list[..i].iter().rev().find(|s| s.tag() == 0).map(Stop::location);
        let after = list[i + 1..].iter().find(|s| s.tag() == 0).map(Stop::location);
        let neighbours = [before, after];
        Some(Self { row, tag, location, untagged, neighbours, nth })
    }

    /// The stop's index in `stops`:
    /// - with a tag only one stop of the row has, that stop; with a tag several have (an Alt-drag
    ///   copy keeps its stop's tag), the one of them at `location`;
    /// - untagged, the untagged stop at `location` (the `nth` while as many share it as did);
    ///   with none there, the one untagged stop whose location isn't in `untagged`: another
    ///   control moved it. That holds only while the row has as many untagged stops as before
    ///   and the stop's untagged neighbours are still at their locations: otherwise the stop is
    ///   gone, or the stop that moved is another one.
    ///
    /// `None` when the row can't tell which stop it is: an operation on it is dropped rather than
    /// applied to another stop.
    pub fn find(&self, stops: &GradientStops) -> Option<usize> {
        match self.row {
            StopRow::Color => self.find_in(&stops.color),
            StopRow::Opacity => self.find_in(&stops.opacity_stops()),
        }
    }

    fn find_in<T: Stop>(&self, list: &[T]) -> Option<usize> {
        let only = |found: Vec<usize>| match found[..] {
            [i] => Some(i),
            _ => None,
        };
        let with_tag = |i: &usize| list[*i].tag() == self.tag;
        let here = |i: &usize| list[*i].location() == self.location;
        if self.tag != 0 {
            let tagged: Vec<usize> = (0..list.len()).filter(with_tag).collect();
            return only(tagged.clone()).or_else(|| only(tagged.into_iter().filter(here).collect()));
        }
        let at: Vec<usize> = (0..list.len()).filter(with_tag).filter(here).collect();
        if !at.is_empty() {
            let were = self.untagged.iter().filter(|&&l| l == self.location).count();
            return at.get(self.nth).copied().filter(|_| at.len() == were);
        }
        let untagged_now: Vec<f32> = list.iter().filter(|s| s.tag() == 0).map(Stop::location).collect();
        let neighbours_stay = self.neighbours.iter().flatten().all(|l| untagged_now.contains(l));
        if untagged_now.len() != self.untagged.len() || !neighbours_stay {
            return None;
        }
        only((0..list.len()).filter(with_tag).filter(|&i| !self.untagged.contains(&list[i].location())).collect())
    }
}

/// What a gradient bar did to the stops, as an operation on them ([`apply`]): what a bar's input
/// that waits for its turn applies on the stops of that turn.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BarOp {
    /// Moves a stop by this much of the bar (Left and Right).
    Nudge { at: StopKey, by: f32 },
    /// Moves the midpoint after stop `left` of `row` by this much of its segment.
    NudgeMidpoint { row: StopRow, left: StopKey, by: f32 },
    /// Adds a stop to `row` halfway after stop `after`, or across the row's widest gap
    /// ([`halfway`]): Insert.
    Add { row: StopRow, after: Option<StopKey> },
    /// Adds a stop to `row` at a location: a click on empty space.
    AddAt { row: StopRow, location: f32 },
    /// Removes a stop: Delete.
    Remove(StopKey),
}

/// `op` applied to `stops`, keeping `selected` on its marker ([`keeping`]); an added stop is
/// selected. `None` when the rules refuse it or its stop can't be found ([`StopKey::find`]).
pub(crate) fn apply(
    op: &BarOp,
    stops: &GradientStops,
    selected: &mut Option<StopRef>,
    rules: &StopRules,
    sampler: &Sampler,
) -> Option<GradientStops> {
    match op {
        BarOp::Nudge { at, by } => {
            let (row, i) = (at.row, at.find(stops)?);
            keeping(stops, selected, Touch::Stop(row, i), |s| move_to(s, row, i, location(s, row, i) + by, rules))
        }
        BarOp::NudgeMidpoint { row, left, by } => {
            let (row, i) = (*row, left.find(stops)?);
            keeping(stops, selected, Touch::Stop(row, i), |s| set_midpoint(s, row, i, midpoint(s, row, i) + by, rules))
        }
        BarOp::Add { row, after } => {
            let after = match after {
                Some(key) => Some(key.find(stops)?),
                None => None,
            };
            keeping(stops, selected, Touch::Adds(*row), |s| add(s, *row, halfway(s, *row, after), sampler, rules))
        }
        BarOp::AddAt { row, location } => keeping(stops, selected, Touch::Adds(*row), |s| add(s, *row, *location, sampler, rules)),
        BarOp::Remove(at) => {
            let (row, i) = (at.row, at.find(stops)?);
            keeping(stops, selected, Touch::Stop(row, i), |s| remove(s, row, i, rules))
        }
    }
}

/// The marker an edit of the stops touches ([`keeping`]).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Touch {
    /// Stop `i` of a row: moved, removed, or one of its fields or its midpoint edited.
    Stop(StopRow, usize),
    /// A stop the edit adds to a row.
    Adds(StopRow),
}

/// `edit` applied to `stops`, with `selected` kept on its marker. `edit` returns the new stops
/// and the index of the stop that stands for the touched one: where it moved, the one that takes
/// a removed stop's index, or the added stop. A selection on the touched stop or its midpoint
/// goes there and an added stop is selected; any other selection is found again by its stop's
/// key ([`StopKey::find`]), and lost when it can't be.
pub(crate) fn keeping(
    stops: &GradientStops,
    selected: &mut Option<StopRef>,
    touch: Touch,
    edit: impl FnOnce(&GradientStops) -> Option<(GradientStops, usize)>,
) -> Option<GradientStops> {
    let (out, j) = edit(stops)?;
    let kept = match (touch, *selected) {
        (Touch::Adds(row), _) => Some(StopRef::stop(row, j)),
        (_, None) => None,
        (Touch::Stop(row, i), Some(m)) if m.row() == row && m.index() == i => Some(m.with_index(j)),
        (_, Some(m)) => StopKey::of(stops, m.row(), m.index()).and_then(|key| key.find(&out)).map(|k| m.with_index(k)),
    };
    *selected = kept.filter(|m| match *m {
        StopRef::Midpoint(row, i) => i + 1 < out.len(row),
        m => m.index() < out.len(m.row()),
    });
    Some(out)
}

/// The location of stop `i` of `row`.
fn location(stops: &GradientStops, row: StopRow, i: usize) -> f32 {
    match row {
        StopRow::Color => stops.color[i].location,
        StopRow::Opacity => stops.opacity_stops()[i].location,
    }
}

/// The midpoint after stop `i` of `row`.
fn midpoint(stops: &GradientStops, row: StopRow, i: usize) -> f32 {
    match row {
        StopRow::Color => stops.color[i].midpoint,
        StopRow::Opacity => stops.opacity_stops()[i].midpoint,
    }
}

/// Inserts `stop` after the stops at its location. Returns its index.
fn place<T: Stop>(list: &mut Vec<T>, stop: T) -> usize {
    let at = list.partition_point(|s| s.location() <= stop.location());
    list.insert(at, stop);
    at
}

/// Inserts `stop` and resets the midpoint of the segment it splits.
fn insert<T: Stop>(list: &mut Vec<T>, stop: T) -> usize {
    let at = place(list, stop);
    if let Some(before) = at.checked_sub(1) {
        list[before].set_midpoint(0.5);
    }
    at
}

/// The stops to edit in `row`: the opacity list is written out when it was empty and an opacity
/// stop is edited.
fn explicit(stops: &GradientStops, row: StopRow) -> GradientStops {
    match row {
        StopRow::Color => stops.clone(),
        StopRow::Opacity => GradientStops { color: stops.color.clone(), opacity: stops.opacity_stops().into_owned() },
    }
}

fn has_row(rules: &StopRules, row: StopRow) -> bool {
    row == StopRow::Color || rules.rows == StopRows::Split
}

/// A new stop at `location` in `row`, taking the gradient's colour or opacity there from
/// `sampler`, selected by its index. The segment it splits gets its midpoint reset to 0.5. Under
/// [`StopRows::Split`] a colour stop is opaque.
pub fn add(stops: &GradientStops, row: StopRow, location: f32, sampler: &Sampler, rules: &StopRules) -> Option<(GradientStops, usize)> {
    if !has_row(rules, row) || stops.len(row) >= rules.max {
        return None;
    }
    let location = clamp_location(location);
    let sampled = sampler(stops, location);
    let mut out = explicit(stops, row);
    let at = match row {
        StopRow::Color => {
            let color = if rules.rows == StopRows::Split { Srgba { a: 1.0, ..sampled } } else { sampled };
            insert(&mut out.color, ColorStop::new(location, color))
        }
        StopRow::Opacity => insert(&mut out.opacity, OpacityStop::new(location, sampled.a)),
    };
    Some((out, at))
}

/// Moves stop `i` of `row` to `location`. It may pass its neighbours; it keeps its place among
/// stops at its new location, on the side it came from, so a stop that doesn't move or only
/// reaches a neighbour's location never swaps with it. Returns its new index.
pub fn move_to(stops: &GradientStops, row: StopRow, i: usize, location: f32, rules: &StopRules) -> Option<(GradientStops, usize)> {
    if !has_row(rules, row) || i >= stops.len(row) {
        return None;
    }
    let location = clamp_location(location);
    let mut out = explicit(stops, row);
    let at = match row {
        StopRow::Color => relocate(&mut out.color, i, location),
        StopRow::Opacity => relocate(&mut out.opacity, i, location),
    };
    Some((out, at))
}

fn relocate<T: Stop>(list: &mut Vec<T>, i: usize, location: f32) -> usize {
    let mut stop = list.remove(i);
    let from = stop.location();
    stop.set_location(location);
    // Moving up it stops before the stops at its new location, moving down after them.
    let at = match location > from {
        true => list.partition_point(|s| s.location() < location),
        false if location == from => i,
        false => list.partition_point(|s| s.location() <= location),
    };
    list.insert(at, stop);
    at
}

/// Removes stop `i` of `row`, never below `rules.min`. Returns the index of the stop that takes
/// its place: the next, or the last when it was the last.
pub fn remove(stops: &GradientStops, row: StopRow, i: usize, rules: &StopRules) -> Option<(GradientStops, usize)> {
    let len = stops.len(row);
    if !has_row(rules, row) || i >= len || len <= rules.min {
        return None;
    }
    let mut out = explicit(stops, row);
    match row {
        StopRow::Color => drop(out.color.remove(i)),
        StopRow::Opacity => drop(out.opacity.remove(i)),
    }
    Some((out, i.min(len - 2)))
}

/// A copy of stop `i` of `row` at `location` (an Alt-drag), with its colour or opacity, midpoint
/// and tag. The segment it splits gets its midpoint reset to 0.5. Returns the copy's index.
pub fn copy(stops: &GradientStops, row: StopRow, i: usize, location: f32, rules: &StopRules) -> Option<(GradientStops, usize)> {
    if !has_row(rules, row) || i >= stops.len(row) || stops.len(row) >= rules.max {
        return None;
    }
    let location = clamp_location(location);
    let mut out = explicit(stops, row);
    let at = match row {
        StopRow::Color => insert(&mut out.color, ColorStop { location, ..stops.color[i] }),
        StopRow::Opacity => {
            let source = out.opacity[i];
            insert(&mut out.opacity, OpacityStop { location, ..source })
        }
    };
    Some((out, at))
}

/// Sets the midpoint after stop `i` of `row`, clamped to `rules.midpoint`. The last stop has none.
pub fn set_midpoint(stops: &GradientStops, row: StopRow, i: usize, midpoint: f32, rules: &StopRules) -> Option<(GradientStops, usize)> {
    if !has_row(rules, row) || i + 1 >= stops.len(row) {
        return None;
    }
    let m = if midpoint.is_finite() { midpoint.clamp(*rules.midpoint.start(), *rules.midpoint.end()) } else { 0.5 };
    let mut out = explicit(stops, row);
    match row {
        StopRow::Color => out.color[i].midpoint = m,
        StopRow::Opacity => out.opacity[i].midpoint = m,
    }
    Some((out, i))
}

/// The location halfway between the selected stop of `row` and the next, or the one before for
/// the last stop; with nothing selected, across the widest gap between stops. A row with one
/// stop is halved towards its farther end of the bar, and an empty row at 0.5.
pub fn halfway(stops: &GradientStops, row: StopRow, selected: Option<usize>) -> f32 {
    let at: Vec<f32> = match row {
        StopRow::Color => stops.color.iter().map(|s| s.location).collect(),
        StopRow::Opacity => stops.opacity_stops().iter().map(|s| s.location).collect(),
    };
    let middle = |a: f32, b: f32| round_location((a + b) / 2.0);
    match (at.as_slice(), selected) {
        ([], _) => 0.5,
        ([only], _) => middle(*only, if *only < 0.5 { 1.0 } else { 0.0 }),
        (at, Some(i)) if i < at.len() => {
            if i + 1 < at.len() {
                middle(at[i], at[i + 1])
            } else {
                middle(at[i - 1], at[i])
            }
        }
        (at, _) => {
            let widest = (0..at.len() - 1).fold(0, |best, i| if at[i + 1] - at[i] > at[best + 1] - at[best] { i } else { best });
            middle(at[widest], at[widest + 1])
        }
    }
}

/// The mix factor at `location` between the stops of `list` it lies between, with the segment's
/// midpoint bent to half: `(from, to, mix)`.
fn segment<T: Stop>(list: &[T], location: f32) -> (usize, usize, f32) {
    let last = list.len() - 1;
    if location <= list[0].location() {
        return (0, 0, 0.0);
    }
    for (i, pair) in list.windows(2).enumerate() {
        let (a, b) = (&pair[0], &pair[1]);
        if location <= b.location() {
            let u = (location - a.location()) / (b.location() - a.location()).max(1e-6);
            let m = a.midpoint().clamp(0.01, 0.99);
            let v = if u < m { 0.5 * u / m } else { 0.5 + 0.5 * (u - m) / (1.0 - m) };
            return (i, i + 1, v);
        }
    }
    (last, last, 0.0)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The gradient at `location` with straight lines in encoded sRGB between stops, each segment
/// bent so its midpoint gives the half mix. Under [`StopRows::Split`] the alpha comes from the
/// opacity stops (opaque when there are none) and the colours' own alpha is ignored; under
/// [`StopRows::Combined`] it is the colours' alpha. No colour stops give opaque black.
pub fn sample(stops: &GradientStops, rows: StopRows, location: f32) -> Srgba {
    if stops.color.is_empty() {
        return Srgba::BLACK;
    }
    let (i, j, t) = segment(&stops.color, location);
    let (a, b) = (stops.color[i].color, stops.color[j].color);
    let mut out = Srgba::new(mix(a.r, b.r, t), mix(a.g, b.g, t), mix(a.b, b.b, t), mix(a.a, b.a, t));
    if rows == StopRows::Split {
        out.a = if stops.opacity.is_empty() {
            1.0
        } else {
            let (i, j, t) = segment(&stops.opacity, location);
            mix(stops.opacity[i].opacity, stops.opacity[j].opacity, t)
        };
    }
    out
}

/// The default [`Sampler`] for `rows`: [`sample`].
pub fn default_sampler(rows: StopRows) -> impl Fn(&GradientStops, f32) -> Srgba {
    move |stops, location| sample(stops, rows, location)
}
