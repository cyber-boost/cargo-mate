use clap::Subcommand;
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    List,
    Get { key: String },
    Set { key: String, value: String, local: bool },
    Reset,
    Init,
    Shortcut { name: String, command: String, local: bool },
    Hook { hook_type: String, command: String, local: bool },
}
pub mod binary_encryptor;
pub mod captain;
pub mod captain_log;
pub mod captain_status;
pub mod config;
pub mod create_secure_binary;
pub mod create_self_protected_binary;
pub mod encrypt_binaries;
pub mod license_guard;
pub mod license;
pub mod log;
pub mod optimize;
pub mod parser;
pub mod shell_integration;
pub mod tide;
pub mod treasure_map;
pub mod version;
pub mod version_commands;
pub mod wtf;
pub use binary_encryptor::*;
pub use captain::*;
pub use captain_log::*;
pub use captain_status::*;
pub use config::*;
pub use create_secure_binary::*;
pub use create_self_protected_binary::*;
pub use encrypt_binaries::*;
pub use license_guard::*;
pub use license::*;
pub use log::*;
pub use optimize::*;
pub use parser::*;
pub use shell_integration::*;
pub use tide::*;
pub use treasure_map::*;
pub use version::*;
pub use version_commands::*;
pub use wtf::*;