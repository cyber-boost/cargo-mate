use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
const FALLBACK_KEY: &str = "145c73704e3471b3eb17e31f7236207e";
const KEY_CACHE_FILE: &str = ".cargo_mate_key_cache";
const MAX_CACHE_AGE: u64 = 24 * 60 * 60;
fn get_cache_file_path() -> PathBuf {
    if let Ok(home) = env::var("HOME") {
        let dir = PathBuf::from(&home).join(".shipwreck").join(".parlor");
        match std::fs::create_dir_all(&dir) {
            Ok(_) => eprintln!("✅ Directory created successfully: {}", dir.display()),
            Err(e) => {
                eprintln!(
                    "❌ Failed to create directory: {} - Error: {}", dir.display(), e
                )
            }
        }
        dir.join(KEY_CACHE_FILE)
    } else {
        let dir = PathBuf::from("/tmp");
        match std::fs::create_dir_all(&dir) {
            Ok(_) => eprintln!("✅ Fallback directory ready: {}", dir.display()),
            Err(e) => {
                eprintln!(
                    "❌ Failed to create fallback directory: {} - Error: {}", dir
                    .display(), e
                )
            }
        }
        dir.join(KEY_CACHE_FILE)
    }
}
fn fetch_key_from_server() -> Option<String> {
    if let Ok(output) = std::process::Command::new("curl")
        .args(&["-s", "--max-time", "5", "https://mate.cargo.do/overboard/key"])
        .output()
    {
        if output.status.success() {
            let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !key.is_empty() && key.len() == 32 {
                return Some(key);
            }
        }
    }
    None
}
fn load_cached_key() -> Option<String> {
    let cache_file = get_cache_file_path();
    if let Ok(content) = fs::read_to_string(&cache_file) {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() >= 2 {
            if let Ok(timestamp) = lines[0].parse::<u64>() {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if now - timestamp < MAX_CACHE_AGE {
                    let key = lines[1].to_string();
                    if !key.is_empty() && key.len() == 32 {
                        return Some(key);
                    }
                }
            }
        }
    }
    None
}
fn save_key_to_cache(key: &str) {
    let cache_file = get_cache_file_path();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let content = format!("{}\n{}", timestamp, key);
    let _ = fs::write(&cache_file, content);
}
pub fn get_protection_key() -> String {
    if let Ok(key) = env::var("CURRENT_KEY") {
        if !key.is_empty() && key.len() == 32 {
            save_key_to_cache(&key);
            env::set_var("CAPTAIN_SOBER", "1");
            env::remove_var("CAPTAIN_DRUNK");
            return key;
        }
    }
    if let Ok(key) = env::var("CARGO_MATE_KEY") {
        if !key.is_empty() && key.len() == 32 {
            save_key_to_cache(&key);
            env::set_var("CAPTAIN_SOBER", "1");
            env::remove_var("CAPTAIN_DRUNK");
            return key;
        }
    }
    if let Ok(key) = env::var("CAPTAIN_KEY") {
        if !key.is_empty() && key.len() == 32 {
            save_key_to_cache(&key);
            env::set_var("CAPTAIN_SOBER", "1");
            env::remove_var("CAPTAIN_DRUNK");
            return key;
        }
    }
    if let Some(key) = fetch_key_from_server() {
        save_key_to_cache(&key);
        env::set_var("CAPTAIN_SOBER", "1");
        env::remove_var("CAPTAIN_DRUNK");
        return key;
    }
    if let Some(key) = load_cached_key() {
        env::set_var("CAPTAIN_CACHE", "1");
        env::remove_var("CAPTAIN_SOBER");
        env::remove_var("CAPTAIN_DRUNK");
        return key;
    }
    env::set_var("CAPTAIN_DRUNK", FALLBACK_KEY);
    env::remove_var("CAPTAIN_SOBER");
    env::remove_var("CAPTAIN_CACHE");
    FALLBACK_KEY.to_string()
}
pub fn is_using_fallback_key() -> bool {
    get_protection_key() == FALLBACK_KEY
}
pub fn is_captain_sober() -> bool {
    env::var("CAPTAIN_SOBER").is_ok()
}
pub fn is_captain_cached() -> bool {
    env::var("CAPTAIN_CACHE").is_ok()
}
pub fn is_captain_drunk() -> bool {
    env::var("CAPTAIN_DRUNK").is_ok()
}
pub fn get_fallback_key() -> &'static str {
    FALLBACK_KEY
}
/// Get captain status as a string
pub fn get_captain_status() -> String {
    if is_captain_sober() {
        "SOBER".to_string()
    } else if is_captain_cached() {
        "CACHED".to_string()
    } else if is_captain_drunk() {
        "DRUNK".to_string()
    } else {
        "UNKNOWN".to_string()
    }
}
/// Get the current key hash for logging/display
pub fn get_key_hash() -> String {
    let key = get_protection_key();
    if key.len() > 8 { format!("{}...", & key[..8]) } else { key }
}