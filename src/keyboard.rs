use crate::{Config, Descriptor, Op, LayerType, MacroEntryType};
use crate::vkbd::VirtualKeyboard;
use std::time::{Instant, Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: u8,
    pub pressed: bool,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    code: u8,
    descriptor: Descriptor,
    layer_idx: usize,
}

#[derive(Debug, Default)]
struct LayerState {
    active_count: u32,
    toggled: bool,
    oneshot_depth: u32,
}

#[derive(Debug, Clone)]
struct OverloadState {
    code: u8,
    layer_idx: usize,
    action_idx: usize,
    timestamp: Instant,
    timeout: Duration,
}

pub struct Keyboard {
    config: Config,
    vkbd: Box<dyn VirtualKeyboard>,
    
    cache: Vec<CacheEntry>,
    layer_state: Vec<LayerState>,
    virtual_keystate: [bool; 256],
    physical_keystate: [bool; 256],
    
    last_pressed_code: u8,
    last_pressed_output_code: u8,
    last_repeatable_action: Option<Descriptor>,
    
    pending_overload: Option<OverloadState>,
    oneshot_latch: bool,
}

impl Keyboard {
    pub fn new(config: Config, vkbd: Box<dyn VirtualKeyboard>) -> Self {
        let nr_layers = config.layers.len();
        let mut layer_state: Vec<LayerState> = (0..nr_layers).map(|_| LayerState::default()).collect();
        // Layer 0 (main) is always active
        layer_state[0].active_count = 1;
        
        Self {
            config,
            vkbd,
            cache: Vec::with_capacity(16),
            layer_state,
            virtual_keystate: [false; 256],
            physical_keystate: [false; 256],
            last_pressed_code: 0,
            last_pressed_output_code: 0,
            last_repeatable_action: None,
            pending_overload: None,
            oneshot_latch: false,
        }
    }

    fn send_key(&mut self, code: u8, pressed: bool) {
        if code == 0 { return; }
        
        if pressed {
            self.last_pressed_output_code = code;
        }

        if self.virtual_keystate[code as usize] != pressed {
            self.virtual_keystate[code as usize] = pressed;
            self.vkbd.send_key(code, if pressed { 1 } else { 0 });
        }
    }

    fn clear_mod(&mut self, code: u8) {
        let guard = self.last_pressed_output_code == code && 
            (code == 125 || code == 56 || code == 100) && 
            !self.config.disable_modifier_guard;

        if guard && !self.virtual_keystate[29] {
            self.send_key(29, true);
            self.send_key(code, false);
            self.send_key(29, false);
        } else {
            self.send_key(code, false);
        }
    }

    fn set_mods(&mut self, mods: u8) {
        for m in &crate::keys::MODIFIERS {
            if (m.mask & mods) != 0 {
                if !self.virtual_keystate[m.key as usize] {
                    self.send_key(m.key, true);
                }
            } else {
                if self.virtual_keystate[m.key as usize] {
                    self.clear_mod(m.key);
                }
            }
        }
    }

    fn is_layer_active(&self, idx: usize) -> bool {
        let state = &self.layer_state[idx];
        if state.active_count > 0 || state.toggled || state.oneshot_depth > 0 {
            return true;
        }
        
        let layer = &self.config.layers[idx];
        if layer.layer_type == LayerType::Composite {
            if !layer.constituents.is_empty() && layer.constituents.iter().all(|&c_idx| self.is_layer_active(c_idx)) {
                return true;
            }
        }
        
        false
    }

    fn update_mods(&mut self, excluded_layer_idx: i32, mut mods: u8) -> u8 {
        for i in 0..self.config.layers.len() {
            if !self.is_layer_active(i) {
                continue;
            }

            let mut excluded = false;
            if excluded_layer_idx != -1 {
                if i == excluded_layer_idx as usize {
                    excluded = true;
                } else if self.config.layers[excluded_layer_idx as usize].layer_type == LayerType::Composite {
                    if self.config.layers[excluded_layer_idx as usize].constituents.contains(&i) {
                        excluded = true;
                    }
                }
            }

            if !excluded {
                mods |= self.config.layers[i].mods;
            }
        }

        self.set_mods(mods);
        mods
    }

    fn activate_layer(&mut self, layer_idx: usize) {
        self.layer_state[layer_idx].active_count += 1;
    }

    fn deactivate_layer(&mut self, layer_idx: usize) {
        if self.layer_state[layer_idx].active_count > 0 {
            self.layer_state[layer_idx].active_count -= 1;
        }
    }

    fn clear_oneshot(&mut self) {
        for i in 0..self.config.layers.len() {
            while self.layer_state[i].oneshot_depth > 0 {
                self.deactivate_layer(i);
                self.layer_state[i].oneshot_depth -= 1;
            }
        }
    }

    pub fn process_event(&mut self, ev: KeyEvent) {
        self.physical_keystate[ev.code as usize] = ev.pressed;

        if let Some(pending) = self.pending_overload.clone() {
            if ev.pressed && ev.code != pending.code {
                self.resolve_overload(true);
            } else if !ev.pressed && ev.code == pending.code {
                if ev.timestamp.duration_since(pending.timestamp) < pending.timeout {
                    self.resolve_overload(false);
                } else {
                    self.resolve_overload(true);
                    self.execute_descriptor(self.config.descriptors[pending.action_idx].clone(), false, ev.timestamp, pending.layer_idx as i32);
                }
                return;
            }
        }

        if ev.pressed {
            self.last_pressed_code = ev.code;
            self.oneshot_latch = false;
            let (desc, layer_idx) = self.lookup_descriptor(ev.code);
            
            match desc.0 {
                Op::Overload { layer_idx: l_idx, action_idx } => {
                    self.pending_overload = Some(OverloadState {
                        code: ev.code,
                        layer_idx: l_idx,
                        action_idx,
                        timestamp: ev.timestamp,
                        timeout: Duration::from_millis(self.config.overload_tap_timeout),
                    });
                }
                _ => {
                    self.cache.push(CacheEntry {
                        code: ev.code,
                        descriptor: desc.clone(),
                        layer_idx,
                    });
                    self.execute_descriptor(desc, true, ev.timestamp, layer_idx as i32);
                }
            }
        } else {
            if let Some(idx) = self.cache.iter().position(|c| c.code == ev.code) {
                let entry = self.cache.remove(idx);
                self.execute_descriptor(entry.descriptor, false, ev.timestamp, entry.layer_idx as i32);
            }
        }
    }

    pub fn check_timeouts(&mut self) {
        let now = Instant::now();
        if let Some(pending) = self.pending_overload.clone() {
            if now.duration_since(pending.timestamp) >= pending.timeout {
                self.resolve_overload(true);
            }
        }
    }

    fn resolve_overload(&mut self, hold: bool) {
        if let Some(pending) = self.pending_overload.take() {
            if hold {
                self.activate_layer(pending.layer_idx);
                self.cache.push(CacheEntry {
                    code: pending.code,
                    descriptor: Descriptor(Op::Layer { layer_idx: pending.layer_idx }),
                    layer_idx: pending.layer_idx,
                });
                self.update_mods(-1, 0);
            } else {
                let desc = self.config.descriptors[pending.action_idx].clone();
                self.execute_descriptor(desc.clone(), true, Instant::now(), -1);
                self.execute_descriptor(desc, false, Instant::now(), -1);
            }
        }
    }

    fn lookup_descriptor(&self, code: u8) -> (Descriptor, usize) {
        let mut active_layers: Vec<usize> = Vec::new();
        for i in 0..self.config.layers.len() {
            if self.is_layer_active(i) {
                active_layers.push(i);
            }
        }
        
        // Priority: Composite layers > others? 
        // In keyd, it's just reverse activation order, but composite layers are "always active" if their parts are.
        // Let's sort them so composite layers come first or follow activation order.
        active_layers.reverse();

        // 1. Check for chords in active layers
        let mut held_keys: Vec<u8> = Vec::new();
        for i in 0..256 {
            if self.physical_keystate[i] {
                held_keys.push(i as u8);
            }
        }
        
        if held_keys.len() > 1 {
            for &layer_idx in &active_layers {
                for chord in &self.config.layers[layer_idx].chords {
                    if chord.keys.len() == held_keys.len() && chord.keys.iter().all(|k| held_keys.contains(k)) {
                        return (chord.descriptor.clone(), layer_idx);
                    }
                }
            }
        }

        // 2. Regular lookup
        for &layer_idx in &active_layers {
            if let Some(Some(desc)) = self.config.layers[layer_idx].keymap.get(code as usize) {
                return (desc.clone(), layer_idx);
            }
        }
        
        if let Some(Some(desc)) = self.config.layers[0].keymap.get(code as usize) {
            return (desc.clone(), 0);
        }

        (Descriptor(Op::KeySequence { code, mods: 0 }), 0)
    }

    fn execute_descriptor(&mut self, desc: Descriptor, pressed: bool, _time: Instant, dl: i32) {
        if pressed {
            match desc.0 {
                Op::KeySequence { .. } | Op::Macro { .. } | Op::Unicode { .. } | Op::Repeat => {
                    self.last_repeatable_action = Some(desc.clone());
                }
                _ => {}
            }
        }

        match desc.0 {
            Op::Repeat => {
                if pressed {
                    if let Some(last) = self.last_repeatable_action.clone() {
                        if let Op::Repeat = last.0 {
                        } else {
                            self.execute_descriptor(last.clone(), true, _time, dl);
                            self.execute_descriptor(last, false, _time, dl);
                        }
                    }
                }
            }
            Op::OneshotK { layer_idx, action_idx } => {
                let action = self.config.descriptors[action_idx].clone();
                self.execute_descriptor(action.clone(), pressed, _time, dl);
                if pressed {
                    self.activate_layer(layer_idx);
                    self.update_mods(dl, 0);
                    self.oneshot_latch = true;
                } else {
                    if self.oneshot_latch {
                        self.layer_state[layer_idx].oneshot_depth += 1;
                    } else {
                        self.deactivate_layer(layer_idx);
                        self.update_mods(-1, 0);
                    }
                }
            }
            Op::KeySequence { code, mods } => {
                if pressed {
                    self.update_mods(dl, mods);
                    self.send_key(code, true);
                    self.clear_oneshot();
                } else {
                    self.send_key(code, false);
                    self.update_mods(-1, 0);
                }
            }
            Op::Layer { layer_idx } => {
                if pressed {
                    self.activate_layer(layer_idx);
                } else {
                    self.deactivate_layer(layer_idx);
                }
                self.update_mods(-1, 0);
            }
            Op::Toggle { layer_idx } => {
                if pressed {
                    self.layer_state[layer_idx].toggled = !self.layer_state[layer_idx].toggled;
                    if self.layer_state[layer_idx].toggled {
                        self.activate_layer(layer_idx);
                    } else {
                        self.deactivate_layer(layer_idx);
                    }
                    self.update_mods(-1, 0);
                    self.clear_oneshot();
                }
            }
            Op::Swap { layer_idx } => {
                if pressed {
                    self.activate_layer(layer_idx);
                } else {
                    self.deactivate_layer(layer_idx);
                }
                self.update_mods(-1, 0);
            }
            Op::Oneshot { layer_idx } => {
                if pressed {
                    self.activate_layer(layer_idx);
                    self.update_mods(dl, 0);
                    self.oneshot_latch = true;
                } else {
                    if self.oneshot_latch {
                        self.layer_state[layer_idx].oneshot_depth += 1;
                    } else {
                        self.deactivate_layer(layer_idx);
                        self.update_mods(-1, 0);
                    }
                }
            }
            Op::Macro { macro_idx } => {
                if pressed {
                    self.clear_oneshot();
                    self.execute_macro(macro_idx, dl);
                }
            }
            Op::Unicode { codepoint } => {
                if pressed {
                    self.clear_oneshot();
                    self.execute_unicode(codepoint, dl);
                }
            }
            Op::Clear => {
                if pressed {
                    for state in &mut self.layer_state {
                        state.toggled = false;
                        state.oneshot_depth = 0;
                        state.active_count = 0;
                    }
                    self.layer_state[0].active_count = 1;
                    self.update_mods(-1, 0);
                }
            }
            _ => {}
        }
    }

    fn execute_macro(&mut self, macro_idx: usize, dl: i32) {
        let m = self.config.macros[macro_idx].clone();
        
        if m.entries.len() == 1 {
            if let MacroEntryType::KeySequence(data) = m.entries[0] {
                let code = (data & 0xFF) as u8;
                let mods = (data >> 8) as u8;
                self.update_mods(dl, mods);
                self.send_key(code, true);
                self.send_key(code, false);
                self.update_mods(-1, 0);
                return;
            }
        }

        self.update_mods(dl, 0);
        for entry in &m.entries {
            match entry {
                MacroEntryType::KeySequence(data) => {
                    let code = (data & 0xFF) as u8;
                    self.send_key(code, true);
                    self.send_key(code, false);
                }
                MacroEntryType::Hold(data) => {
                    self.send_key(*data as u8, true);
                }
                MacroEntryType::Release(data) => {
                    self.send_key(*data as u8, false);
                }
                MacroEntryType::Timeout(ms) => {
                    std::thread::sleep(Duration::from_millis(*ms as u64));
                }
                MacroEntryType::Unicode(cp) => {
                    self.execute_unicode(*cp as u32, -1);
                }
            }
        }
        self.update_mods(-1, 0);
    }

    fn execute_unicode(&mut self, codepoint: u32, dl: i32) {
        if let Some(idx) = crate::unicode::lookup_index(codepoint) {
            let seq = crate::unicode::get_sequence(idx);
            self.update_mods(dl, 0);
            for code in seq {
                self.send_key(code, true);
                self.send_key(code, false);
            }
            self.update_mods(-1, 0);
        }
    }
}
