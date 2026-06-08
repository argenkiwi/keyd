use crate::config::*;
use crate::config_impl::{config_check_match, config_parse};
use crate::keyboard_types::*;
use crate::device::*;
use crate::vkbd::Vkbd;

fn current_time_ms() -> i64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as i64
}

/// Convert device capability flags to config ID flags used by config_check_match.
fn caps_to_id_flags(caps: u8) -> u8 {
    let mut flags: u8 = 0;
    if caps & CAP_KEY != 0       { flags |= ID_KEY; }
    if caps & CAP_KEYBOARD != 0  { flags |= ID_KEYBOARD; }
    if caps & CAP_MOUSE_ABS != 0 { flags |= ID_TRACKPAD; }
    if caps & CAP_MOUSE != 0     { flags |= ID_MOUSE; }
    flags
}

/// Find the best-matching keyboard for `device`, grab it if matched.
/// Returns the index into `keyboards` or None if no config matches.
fn manage_device(keyboards: &[Keyboard], device: &mut Device) -> Option<usize> {
    if device.is_virtual {
        return None;
    }

    let flags = caps_to_id_flags(device.capabilities);
    let mut best_idx: Option<usize> = None;
    let mut best_rank: i32 = 0;

    for (i, kbd) in keyboards.iter().enumerate() {
        let r = config_check_match(&kbd.config, &device.id, flags);
        if r > best_rank {
            best_rank = r;
            best_idx = Some(i);
        }
    }

    // Wildcard (rank=1) must not match mice or trackpads — only real keyboards.
    if best_rank == 1 && !((flags & ID_KEYBOARD != 0) && (flags & ID_TRACKPAD == 0)) {
        best_idx = None;
    }

    if best_idx.is_some() {
        if device.grab().is_err() {
            eprintln!("WARNING: Failed to grab {}", device.path);
            return None;
        }
        eprintln!("DEVICE: match    {}  ({})", device.id, device.name);
    } else {
        let _ = device.ungrab();
        eprintln!("DEVICE: ignoring {}  ({})", device.id, device.name);
    }

    best_idx
}

// ── State that the Output trait implementation needs ───────────────────────

struct OutputState {
    vkbd: Vkbd,
    keystate: [u8; 256],
}

pub struct Daemon {
    output: OutputState,
    pub keyboards: Vec<Keyboard>,
    pub devices: Vec<Device>,
    /// Parallel to `devices`: which keyboard index each device maps to.
    pub device_kbd: Vec<Option<usize>>,
}

// ── Daemon impl ────────────────────────────────────────────────────────────

impl Daemon {
    pub fn new() -> Result<Self, String> {
        let vkbd = Vkbd::init("keyd virtual keyboard")?;
        Ok(Daemon {
            output: OutputState { vkbd, keystate: [0; 256] },
            keyboards: Vec::new(),
            devices: Vec::new(),
            device_kbd: Vec::new(),
        })
    }

    pub fn load_config(&mut self, path: &str) -> Result<(), String> {
        let cfg = config_parse(path)?;
        self.keyboards.push(Keyboard::new(cfg));
        Ok(())
    }

    /// Route events to the keyboard at `kbd_idx`.
    /// Splits `self` into disjoint borrows so the `Output` adapter and the
    /// `Keyboard` can be accessed simultaneously.
    fn dispatch_kbd(&mut self, kbd_idx: usize, events: &[KeyEvent]) -> i64 {
        let (keyboards, out) = (&mut self.keyboards, &mut self.output);
        let mut adapter = DaemonOutput { vkbd: &out.vkbd, keystate: &mut out.keystate };
        keyboards[kbd_idx].kbd_process_events(&mut adapter, events)
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.devices = Device::scan();
        self.device_kbd = self.devices.iter_mut()
            .map(|dev| manage_device(&self.keyboards, dev))
            .collect();

        if self.devices.is_empty() {
            return Err("No input devices found".to_string());
        }

        // timeout_ms < 0 → poll indefinitely; ≥ 0 → fire synthetic timeout event.
        let mut timeout_ms: i64 = -1;

        loop {
            // Build pollfd array for all live devices.
            let pfds_count = self.devices.len();
            let mut pfds: Vec<libc::pollfd> = self.devices.iter()
                .map(|d| libc::pollfd {
                    fd: d.fd,
                    events: libc::POLLIN | libc::POLLERR,
                    revents: 0,
                })
                .collect();

            let poll_timeout = if timeout_ms < 0 {
                -1i32
            } else {
                timeout_ms.min(i32::MAX as i64) as i32
            };

            let poll_start = current_time_ms();
            unsafe {
                libc::poll(
                    pfds.as_mut_ptr(),
                    pfds_count as libc::nfds_t,
                    poll_timeout,
                )
            };
            let now = current_time_ms();
            let elapsed = now - poll_start;

            // Advance / fire pending timeout.
            if timeout_ms >= 0 {
                timeout_ms -= elapsed;
                if timeout_ms <= 0 {
                    // Fire a synthetic (code=0) timeout tick for every keyboard.
                    let mut next: i64 = -1;
                    for ki in 0..self.keyboards.len() {
                        let t = self.dispatch_kbd(ki, &[]);
                        if t > 0 {
                            next = if next < 0 { t } else { next.min(t) };
                        }
                    }
                    timeout_ms = next;
                }
            }

            // Drain events from every ready device.
            for i in 0..pfds_count {
                if pfds[i].revents == 0 {
                    continue;
                }

                let kbd_idx = self.device_kbd[i];

                // Read all queued events from this device.
                loop {
                    let devev = match self.devices[i].read_event() {
                        Some(e) => e,
                        None => break,
                    };

                    match devev.event_type {
                        DeviceEventType::Removed => {
                            eprintln!("DEVICE: removed {}", self.devices[i].path);
                            self.devices[i].fd = -1;
                            break;
                        }
                        DeviceEventType::Key => {
                            if let Some(ki) = kbd_idx {
                                let ev = KeyEvent {
                                    code: devev.code,
                                    pressed: devev.pressed,
                                    timestamp: now as i32,
                                };
                                let next = self.dispatch_kbd(ki, &[ev]);
                                if next > 0 {
                                    timeout_ms = if timeout_ms < 0 {
                                        next
                                    } else {
                                        timeout_ms.min(next)
                                    };
                                }
                            }
                        }
                        // Mouse / LED events handled in later phases.
                        _ => {}
                    }
                }
            }

            // Compact removed devices (fd == -1).
            let mut j = 0;
            for i in 0..self.devices.len() {
                if self.devices[i].fd != -1 {
                    self.devices.swap(i, j);
                    self.device_kbd.swap(i, j);
                    j += 1;
                }
            }
            self.devices.truncate(j);
            self.device_kbd.truncate(j);
        }
    }
}

// ── Output adapter ─────────────────────────────────────────────────────────

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
        // Layer-change notifications (IPC listen) implemented in Phase 10.
    }
}
