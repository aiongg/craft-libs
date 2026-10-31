//! Per-command modal rules.

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// With Gate::modal and NAVIGATION_COMMANDS, view.zoomIn is allowed and image.crop is not; the built-in Quit always is.
fn navigation_commands_run_while_a_dialog_is_open() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// With ModalRules::workspace, even the in-dialog commands are refused; Hide and Minimize still run.
fn a_workspace_blocks_every_app_command() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// With a pending rule allowing view.*, the native menu greys image.* and drops its clicks; view.zoomIn stays enabled.
fn a_pending_crop_greys_what_its_rule_refuses() {
    todo!()
}
