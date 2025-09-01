use anyhow::Result;
use crate::tools;
use crate::cmd::smune::ToolAction;
pub fn handle_tool(action: ToolAction) -> Result<()> {
    match action {
        ToolAction::List => {
            tools::list_tools();
        }
        ToolAction::Help { name } => {
            tools::show_tool_help(&name);
        }
        ToolAction::Run { name, args } => {
            tools::run_tool(&name, &args)?;
        }
        ToolAction::Execute(args) => {
            if args.is_empty() {
                tools::list_tools();
            } else {
                let tool_name = &args[0];
                let tool_args = &args[1..];
                tools::run_tool(tool_name, tool_args)?;
            }
        }
    }
    Ok(())
}