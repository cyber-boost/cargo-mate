use anyhow::Result;
use crate::cmd::smune::MutinyAction;
use crate::mutiny::MutinyMode;
pub fn handle_mutiny_internal(action: MutinyAction) -> Result<()> {
    let mut mutiny = MutinyMode::new()?;
    match action {
        MutinyAction::Activate { reason } => {
            mutiny.activate(&reason)?;
        }
        MutinyAction::Deactivate => {
            mutiny.deactivate()?;
        }
        MutinyAction::AllowWarnings => {
            mutiny.allow_warnings()?;
        }
        MutinyAction::SkipTests => {
            mutiny.skip_tests()?;
        }
        MutinyAction::Force => {
            mutiny.force_build()?;
        }
        MutinyAction::Yolo => {
            mutiny.yolo_mode()?;
        }
        MutinyAction::Status => {
            mutiny.status();
        }
    }
    Ok(())
}