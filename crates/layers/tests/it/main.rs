//! The integration binary of craft-layers: every test is a registered trial.

#[cfg(spike_full)]
pub(crate) mod blend_rows;
#[cfg(spike_full)]
pub(crate) mod comps;
#[cfg(spike_full)]
pub(crate) mod panel_drag_menu;
#[cfg(spike_full)]
pub(crate) mod panel_eye;
#[cfg(spike_full)]
pub(crate) mod panel_filter;
#[cfg(spike_full)]
pub(crate) mod panel_input;
#[cfg(spike_full)]
pub(crate) mod panel_memo;
#[cfg(spike_full)]
pub(crate) mod panel_order;
#[cfg(spike_full)]
pub(crate) mod panel_reveal;
#[cfg(spike_full)]
pub(crate) mod panel_rows;

#[cfg(spike_full)]
craft_ui::registry!();

// The spike build (G1) compiles none of this egui-based harness.
#[cfg(not(spike_full))]
fn main() {}

#[cfg(spike_full)]
fn main() {
    craft_ui::testing::run(module_path!(), env!("CARGO_PKG_NAME"), &craft_ui::testing::Registry::of(&[register, craft_layers::testing::register]));
}
