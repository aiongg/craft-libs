//! Stories of the text editor: a caret, a selection and marked text on a canvas.

use std::ops::Range;

use craft_behavior::kurbo::Point;
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Rect, Ui};

use super::{CaretLine, EditKind, HostLine, HostRun, TextEditor, TextEditorHost, TextSelection};

/// A text of lines at `\n` in a monospace layout: char `c` of a line sits at x = 8c, line `l` at y = 16l.
struct MonoHost {
    text: String,
    revision: u64,
}

const CELL: f64 = 8.0;
const LINE: f64 = 16.0;

impl MonoHost {
    fn starts(&self) -> Vec<usize> {
        let mut starts = vec![0];
        starts.extend(self.text.chars().enumerate().filter(|(_, c)| *c == '\n').map(|(at, _)| at + 1));
        starts
    }

    fn line_of(&self, offset: usize) -> usize {
        self.starts().iter().rposition(|start| *start <= offset).unwrap_or(0)
    }

    fn line_text(&self, line: usize) -> &str {
        self.text.split('\n').nth(line).unwrap_or("")
    }
}

impl TextEditorHost for MonoHost {
    fn text(&self) -> &str {
        &self.text
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn replace(&mut self, range: Range<usize>, text: &str, _: EditKind) -> bool {
        let byte = |chars: usize| self.text.char_indices().nth(chars).map_or(self.text.len(), |(at, _)| at);
        let (from, to) = (byte(range.start), byte(range.end));
        self.text.replace_range(from..to, text);
        self.revision += 1;
        true
    }

    fn hit(&mut self, at: Point) -> (usize, usize) {
        let starts = self.starts();
        let line = ((at.y / LINE).floor().max(0.0) as usize).min(starts.len() - 1);
        let width = self.line_text(line).chars().count();
        (starts[line] + ((at.x / CELL).round().max(0.0) as usize).min(width), line)
    }

    fn caret(&mut self, offset: usize, line: Option<usize>) -> Option<CaretLine> {
        let line = line.unwrap_or_else(|| self.line_of(offset));
        let x = (offset - self.starts()[line]) as f64 * CELL;
        Some(CaretLine { top: Point::new(x, line as f64 * LINE), bottom: Point::new(x, (line + 1) as f64 * LINE) })
    }

    fn selection_quads(&mut self, range: Range<usize>) -> Vec<[Point; 4]> {
        let starts = self.starts();
        (self.line_of(range.start)..=self.line_of(range.end))
            .map(|line| {
                let length = self.line_text(line).chars().count();
                let from = range.start.max(starts[line]) - starts[line];
                let to = range.end.min(starts[line] + length) - starts[line];
                let (x0, x1, y0, y1) = (from as f64 * CELL, to as f64 * CELL, line as f64 * LINE, (line + 1) as f64 * LINE);
                [Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)]
            })
            .collect()
    }

    fn underline_segments(&mut self, range: Range<usize>) -> Vec<[Point; 2]> {
        let starts = self.starts();
        let line = self.line_of(range.start);
        let y = (line + 1) as f64 * LINE - 2.0;
        vec![[Point::new((range.start - starts[line]) as f64 * CELL, y), Point::new((range.end - starts[line]) as f64 * CELL, y)]]
    }

    fn lines(&mut self) -> Vec<HostLine> {
        let starts = self.starts();
        (0..starts.len())
            .map(|line| {
                let text = self.line_text(line).to_owned();
                let count = text.chars().count();
                let positions: Vec<f32> = (0..count).map(|at| at as f32 * CELL as f32).collect();
                let rect = Rect::from_min_size(egui::pos2(0.0, line as f32 * LINE as f32), egui::vec2(count as f32 * CELL as f32, LINE as f32));
                HostLine {
                    chars: starts[line]..starts[line] + count,
                    rect,
                    baseline: rect.bottom() - 4.0,
                    runs: vec![HostRun { bytes: 0..text.len(), rtl: false, positions, widths: vec![CELL as f32; count] }],
                    text,
                }
            })
            .collect()
    }

    fn line_range(&mut self, line: usize) -> Range<usize> {
        let starts = self.starts();
        let line = line.min(starts.len() - 1);
        starts[line]..starts[line] + self.line_text(line).chars().count()
    }

    fn line_step(&mut self, offset: usize, lines: i32) -> usize {
        let starts = self.starts();
        let line = self.line_of(offset);
        let target = (line as i32 + lines).clamp(0, starts.len() as i32 - 1) as usize;
        let range = self.line_range(target);
        (starts[target] + (offset - starts[line])).min(range.end)
    }

    fn line_edge(&mut self, offset: usize, end: bool) -> usize {
        let range = self.line_range(self.line_of(offset));
        if end { range.end } else { range.start }
    }
}

/// The editor over `text` with `selection`, and `marked` chars of marked text when given, on a canvas.
fn on_canvas(ui: &mut Ui, data: &mut StoryData, text: &str, selection: TextSelection, marked: Option<Range<usize>>) -> StoryOutput {
    let mut host = MonoHost { text: data.value("text", text.to_owned()).clone(), revision: 0 };
    let selection = data.value("selection", selection);
    let _ = marked;
    let shown = Canvas::new("Document").show(ui, |ui, body| {
        let bounds = Rect::from_min_size(body.rect.min, egui::vec2(240.0, 48.0));
        TextEditor::new("Text", &mut host, Bind::bound(&mut *selection)).show_at(ui, bounds)
    });
    shown.inner.map(StoryOutput::from).unwrap_or_default()
}

#[story(TextEditor<'static>, covers = [Default], drive = Drive::Never)]
fn caret(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, data, "Hello craft\nSecond line", TextSelection::caret_at(5), None)
}

#[story(TextEditor<'static>, covers = [LongText], drive = Drive::Never)]
fn long_text(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, data, &"A line of text that wraps across the frame. ".repeat(10), TextSelection::caret_at(0), None)
}

#[story(TextEditor<'static>, covers = [], drive = Drive::Never)]
fn selection(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, data, "Hello craft\nSecond line", TextSelection { anchor: 6, caret: 14 }, None)
}

#[story(TextEditor<'static>, covers = [], drive = Drive::Never)]
fn composing(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_canvas(ui, data, "Hello かんじ", TextSelection::caret_at(9), Some(6..9))
}
