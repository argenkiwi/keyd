use crate::config::*;
use crate::config_impl::*;
use crate::keyboard_types::*;

pub struct TestOutput {
    pub events: Vec<KeyEvent>,
}

impl TestOutput {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }
}

impl Output for TestOutput {
    fn send_key(&mut self, code: u8, state: u8) {
        self.events.push(KeyEvent { code, pressed: state, timestamp: 0 });
    }
    fn on_layer_change(&mut self, _kbd: &Keyboard, _layer_idx: usize, _active: u8) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::*;

    #[test]
    fn test_basic_remapping() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[main]\na = b\n").unwrap();
        let mut kbd = Keyboard::new(cfg);
        let mut output = TestOutput::new();

        let events = [
            KeyEvent { code: KEYD_A, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_A, pressed: 0, timestamp: 0 },
        ];

        kbd.kbd_process_events(&mut output, &events);

        for event in &output.events {
            println!("Output Event: code={}, pressed={}", event.code, event.pressed);
        }

        assert_eq!(output.events.len(), 2);
        assert_eq!(output.events[0].code, KEYD_B);
        assert_eq!(output.events[0].pressed, 1);
        assert_eq!(output.events[1].code, KEYD_B);
        assert_eq!(output.events[1].pressed, 0);
    }

    #[test]
    fn test_layer_switching() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[main]\ncapslock = layer(nav)\n\n[nav]\nh = left\n").unwrap();
        let mut kbd = Keyboard::new(cfg);
        let mut output = TestOutput::new();

        let events = [
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_H, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_H, pressed: 0, timestamp: 0 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 0, timestamp: 0 },
        ];

        kbd.kbd_process_events(&mut output, &events);

        for event in &output.events {
            println!("Output Event: code={}, pressed={}", event.code, event.pressed);
        }

        assert_eq!(output.events.len(), 2);
        assert_eq!(output.events[0].code, KEYD_LEFT);
        assert_eq!(output.events[0].pressed, 1);
        assert_eq!(output.events[1].code, KEYD_LEFT);
        assert_eq!(output.events[1].pressed, 0);
    }
}
