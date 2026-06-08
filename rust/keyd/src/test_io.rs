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

        assert_eq!(output.events.len(), 2);
        assert_eq!(output.events[0].code, KEYD_LEFT);
        assert_eq!(output.events[0].pressed, 1);
        assert_eq!(output.events[1].code, KEYD_LEFT);
        assert_eq!(output.events[1].pressed, 0);
    }

    #[test]
    fn test_toggle_layer() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg,
            "[ids]\n*\n\n[main]\ncapslock = toggle(nav)\n\n[nav]\nh = left\n"
        ).unwrap();
        let mut kbd = Keyboard::new(cfg);
        let mut output = TestOutput::new();

        // First tap: toggle nav ON → h produces left
        // Second tap: toggle nav OFF → h produces passthrough h
        let events = [
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 0, timestamp: 0 },
            KeyEvent { code: KEYD_H,        pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_H,        pressed: 0, timestamp: 0 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 0, timestamp: 0 },
            KeyEvent { code: KEYD_H,        pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_H,        pressed: 0, timestamp: 0 },
        ];
        kbd.kbd_process_events(&mut output, &events);

        // First h → left (nav active)
        assert_eq!(output.events[0].code, KEYD_LEFT);
        assert_eq!(output.events[0].pressed, 1);
        assert_eq!(output.events[1].code, KEYD_LEFT);
        assert_eq!(output.events[1].pressed, 0);

        // Second h → h (nav deactivated)
        assert_eq!(output.events[2].code, KEYD_H);
        assert_eq!(output.events[2].pressed, 1);
        assert_eq!(output.events[3].code, KEYD_H);
        assert_eq!(output.events[3].pressed, 0);
    }

    #[test]
    fn test_clear_op() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg,
            "[ids]\n*\n\n[main]\ncapslock = toggle(nav)\nx = clear\n\n[nav]\nh = left\n"
        ).unwrap();
        let mut kbd = Keyboard::new(cfg);
        let mut output = TestOutput::new();

        // Toggle nav on, then clear it, then h should be passthrough
        let events = [
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 0, timestamp: 0 },
            KeyEvent { code: KEYD_X,        pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_X,        pressed: 0, timestamp: 0 },
            KeyEvent { code: KEYD_H,        pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_H,        pressed: 0, timestamp: 0 },
        ];
        kbd.kbd_process_events(&mut output, &events);

        // h should be passthrough (nav was cleared)
        let h_events: Vec<_> = output.events.iter().filter(|e| e.code == KEYD_H).collect();
        assert!(!h_events.is_empty(), "expected h key events after clear");
        assert_eq!(h_events[0].code, KEYD_H);
    }

    #[test]
    fn test_overload_tap() {
        // capslock = overload(control, esc): quick tap → ESC
        let mut cfg = Config::new();
        config_parse_string(&mut cfg,
            "[ids]\n*\n\n[main]\ncapslock = overload(control, esc)\n"
        ).unwrap();
        let mut kbd = Keyboard::new(cfg);
        let mut output = TestOutput::new();

        let events = [
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 0, timestamp: 50 },
        ];
        kbd.kbd_process_events(&mut output, &events);

        // The control layer activates briefly on press (leftctrl momentarily down/up),
        // then ESC fires on tap release — matching C's overload() behaviour.
        let codes: Vec<u8> = output.events.iter().map(|e| e.code).collect();
        assert!(codes.contains(&KEYD_ESC), "tap should produce ESC");
        let esc_down = output.events.iter().find(|e| e.code == KEYD_ESC && e.pressed != 0);
        assert!(esc_down.is_some(), "ESC press event must be present");
    }

    #[test]
    fn test_overload_hold() {
        // capslock = overload(control, esc): hold while pressing a → C-a
        let mut cfg = Config::new();
        config_parse_string(&mut cfg,
            "[ids]\n*\n\n[main]\ncapslock = overload(control, esc)\n"
        ).unwrap();
        let mut kbd = Keyboard::new(cfg);
        let mut output = TestOutput::new();

        let events = [
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_A,        pressed: 1, timestamp: 10 },
            KeyEvent { code: KEYD_A,        pressed: 0, timestamp: 20 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 0, timestamp: 30 },
        ];
        kbd.kbd_process_events(&mut output, &events);

        // Control modifier should have been pressed before A.
        let codes: Vec<u8> = output.events.iter().map(|e| e.code).collect();
        assert!(codes.contains(&KEYD_LEFTCTRL), "control should be pressed during hold");
        assert!(codes.contains(&KEYD_A),        "a should be emitted");
        // ESC must NOT appear (no tap fired).
        assert!(!codes.contains(&KEYD_ESC), "esc must not fire on hold");
    }

    #[test]
    fn test_overloadt_tap() {
        // capslock = overloadt(nav, a, 200): released within 200ms → tap action 'a'
        let mut cfg = Config::new();
        config_parse_string(&mut cfg,
            "[ids]\n*\n\n[main]\ncapslock = overloadt(nav, a, 200)\n\n[nav]\n"
        ).unwrap();
        let mut kbd = Keyboard::new(cfg);
        let mut output = TestOutput::new();

        // Release well before the 200ms deadline.
        let events = [
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 0, timestamp: 50 },
        ];
        kbd.kbd_process_events(&mut output, &events);

        let sent: Vec<_> = output.events.iter().map(|e| e.code).collect();
        assert!(sent.contains(&KEYD_A), "tap should produce 'a'");
    }

    #[test]
    fn test_overloadt_timeout() {
        // capslock = overloadt(nav, a, 200): timeout fires → layer activated
        let mut cfg = Config::new();
        config_parse_string(&mut cfg,
            "[ids]\n*\n\n[main]\ncapslock = overloadt(nav, a, 200)\n\n[nav]\nh = left\n"
        ).unwrap();
        let mut kbd = Keyboard::new(cfg);
        let mut output = TestOutput::new();

        // Synthetic timeout tick at t=200, then h while layer active, then capslock release.
        let events = [
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 1, timestamp: 0 },
            // Simulate timeout expiry via synthetic event (code=0 via kbd_process_events).
            // Instead, drive it directly with the timeout injection in kbd_process_events
            // by having the next real event arrive after the deadline.
            KeyEvent { code: KEYD_H,        pressed: 1, timestamp: 300 },
            KeyEvent { code: KEYD_H,        pressed: 0, timestamp: 300 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 0, timestamp: 350 },
        ];
        kbd.kbd_process_events(&mut output, &events);

        let sent: Vec<u8> = output.events.iter().map(|e| e.code).collect();
        // The timeout injection in kbd_process_events will fire at t=200,
        // resolving to layer(nav). Then h → left.
        assert!(sent.contains(&KEYD_LEFT), "after timeout, h should produce left in nav layer");
        assert!(!sent.contains(&KEYD_A),   "tap action must not fire on timeout");
    }

    #[test]
    fn test_swap_layer() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg,
            "[ids]\n*\n\n[main]\ncapslock = layer(nav)\n\n[nav]\nj = swap(vim)\n\n[vim]\nh = left\n"
        ).unwrap();
        let mut kbd = Keyboard::new(cfg);
        let mut output = TestOutput::new();

        // Press capslock (activate nav), press j (swap nav→vim),
        // press h (in vim → left), release all
        let events = [
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 1, timestamp: 0 },
            KeyEvent { code: KEYD_J,        pressed: 1, timestamp: 1 },
            KeyEvent { code: KEYD_J,        pressed: 0, timestamp: 1 },
            KeyEvent { code: KEYD_H,        pressed: 1, timestamp: 2 },
            KeyEvent { code: KEYD_H,        pressed: 0, timestamp: 2 },
            KeyEvent { code: KEYD_CAPSLOCK, pressed: 0, timestamp: 3 },
        ];
        kbd.kbd_process_events(&mut output, &events);

        let h_down: Vec<_> = output.events.iter().filter(|e| e.pressed != 0).collect();
        assert!(!h_down.is_empty());
        assert_eq!(h_down[0].code, KEYD_LEFT, "h should produce left in vim layer after swap");
    }
}
