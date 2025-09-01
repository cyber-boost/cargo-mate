use anyhow::Result;
use crate::strip::StripArgs;
pub fn handle_strip_command(args: StripArgs) -> Result<()> {
    crate::strip::handle_strip_command(args)
}