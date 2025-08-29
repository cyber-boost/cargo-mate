use anyhow::Result;
#[derive(Debug, Clone, clap::Subcommand)]
pub enum WtfAction {
    Ask { input: String, #[arg(long)] file: bool },
    #[command(hide = true)]
    Direct { input: String, #[arg(long)] file: bool },
    Er { #[arg(default_value = "10")] count: usize },
    Ollama { #[command(subcommand)] command: OllamaCommand },
    List { #[arg(default_value = "10")] limit: usize },
    Show { id: String },
    History { #[arg(default_value = "10")] limit: usize },
    Checklist { #[arg(default_value = "10")] limit: usize },
    Interactive,
}
#[derive(Debug, Clone, clap::Subcommand)]
pub enum OllamaCommand {
    Enable { #[arg(default_value = "llama2")] model: String },
    Disable,
    Status,
    Models,
}
pub fn handle_wtf_action(_action: WtfAction) -> Result<()> {
    eprintln!("Not implemented: handle_wtf_action");
    Ok(())
}
pub fn action_to_args(_action: &WtfAction) -> Vec<String> {
    eprintln!("Not implemented: action_to_args");
    Vec::new()
}
pub fn handle_wtf(_question: &str, _interactive: bool) -> Result<()> {
    eprintln!("Not implemented: handle_wtf");
    Ok(())
}
pub fn display_api_failure_art() {
    eprintln!("Not implemented: display_api_failure_art");
}