//! Story stubs for the step 3 shell part (not registered: registering changes the registry dump
//! and would run the skeleton's `todo!()` bodies). The fill task that lands a component's body
//! moves its story into the component's module and registers it there.
//!
//! Story ids:
//! - `patterns/dock/panel-menu`: a dock group with a panel whose `panel_menu_entries` are
//!   non-empty; the ≡ button and a tab's right click open the same menu.
//! - `patterns/dock/titled-frame`: `FloatingFrame::show_titled` with title drag, auto height and a
//!   dock drop target.
//! - `patterns/dock/arrange`: the Window › Arrange rows and the 2-up to 6-up cells drawn over a
//!   fixed area.
//! - `patterns/dock/workspaces`: `WorkspaceSwitcher` with the built-in set and two saved ones.
//! - `patterns/shell/start`: `StartScreen` with six recent files in Grid and List, two pinned
//!   folders and three templates; and the empty state.
//! - `patterns/shell/notice-center`: three notices, one with Don't Show Again, one error.
//! - `patterns/shell/tour`: a three-step tour over a mock window (target, no target, last step).
//! - `patterns/shell/status-readout`: the readout in a status bar with every item.
//! - `patterns/shell/toolbar-drawer`: a tool rail with the drawer open.
//! - `patterns/canvas/selection-bar`: the bar over a rect selection, above and flipped below.
//! - `patterns/canvas/tool-pill`: the pill at the bottom centre of a canvas area.
