pub struct Log;
impl Log {
    pub fn new() -> Self {
        Log
    }
    pub fn log(&self, message: &str, tags: Vec<String>) -> anyhow::Result<()> {
        use std::fs::{self, OpenOptions};
        use std::io::Write;
        use std::path::PathBuf;
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home_dir.join(".shipwreck");
        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }
        let log_file = config_dir.join("fell-overboard.log");
        if !log_file.exists() {
            let _ = OpenOptions::new().create(true).append(true).open(&log_file);
        }
        let log_entry = if !tags.is_empty() {
            format!("[{}] {}\n", tags.join(","), message)
        } else {
            format!("{}\n", message)
        };
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = file.write_all(log_entry.as_bytes());
        }
        Ok(())
    }
}