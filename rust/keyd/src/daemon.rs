use crate::config::*;
use crate::config_impl::*;
use crate::keyboard_types::*;
use crate::device::*;
use crate::vkbd::*;

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

        loop {
            for i in 0..self.devices.len() {
                if let Some(event) = self.devices[i].read_event() {
                    if event.event_type == DeviceEventType::Key {
                        let kbd_event = KeyEvent {
                            code: event.code,
                            pressed: event.pressed,
                            timestamp: 0,
                        };
                        
                        if !self.keyboards.is_empty() {
                            let mut kbd = self.keyboards.remove(0);
                            {
                                let mut output = DaemonOutput { 
                                    vkbd: &self.vkbd,
                                    keystate: &mut self.keystate,
                                };
                                kbd.kbd_process_events(&mut output, &[kbd_event]);
                            }
                            self.keyboards.insert(0, kbd);
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
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
