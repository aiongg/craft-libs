//! The list of history states every History panel and undo list shows: when a state is added,
//! merged, dropped or deleted, linear and non-linear history, the state limit, Clear History, and
//! the grouping marks that make a gesture one state (`StepList`, `StepRows`, `StepMark`,
//! `QuietMarks`).

mod group;
#[cfg(test)]
mod group_tests;
mod list;
#[cfg(test)]
mod tests;
