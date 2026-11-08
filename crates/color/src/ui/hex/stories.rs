//! Stories of `HexPairField`: the field of a Web Color slider.

use craft_behavior::PlainText;
use craft_ui::contract::Bind;
use craft_ui::contract::Show as _;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use craft_ui::text::TextField;
use egui::Ui;

use super::{HexPair as _, HexPairField};

#[story(TextField<'static, PlainText>, covers = [], drive = Drive::Never)]
fn hex_pair_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let byte = data.value("byte", 0xA5u8);
    HexPairField::hex_pair("Red", Bind::bound(byte)).show(ui).into()
}
