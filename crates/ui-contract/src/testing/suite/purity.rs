//! The purity case: drawing code changes no state and sends nothing. The case's fixtures run under the purity probe
//! ([`StoryHarness::probe_purity`]): after the settled build and after each drive the story builds again with no
//! input ([`StoryHarness::rerun_check`]) and its values are compared. The case drives each story as the `values` case does and reports what
//! the fixture saw.

use super::drive::{Plan, for_input, plan, run, variants};
use super::{ANY, Case, Subject};
use crate::contract::Probe;
use crate::testing::story::Story;
use crate::testing::story_run::{Impure, StoryHarness};

pub(super) const CASES: &[Case] = &[Case::new("measure-pure", ANY, drawing_changes_nothing)];

fn describe(story: &str, impure: &Impure) -> String {
    if impure.measuring {
        format!("{story}: measuring run changed {:?} / sent {:?}", impure.changed, impure.effects)
    } else {
        format!("{story}: rerun pass {} changed {:?}", impure.pass, impure.changed)
    }
}

fn first_violation(story: &str, fixture: &StoryHarness) -> Option<String> {
    fixture.impure().first().map(|impure| describe(story, impure))
}

/// A fixture of `story` under the purity probe, settled for the drives it takes.
fn probed(story: &Story) -> StoryHarness {
    let mut fixture = StoryHarness::story(story);
    fixture.probe_purity();
    let mut fixture = for_input(fixture);
    fixture.rerun_check();
    fixture
}

/// The `measure-pure` case over one story of the type `probe` describes: settled and then driven by each way of its
/// drive, the story shows no impure pass.
pub fn measure_pure_case(probe: &Probe, story: &Story) -> Result<(), String> {
    let fixture = probed(story);
    let plan = plan(story, probe, &fixture);
    pure_from(probe, story, &fixture, plan)
}

/// The `measure-pure` case for `story`, whose probed settled fixture is `settled_fixture` and whose drive is `plan`.
fn pure_from(probe: &Probe, story: &Story, settled_fixture: &StoryHarness, plan: Plan) -> Result<(), String> {
    if let Some(failure) = first_violation(story.name, settled_fixture) {
        return Err(failure);
    }
    for variant in variants(plan, probe) {
        let mut fixture = probed(story);
        run(&mut fixture, plan, variant);
        fixture.rerun_check();
        if let Some(failure) = first_violation(story.name, &fixture) {
            return Err(failure);
        }
    }
    Ok(())
}

fn drawing_changes_nothing(subject: &Subject) -> Result<(), String> {
    let failures: Vec<String> =
        subject.stories.iter().filter_map(|story| pure_from(subject.probe, story, &probed(story), subject.base(story).plan).err()).collect();
    if failures.is_empty() { Ok(()) } else { Err(failures.join("; ")) }
}
