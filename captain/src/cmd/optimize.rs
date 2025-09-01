use anyhow::Result;
use crate::captain::optimize;
use crate::cmd::smune::OptimizeAction;
pub fn handle_optimize(action: OptimizeAction) -> Result<()> {
    let optimizer = optimize::BuildOptimizer::new(None)?;
    match action {
        OptimizeAction::Aggressive => {
            optimizer.optimize_build(optimize::OptimizationProfile::Aggressive)?;
        }
        OptimizeAction::Balanced => {
            optimizer.optimize_build(optimize::OptimizationProfile::Balanced)?;
        }
        OptimizeAction::Conservative => {
            optimizer.optimize_build(optimize::OptimizationProfile::Conservative)?;
        }
        OptimizeAction::Custom {
            jobs,
            incremental,
            opt_level,
            debug_level,
            codegen_units,
        } => {
            let incremental_bool = incremental.to_lowercase() == "true";
            let profile = optimize::OptimizationProfile::Custom {
                jobs,
                incremental: incremental_bool,
                opt_level,
                debug_level,
                codegen_units,
            };
            optimizer.optimize_build(profile)?;
        }
        OptimizeAction::Status => {
            optimizer.show_status()?;
        }
        OptimizeAction::Recommendations => {
            optimizer.show_recommendations()?;
        }
        OptimizeAction::Restore => {
            optimizer.restore_backup()?;
        }
    }
    Ok(())
}