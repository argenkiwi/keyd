use std::path::Path;
use clap::Parser;
use tracing::{info, error};
use tokio::sync::mpsc;
use keyd_rs::keyboard::{Keyboard, KeyEvent};
use keyd_rs::vkbd::uinput::UinputBackend;
use keyd_rs::device::DeviceManager;
use keyd_rs::config;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "/etc/keyd/default.conf")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!("Starting keyd-rs");

    let config_path = Path::new(&args.config);
    if !config_path.exists() {
        error!("Config file not found: {}", args.config);
    }

    let config = match config::parse_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to parse config: {}", e);
            let mut c = keyd_rs::Config::default();
            c.path = args.config;
            c.layers.push(keyd_rs::Layer {
                name: "main".to_string(),
                ..Default::default()
            });
            c
        }
    };

    let vkbd = Box::new(UinputBackend::new("keyd virtual keyboard")?);
    let mut keyboard = Keyboard::new(config, vkbd);
    let mut device_manager = DeviceManager::new()?;

    let (control_tx, mut control_rx) = mpsc::channel(10);
    // Use a temporary path for the socket in test environments if /var/run is not writable
    let socket_path = "/tmp/keyd.socket"; 
    let ipc_server = keyd_rs::ipc::IpcServer::new(socket_path)?;
    tokio::spawn(async move {
        ipc_server.run(control_tx).await;
    });

    info!("Event loop started");

    let (tx, mut rx) = mpsc::channel(100);

    // Initial scan
    let devices = device_manager.scan();
    for dev in devices {
        info!("Monitoring device: {} ({})", dev.name, dev.id);
        spawn_device_task(dev.path.clone(), tx.clone());
    }

    loop {
        tokio::select! {
            Some(ev) = rx.recv() => {
                keyboard.process_event(ev);
                keyboard.check_timeouts();
            }
            Some(msg) = control_rx.recv() => {
                match msg {
                    keyd_rs::ipc::ControlMessage::Eval(exp, reply) => {
                        info!("Received eval request: {}", exp);
                        let _ = reply.send(keyd_rs::ipc::IpcResponse::Success("Evaluated".to_string()));
                    }
                    keyd_rs::ipc::ControlMessage::Reload => {
                        info!("Received reload request");
                    }
                }
            }
            _new_devs = tokio::task::spawn_blocking(move || {
                Vec::<keyd_rs::device::DeviceInfo>::new()
            }) => {
            }
            _ = tokio::time::sleep(Duration::from_millis(10)) => {
                keyboard.check_timeouts();
            }
        }
    }
}

fn spawn_device_task(path: std::path::PathBuf, tx: mpsc::Sender<KeyEvent>) {
    tokio::spawn(async move {
        if let Ok(mut device) = evdev::Device::open(&path) {
            loop {
                if let Ok(events) = device.fetch_events() {
                    for ev in events {
                        if ev.event_type() == evdev::EventType::KEY {
                            if let Some(code) = keyd_rs::device::map_evdev_code(ev.code()) {
                                let key_ev = KeyEvent {
                                    code,
                                    pressed: ev.value() == 1,
                                    timestamp: Instant::now(),
                                };
                                let _ = tx.send(key_ev).await;
                            }
                        }
                    }
                }
                tokio::task::yield_now().await;
            }
        }
    });
}
