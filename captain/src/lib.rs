pub mod anchor;
pub mod admin_msg;
pub mod affiliate;
pub mod captain;
pub mod captain_log;
pub mod checklist;
pub mod display;
pub mod history;
pub mod journey;
pub mod mutiny;
pub mod optimize;
pub mod parser;
pub mod scrub;
pub mod smart_parser;
pub mod tide;
pub mod tools;
pub mod treasure_map;
pub mod user;
pub mod version;
pub use crate::captain::config::ConfigManager;
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub fn init_wasm() {
    console_log::init_with_level(log::Level::Info).ok();
}
pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}