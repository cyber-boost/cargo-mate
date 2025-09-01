use anyhow::Result;
use crate::captain::wtf::WtfAction;
pub fn handle_wtf(action: WtfAction) -> Result<()> {
    crate::captain::wtf::handle_wtf_action(action)
}