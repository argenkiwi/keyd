use crate::config::*;
use crate::macro_types::*;
use crate::keyboard_types::*;
use crate::keys::*;
use crate::macro_parse::macro_parse;
use crate::unicode::unicode_get_sequence;

impl Keyboard {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            cache: [None; 16],
            last_pressed_output_code: 0,
            last_pressed_code: 0,
            oneshot_latch: 0,
            inhibit_modifier_guard: 0,
            macro_play: MacroPlayState { active_idx: None, layer: -1, timeout: 0, repeat_interval: 0 },
            overload_last_layer_code: -1,
            oneshot_timeout: 0,
            overload_start_time: 0,
            last_simple_key_time: 0,
            timeouts: [0; 128],
            nr_timeouts: 0,
            active_chords: Vec::new(),
            chord: ChordState {
                queue: [KeyEvent { code: 0, pressed: 0, timestamp: 0 }; 32],
                queue_sz: 0,
                match_idx: None,
                match_layer: -1,
                start_code: 0,
                last_code_time: 0,
                state: ChordStatus::Inactive,
            },
            pending_timeout: None,
            pending_overload: None,
            layer_state: [LayerState { activation_time: 0, active: 0, toggled: 0, oneshot_depth: 0 }; MAX_LAYERS],
            last_repeatable_action: Descriptor { op: Op::KeySequence, data: DescriptorData::None },
            keystate: [0; 256],
            scroll: ScrollState { x: 0, y: 0, sensitivity: 0, active: 0 },
        }
    }

    fn cache_set(&mut self, code: u8, ent: Option<CacheEntry>) -> bool {
        let mut slot = None;
        for i in 0..16 {
            if let Some(c) = self.cache[i] {
                if c.code == code {
                    slot = Some(i);
                    break;
                }
            } else if slot.is_none() {
                slot = Some(i);
            }
        }

        if let Some(i) = slot {
            if let Some(mut e) = ent {
                e.code = code;
                self.cache[i] = Some(e);
            } else {
                self.cache[i] = None;
            }
            true
        } else {
            false
        }
    }

    fn cache_get(&self, code: u8) -> Option<CacheEntry> {
        for i in 0..16 {
            if let Some(c) = self.cache[i] {
                if c.code == code {
                    return Some(c);
                }
            }
        }
        None
    }

    fn reset_keystate<O: Output>(&mut self, output: &mut O) {
        for i in 0..256 {
            if self.keystate[i] != 0 {
                output.send_key(i as u8, 0);
                self.keystate[i] = 0;
            }
        }
    }

    fn send_key<O: Output>(&mut self, output: &mut O, code: u8, pressed: u8) {
        if code == KEYD_NOOP || code == KEYD_EXTERNAL_MOUSE_BUTTON {
            return;
        }

        if pressed != 0 {
            self.last_pressed_output_code = code;
        }

        if self.keystate[code as usize] != pressed {
            self.keystate[code as usize] = pressed;
            output.send_key(code, pressed);
        }
    }

    fn clear_mod<O: Output>(&mut self, output: &mut O, code: u8) {
        let guard = (self.last_pressed_output_code == code) &&
                    (code == KEYD_LEFTMETA || code == KEYD_LEFTALT || code == KEYD_RIGHTALT) &&
                    self.inhibit_modifier_guard == 0 &&
                    self.config.disable_modifier_guard == 0;

        if guard && self.keystate[KEYD_LEFTCTRL as usize] == 0 {
            self.send_key(output, KEYD_LEFTCTRL, 1);
            self.send_key(output, code, 0);
            self.send_key(output, KEYD_LEFTCTRL, 0);
        } else {
            self.send_key(output, code, 0);
        }
    }

    fn set_mods<O: Output>(&mut self, output: &mut O, mods: u8) {
        for m in &MODIFIERS {
            if (m.mask & mods) != 0 {
                if self.keystate[m.key as usize] == 0 {
                    self.send_key(output, m.key, 1);
                }
            } else {
                if self.keystate[m.key as usize] != 0 {
                    self.clear_mod(output, m.key);
                }
            }
        }
    }

    fn update_mods<O: Output>(&mut self, output: &mut O, excluded_layer_idx: i32, mods: u8) -> u8 {
        let mut final_mods = mods;
        for i in 0..self.config.layers.len() {
            if self.layer_state[i].active != 0 && (excluded_layer_idx == -1 || i != excluded_layer_idx as usize) {
                final_mods |= self.config.layers[i].mods;
            }
        }
        self.set_mods(output, final_mods);
        final_mods
    }

    fn resolve_descriptor(&self, code: u8) -> (Descriptor, i32) {
        // Walk active layers in activation order (most recently activated first)
        let mut active_layers: Vec<usize> = (0..self.config.layers.len())
            .filter(|&i| self.layer_state[i].active != 0)
            .collect();
        
        active_layers.sort_by_key(|&i| std::cmp::Reverse(self.layer_state[i].activation_time));

        for &i in &active_layers {
            let d = &self.config.layers[i].keymap[code as usize];
            if d.op != Op::KeySequence || if let DescriptorData::KeySequence(ref ks) = d.data { ks.code != 0 } else { false } {
                return (*d, i as i32);
            }
        }

        // Fallback to main layer
        let main_idx = 0; // Assuming main is always at index 0
        (self.config.layers[main_idx].keymap[code as usize], main_idx as i32)
    }

    pub fn kbd_process_events<O: Output>(&mut self, output: &mut O, events: &[KeyEvent]) -> i64 {
        let mut next_timeout = 0;
        for event in events {
             self.process_event(output, event.code, event.pressed, event.timestamp as i64);
        }
        // Simplified return for now
        next_timeout
    }

    fn process_event<O: Output>(&mut self, output: &mut O, code: u8, pressed: u8, time: i64) {
        if pressed != 0 {
            let (d, layer) = self.resolve_descriptor(code);
            self.execute_descriptor(output, d, code, layer, pressed, time);
        } else {
            if let Some(entry) = self.cache_get(code) {
                self.execute_descriptor(output, entry.d, code, entry.layer, pressed, time);
                self.cache_set(code, None);
            } else {
                // Should not happen if everything is balanced
                let (d, layer) = self.resolve_descriptor(code);
                self.execute_descriptor(output, d, code, layer, pressed, time);
            }
        }
    }

    fn execute_descriptor<O: Output>(&mut self, output: &mut O, d: Descriptor, code: u8, layer: i32, pressed: u8, time: i64) {
        match d.op {
            Op::KeySequence => {
                if let DescriptorData::KeySequence(ks) = d.data {
                    if pressed != 0 {
                        self.cache_set(code, Some(CacheEntry { code, d, dl: 0, layer }));
                        let mods = self.update_mods(output, -1, ks.mods);
                        self.send_key(output, ks.code, 1);
                    } else {
                        if let DescriptorData::KeySequence(ks) = d.data {
                            self.send_key(output, ks.code, 0);
                            self.update_mods(output, -1, 0);
                        }
                    }
                } else {
                    // Implicit keypress
                    if pressed != 0 {
                        self.cache_set(code, Some(CacheEntry { code, d, dl: 0, layer }));
                        self.update_mods(output, -1, 0);
                        self.send_key(output, code, 1);
                    } else {
                        self.send_key(output, code, 0);
                        self.update_mods(output, -1, 0);
                    }
                }
            }
            Op::Layer => {
                if let DescriptorData::Layer(l) = d.data {
                    let layer_idx = l.idx as usize;
                    if pressed != 0 {
                        self.layer_state[layer_idx].active += 1;
                        self.layer_state[layer_idx].activation_time = time;
                        output.on_layer_change(self, layer_idx, 1);
                        self.cache_set(code, Some(CacheEntry { code, d, dl: 0, layer }));
                        self.update_mods(output, -1, 0);
                    } else {
                        if self.layer_state[layer_idx].active > 0 {
                            self.layer_state[layer_idx].active -= 1;
                            output.on_layer_change(self, layer_idx, 0);
                        }
                        self.update_mods(output, -1, 0);
                    }
                }
            }
            _ => {
                // TODO: Implement other ops
            }
        }
    }
}
