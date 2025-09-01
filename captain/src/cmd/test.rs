use anyhow::{Result, Context};
use chrono;
pub fn handle_test() -> Result<()> {
    println!("🧪 Running test command that will generate and log an error...");
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    std::fs::create_dir_all(shipwreck.join("errors"))?;
    let error_file = shipwreck.join("errors").join("latest.txt");
    let error_message = format!(
        "🧪 Test Error: This is a deliberate test error from the test command\nTime: {}\nCommand: cm test\nError: Test error - demonstrating error logging functionality\n",
        chrono::Utc::now().to_rfc3339()
    );
    std::fs::write(&error_file, error_message)?;
    println!("📝 Error logged to: {}", error_file.display());
    println!("✅ Test error successfully logged!");
    println!("💡 Now run 'cm view errors' to see this error");
    Ok(())
}