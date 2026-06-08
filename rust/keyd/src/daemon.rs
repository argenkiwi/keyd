use libc;
use crate::config::*;
use crate::config_impl::*;
use crate::keyboard_types::*;
use crate::device::*;
use crate::vkbd::*;

fn current_time_ms() -> i32 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as i32
}

pub struct Daemon {
    pub vkbd: Vkbd,
    pub keyboards: Vec<Keyboard>,
    pub devices: Vec<Device>,
    pub keystate: [u8; 256],
}

impl Daemon {
    pub fn new() -> Result<Self, String> {
        let vkbd = Vkbd::init("keyd virtual keyboard")?;
        Ok(Daemon {
            vkbd,
            keyboards: Vec::new(),
            devices: Vec::new(),
            keystate: [0; 256],
        })
    }

    pub fn load_config(&mut self, path: &str) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, &content)?;
        let kbd = Keyboard::new(cfg);
        self.keyboards.push(kbd);
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.devices = Device::scan();
        for dev in &mut self.devices {
            if !dev.is_virtual {
                let _ = dev.grab();
            }
        }

        if self.devices.is_empty() {
            return Err("No input devices found".to_string());
        }

        // timeout_ms < 0 means block indefinitely, matching C evloop behaviour.
        let mut timeout_ms: i32 = -1;

        loop {
            let mut pfd = libc::pollfd {
                fd: self.devices[0].fd,
                events: libc::POLLIN,
                revents: 0,
            };

            let poll_timeout = if timeout_ms < 0 { -1i32 } else { timeout_ms };
            let poll_start = current_time_ms();
            unsafe { libc::poll(&mut pfd, 1, poll_timeout) };
            let elapsed = current_time_ms() - poll_start;

            // Handle timeout expiry (drives tap-hold, oneshot, etc.).
            if timeout_ms >= 0 {
                timeout_ms -= elapsed;
                if timeout_ms <= 0 {
                    if !self.keyboards.is_empty() {
                        let mut kbd = self.keyboards.remove(0);
                        let mut output = DaemonOutput {
                            vkbd: &self.vkbd,
                            keystate: &mut self.keystate,
                        };
                        let next = kbd.kbd_process_events(&mut output, &[]);
                        self.keyboards.insert(0, kbd);
                        timeout_ms = if next > 0 { next as i32 } else { -1 };
                    } else {
                        timeout_ms = -1;
                    }
                }
            }

            // Drain all available events from the pipe.
            if pfd.revents & libc::POLLIN != 0 {
                while let Some(event) = self.devices[0].read_event() {
                    if event.event_type == DeviceEventType::Key {
                        let kbd_event = KeyEvent {
                            code: event.code,
                            pressed: event.pressed,
                            timestamp: current_time_ms(),
                        };

                        if !self.keyboards.is_empty() {
                            let mut kbd = self.keyboards.remove(0);
                            let mut output = DaemonOutput {
                                vkbd: &self.vkbd,
                                keystate: &mut self.keystate,
                            };
                            let next = kbd.kbd_process_events(&mut output, &[kbd_event]);
                            self.keyboards.insert(0, kbd);
                            let next_i32 = if next > 0 { next as i32 } else { -1 };
                            timeout_ms = match (timeout_ms, next_i32) {
                                (-1, n) => n,
                                (t, -1) => t,
                                (t, n)  => t.min(n),
                            };
                        }
                    }
                }
            }
        }
    }
}

struct DaemonOutput<'a> {
    vkbd: &'a Vkbd,
    keystate: &'a mut [u8; 256],
}

impl<'a> Output for DaemonOutput<'a> {
    fn send_key(&mut self, code: u8, state: u8) {
        self.keystate[code as usize] = state;
        self.vkbd.send_key(code, state);
    }
    fn on_layer_change(&mut self, _kbd: &Keyboard, _layer_idx: usize, _active: u8) {
    }
}
