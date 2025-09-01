use anyhow::Result;
use crate::sweeping::{SweepCommands, run_sweep};
pub fn handle_sweep(action: SweepCommands) -> Result<()> {
    run_sweep(action, false)
}