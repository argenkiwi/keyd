pub mod keys;
pub mod macro_types;
pub mod config;
pub mod ini;
pub mod unicode;
pub mod macro_parse;
pub mod config_parse;
pub mod config_impl;
pub mod vkbd;
pub mod device;
pub mod keyboard_types;
pub mod keyboard_impl;
pub mod daemon;
pub mod ipc;
#[cfg(target_os = "macos")]
pub mod macos_input;
#[cfg(test)]
pub mod tests;
#[cfg(test)]
pub mod test_io;

use clap::{Parser, Subcommand};
use crate::daemon::Daemon;
use crate::keys::KEYCODE_TABLE;

#[derive(Parser)]
#[command(version = "2.6.0", about = "A key remapping daemon for Linux.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the keyd daemon
    Daemon {
        /// Path to the configuration file
        #[arg(short, long, default_value = "/etc/keyd/default.conf")]
        config: String,
    },
    /// List all valid key names
    ListKeys,
    /// Monitor key events
    Monitor,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Daemon { config }) => {
            let mut daemon = Daemon::new().expect("Failed to initialize daemon");
            daemon.load_config(&config).expect("Failed to load config");
            println!("Starting keyd daemon...");
            daemon.run().expect("Daemon error");
        }
        Some(Commands::ListKeys) => {
            for i in 0..256 {
                let ent = &KEYCODE_TABLE[i];
                if let Some(name) = ent.name {
                    println!("{}", name);
                }
                if let Some(alt) = ent.alt_name {
                    if !alt.is_empty() {
                        println!("{}", alt);
                    }
                }
                if let Some(shifted) = ent.shifted_name {
                    println!("{}", shifted);
                }
            }
        }
        Some(Commands::Monitor) => {
            println!("Monitoring devices (requires root)...");
            let mut devices = crate::device::Device::scan();
            loop {
                for dev in &mut devices {
                    if let Some(ev) = dev.read_event() {
                        if ev.event_type == crate::device::DeviceEventType::Key {
                            let key_name = KEYCODE_TABLE[ev.code as usize].name.unwrap_or("UNKNOWN");
                            println!("device: {}, key: {} ({}), state: {}", dev.name, key_name, ev.code, if ev.pressed != 0 { "down" } else { "up" });
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        None => {
            // Default behavior if no command: print help?
            println!("Use --help for usage information.");
        }
    }
}
