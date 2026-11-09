//! The purity case: drawing code changes no state and sends nothing. The fixture runs each story's `show` in a
//! measuring run before every real pass and compares state around the later passes of a frame; the case drives
//! each story as the `values` case does and reports what the fixture saw.

use super::drive::{run, settled, variants};
use super::{ANY, Case, Subject};
use crate::contract::Probe;
use crate::testing::fixture::{Fixture, Impure};
use crate::testing::story::Story;

pub(super) const CASES: &[Case] = &[Case::new("measure-pure", ANY, drawing_changes_nothing)];

fn describe(story: &str, impure: &Impure) -> String {
    if impure.measuring {
        format!("{story}: measuring run changed {:?} / sent {:?}", impure.changed, impure.effects)
    } else {
        format!("{story}: rerun pass {} changed {:?}", impure.pass, impure.changed)
    }
}

fn first_violation(story: &str, fixture: &Fixture) -> Option<String> {
    fixture.impure().first().map(|impure| describe(story, impure))
}

/// The `measure-pure` case over one story of the type `probe` describes: settled and then driven by each way of its
/// drive, the story shows no impure pass.
pub fn measure_pure_case(probe: &Probe, story: &Story) -> Result<(), String> {
    let (fixture, plan) = settled(story, probe);
    if let Some(failure) = first_violation(story.name, &fixture) {
        return Err(failure);
    }
    for variant in variants(plan, probe) {
        let (mut fixture, _) = settled(story, probe);
        run(&mut fixture, plan, variant);
        if let Some(failure) = first_violation(story.name, &fixture) {
            return Err(failure);
        }
    }
    Ok(())
}

fn drawing_changes_nothing(subject: &Subject) -> Result<(), String> {
    let failures: Vec<String> = subject.stories.iter().filter_map(|story| measure_pure_case(subject.probe, story).err()).collect();
    if failures.is_empty() { Ok(()) } else { Err(failures.join("; ")) }
}
