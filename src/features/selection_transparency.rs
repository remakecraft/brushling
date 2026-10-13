//! Feature selection.transparency: one shared opaque / transparent setting
//! (`State::options.opaque`), changed from the options box of the selection
//! and text tools (`ToolOption::Opaque`) or the Image > Draw Opaque check
//! item. In transparent mode floating pixels equal to the *current*
//! background colour are neither shown nor committed; the rule itself is
//! `State::draws_floating` (used by the composite layer and by
//! `stamp_selection`, and to be used by the text frame). Because the rule
//! reads the setting at display / commit time, switching mode while a
//! selection floats takes effect immediately.

use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command("Image > Draw Opaque", toggle);
}

fn toggle(st: &mut State) -> HResult<()> {
    let opaque = !st.options.opaque;
    st.set_tool_option(ToolOption::Opaque(opaque))
}
