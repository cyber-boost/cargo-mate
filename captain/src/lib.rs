pub mod anchor;
pub mod admin_msg;
pub mod affiliate;
pub mod captain;
pub mod checklist;
pub mod cmd;
pub mod display;
pub mod history;
pub mod journey;
pub mod mutiny;
pub mod probe;
pub mod scrubme;
pub mod scat;
pub mod scrub;
pub mod smart_parser;
pub mod strip;
pub mod sweeping;
pub mod tools;
pub use captain::*;
pub use crate::captain::config::ConfigManager;
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub fn init_wasm() {
    console_log::init_with_level(log::Level::Info).ok();
}
pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}