use anyhow::Result;
use crate::cmd::smune::TideAction;
use crate::captain::tide::TideCharts;
pub fn handle_tide(action: TideAction) -> Result<()> {
    let mut charts = TideCharts::new()?;
    match action {
        TideAction::Show => {
            charts.show_interactive()?;
        }
        TideAction::Analyze => {
            charts.analyze_dependencies()?;
        }
        TideAction::Export { path } => {
            charts.export_csv(&path)?;
        }
    }
    Ok(())
}