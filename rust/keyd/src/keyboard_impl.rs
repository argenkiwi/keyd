use crate::config::*;
use crate::keyboard_types::*;
use crate::keys::*;

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
            layer_state: {
                let mut ls = [LayerState { activation_time: 0, active: 0, toggled: 0, oneshot_depth: 0 }; MAX_LAYERS];
                ls[0].active = 1; // main layer (index 0) is always active
                ls
            },
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

    fn clear_oneshot(&mut self) {
        // Full implementation in Phase 6 (oneshot state machine).
    }

    fn calculate_main_loop_timeout(&mut self, time: i64) -> i64 {
        // Full implementation in Phase 5 (timeout scheduling).
        // Prune expired timeouts and return ms until next one; 0 = none pending.
        let mut earliest: i64 = 0;
        let mut n = 0;
        for i in 0..self.nr_timeouts {
            if self.timeouts[i] > time {
                if earliest == 0 || self.timeouts[i] < earliest {
                    earliest = self.timeouts[i];
                }
                self.timeouts[n] = self.timeouts[i];
                n += 1;
            }
        }
        self.nr_timeouts = n;
        if earliest > 0 { earliest - time } else { 0 }
    }

    fn schedule_timeout(&mut self, deadline_ms: i64) {
        if self.nr_timeouts < self.timeouts.len() {
            self.timeouts[self.nr_timeouts] = deadline_ms;
            self.nr_timeouts += 1;
        }
    }

    /// Returns `true` if the event was consumed by the pending overload state machine.
    fn handle_pending_overload<O: Output>(
        &mut self, output: &mut O, code: u8, pressed: u8, time: i64,
    ) -> bool {
        if self.pending_overload.is_none() {
            return false;
        }

        // Let through key-up events for keys that were already held *before* the
        // pending overload started (they won't be in the queue and aren't the overload key).
        if code != 0 && pressed == 0 {
            let known = {
                let po = self.pending_overload.as_ref().unwrap();
                code == po.code
                    || po.queue[..po.queue_sz].iter().any(|e| e.code == code)
            };
            if !known {
                return false;
            }
        }

        // Enqueue real (non-synthetic) events.
        if code != 0 {
            let po = self.pending_overload.as_mut().unwrap();
            if po.queue_sz < po.queue.len() {
                po.queue[po.queue_sz] = KeyEvent { code, pressed, timestamp: time as i32 };
                po.queue_sz += 1;
            }
        }

        // Decide if we can resolve now.
        let resolve: Option<Descriptor> = {
            let po = self.pending_overload.as_ref().unwrap();
            if time >= po.expiration {
                Some(po.action2)                               // timeout → hold action
            } else if code == po.code && pressed == 0 {
                Some(po.action1)                               // overload key released → tap
            } else if po.resolve_on_interrupt != 0 && pressed == 0 {
                Some(po.action2)                               // overloadt2: any key-up → hold
            } else {
                None
            }
        };

        if let Some(action) = resolve {
            // Snapshot the queue before mutating self.
            let (overload_code, dl, queue_snap, queue_sz) = {
                let po = self.pending_overload.as_ref().unwrap();
                let sz = po.queue_sz;
                let mut q = [KeyEvent { code: 0, pressed: 0, timestamp: 0 }; 32];
                q[..sz].copy_from_slice(&po.queue[..sz]);
                (po.code, po.dl as i32, q, sz)
            };

            self.pending_overload = None;

            // Lock in the resolved action for this key's cache entry.
            self.cache_set(overload_code, Some(CacheEntry {
                code: overload_code, d: action, dl, layer: 0,
            }));
            self.execute_descriptor(output, action, overload_code, dl, 1, time);

            // Replay queued events (e.g. the overload key-release and any interleaved keys).
            if queue_sz > 0 {
                self.kbd_process_events(output, &queue_snap[..queue_sz]);
            }
        }

        true
    }

    fn reset_keystate<O: Output>(&mut self, output: &mut O) {
        for i in 0..256 {
            if self.keystate[i] != 0 {
                output.send_key(i as u8, 0);
                self.keystate[i] = 0;
            }
        }
    }

    fn activate_layer<O: Output>(&mut self, output: &mut O, code: u8, idx: usize, time: i64) {
        self.layer_state[idx].activation_time = time;
        self.layer_state[idx].active += 1;
        // Update the cache entry for the activating key so its layer reflects the new layer.
        for i in 0..16 {
            if let Some(ref mut ce) = self.cache[i] {
                if ce.code == code {
                    ce.layer = idx as i32;
                    break;
                }
            }
        }
        output.on_layer_change(self, idx, 1);
    }

    fn deactivate_layer<O: Output>(&mut self, output: &mut O, idx: usize) {
        debug_assert!(self.layer_state[idx].active > 0, "deactivate_layer called on inactive layer {}", idx);
        if self.layer_state[idx].active > 0 {
            self.layer_state[idx].active -= 1;
        }
        output.on_layer_change(self, idx, 0);
    }

    fn clear<O: Output>(&mut self, output: &mut O) {
        self.clear_oneshot();
        for i in 1..self.config.layers.len() {
            if self.config.layers[i].layer_type != LayerType::Layout {
                if self.layer_state[i].toggled != 0 {
                    self.layer_state[i].toggled = 0;
                    self.deactivate_layer(output, i);
                }
            }
        }
        self.macro_play.active_idx = None;
        self.reset_keystate(output);
    }

    fn setlayout<O: Output>(&mut self, output: &mut O, idx: usize) {
        self.clear(output);
        for i in 1..self.config.layers.len() {
            if self.config.layers[i].layer_type == LayerType::Layout {
                self.layer_state[i].active = 0;
            }
        }
        // Setting layout to main (idx=0) just clears all other layouts.
        if idx != 0 {
            self.layer_state[idx].activation_time = 1;
            self.layer_state[idx].active = 1;
        }
        output.on_layer_change(self, idx, 1);
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
        let mut i = 0;
        let mut pending_timeout: i64 = 0;
        let mut timeout_ts: i64 = 0;

        while i < events.len() {
            let ev = &events[i];
            let ev_ts = ev.timestamp as i64;

            // If a timeout deadline has passed before this event, fire it first.
            if pending_timeout > 0 && timeout_ts <= ev_ts {
                pending_timeout = self.process_event(output, 0, 0, timeout_ts);
                timeout_ts += pending_timeout;
            } else {
                pending_timeout = self.process_event(output, ev.code, ev.pressed, ev_ts);
                timeout_ts = ev_ts + pending_timeout;
                i += 1;
            }
        }

        pending_timeout
    }

    fn process_event<O: Output>(&mut self, output: &mut O, code: u8, pressed: u8, time: i64) -> i64 {
        // Phases 7/8 will insert handle_pending_timeout / handle_chord here.

        if self.handle_pending_overload(output, code, pressed, time) {
            return self.calculate_main_loop_timeout(time);
        }

        if code != 0 {
            if pressed != 0 {
                // Guard against successive key-down for the same code (e.g. two devices
                // mapped to the same config both sending the same key).
                if self.cache_get(code).is_some() {
                    return self.calculate_main_loop_timeout(time);
                }
                let (d, layer) = self.resolve_descriptor(code);
                // Store in cache before executing so re-entrant lookups work correctly.
                self.cache_set(code, Some(CacheEntry { code, d, dl: layer, layer }));
                self.execute_descriptor(output, d, code, layer, pressed, time);
            } else {
                if let Some(entry) = self.cache_get(code) {
                    let d = entry.d;
                    let layer = entry.layer;
                    // Clear cache before executing so the key is no longer seen as held.
                    self.cache_set(code, None);
                    self.execute_descriptor(output, d, code, layer, pressed, time);
                }
                // Silently ignore releases with no matching press (already cleaned up).
            }
        }

        self.calculate_main_loop_timeout(time)
    }

    fn execute_descriptor<O: Output>(&mut self, output: &mut O, d: Descriptor, code: u8, layer: i32, pressed: u8, time: i64) {
        match d.op {
            Op::KeySequence => {
                if let DescriptorData::KeySequence(ks) = d.data {
                    let new_code = ks.code;
                    let mods = ks.mods;
                    if pressed != 0 {
                        if self.keystate[new_code as usize] != 0 {
                            self.send_key(output, new_code, 0);
                        }
                        let active_mods = self.update_mods(output, layer, mods);
                        let mut ra = d;
                        if let DescriptorData::KeySequence(ref mut ra_ks) = ra.data {
                            ra_ks.mods = active_mods;
                        }
                        self.last_repeatable_action = ra;
                        self.send_key(output, new_code, 1);
                        self.clear_oneshot();
                    } else {
                        self.send_key(output, new_code, 0);
                        self.update_mods(output, -1, 0);
                    }
                    if mods == 0 || mods == MOD_SHIFT {
                        self.last_simple_key_time = time;
                    }
                } else {
                    // Passthrough (DescriptorData::None)
                    if pressed != 0 {
                        self.update_mods(output, layer, 0);
                        self.last_repeatable_action = d;
                        self.send_key(output, code, 1);
                        self.clear_oneshot();
                        self.last_simple_key_time = time;
                    } else {
                        self.send_key(output, code, 0);
                        self.update_mods(output, -1, 0);
                    }
                }
            }

            Op::Layer | Op::LayerM => {
                let idx = match d.data {
                    DescriptorData::Layer(l) => l.idx as usize,
                    DescriptorData::LayerMacro(lm) => lm.idx as usize,
                    _ => return,
                };
                if pressed != 0 {
                    self.activate_layer(output, code, idx, time);
                } else {
                    self.deactivate_layer(output, idx);
                }
                // If this is a solo tap of a modifier-like key, suppress the spurious
                // modifier tap the OS might register.
                if self.last_pressed_code == code {
                    self.inhibit_modifier_guard = 1;
                    self.update_mods(output, -1, 0);
                    self.inhibit_modifier_guard = 0;
                } else {
                    self.update_mods(output, -1, 0);
                }
            }

            Op::Layout => {
                if pressed != 0 {
                    if let DescriptorData::Layer(l) = d.data {
                        self.setlayout(output, l.idx as usize);
                    }
                }
            }

            Op::Toggle | Op::ToggleM => {
                let idx = match d.data {
                    DescriptorData::Layer(l) => l.idx as usize,
                    DescriptorData::LayerMacro(lm) => lm.idx as usize,
                    _ => return,
                };
                if pressed != 0 {
                    self.layer_state[idx].toggled ^= 1;
                    if self.layer_state[idx].toggled != 0 {
                        self.activate_layer(output, code, idx, time);
                    } else {
                        self.deactivate_layer(output, idx);
                    }
                    self.update_mods(output, -1, 0);
                    self.clear_oneshot();
                    // ToggleM: macro execution in Phase 9
                }
            }

            Op::Swap | Op::SwapM => {
                let idx = match d.data {
                    DescriptorData::Layer(l) => l.idx as usize,
                    DescriptorData::LayerMacro(lm) => lm.idx as usize,
                    _ => return,
                };
                let dl = layer as usize;
                if pressed != 0 {
                    if self.layer_state[dl].toggled != 0 {
                        // Source layer was toggled: swap the toggle to target layer.
                        self.deactivate_layer(output, dl);
                        self.layer_state[dl].toggled = 0;
                        self.activate_layer(output, 0, idx, time);
                        self.layer_state[idx].toggled = 1;
                        self.update_mods(output, -1, 0);
                    } else if self.layer_state[dl].oneshot_depth > 0 {
                        // Source layer was a oneshot: swap the oneshot to target layer.
                        self.deactivate_layer(output, dl);
                        self.layer_state[dl].oneshot_depth -= 1;
                        self.activate_layer(output, 0, idx, time);
                        self.layer_state[idx].oneshot_depth += 1;
                        self.update_mods(output, -1, 0);
                    } else {
                        // Normal case: find the cache entry that is holding dl active,
                        // patch its descriptor to layer(idx), then swap activation.
                        let mut ce_code: Option<u8> = None;
                        for i in 0..16 {
                            if let Some(ref mut ce) = self.cache[i] {
                                if ce.layer == dl as i32
                                    && ce.layer != 0
                                    && self.config.layers[ce.layer as usize].layer_type == LayerType::Normal
                                {
                                    ce.d = Descriptor {
                                        op: Op::Layer,
                                        data: DescriptorData::Layer(DescLayer { idx: idx as i16 }),
                                    };
                                    ce_code = Some(ce.code);
                                    break;
                                }
                            }
                        }
                        if let Some(activating_code) = ce_code {
                            self.deactivate_layer(output, dl);
                            self.activate_layer(output, activating_code, idx, time);
                            self.update_mods(output, -1, 0);
                        }
                    }
                    // SwapM: macro execution in Phase 9
                }
                // SwapM release: key release in Phase 9
            }

            Op::Clear => {
                if pressed != 0 {
                    self.clear(output);
                }
            }

            Op::ClearM => {
                if pressed != 0 {
                    self.clear(output);
                    // Macro execution in Phase 9
                }
            }

            Op::Repeat => {
                if pressed != 0 {
                    let ra = self.last_repeatable_action;
                    // Re-execute the last repeatable action as a press+release.
                    self.execute_descriptor(output, ra, code, layer, 1, time);
                    // Patch the cache entry so the release undoes the repeated action.
                    for i in 0..16 {
                        if let Some(ref mut ce) = self.cache[i] {
                            if ce.code == code {
                                ce.d = ra;
                                break;
                            }
                        }
                    }
                }
            }

            Op::Overload => {
                if let DescriptorData::Overload(ov) = d.data {
                    let layer_idx  = ov.layer_idx as usize;
                    let action_desc = self.config.descriptors[ov.action_idx as usize];
                    if pressed != 0 {
                        self.overload_start_time = time;
                        self.activate_layer(output, code, layer_idx, time);
                        self.update_mods(output, -1, 0);
                    } else {
                        self.deactivate_layer(output, layer_idx);
                        self.update_mods(output, -1, 0);
                        let tap_timeout = self.config.overload_tap_timeout;
                        let is_tap = self.last_pressed_code == code
                            && (tap_timeout == 0
                                || (time - self.overload_start_time) < tap_timeout);
                        if is_tap {
                            self.execute_descriptor(output, action_desc, code, layer, 1, time);
                            self.execute_descriptor(output, action_desc, code, layer, 0, time);
                        }
                    }
                }
            }

            Op::OverloadTimeout | Op::OverloadTimeoutTap => {
                if let DescriptorData::OverloadTo(ov) = d.data {
                    if pressed != 0 {
                        // action1 = the tap descriptor; action2 = layer activation.
                        let action1 = self.config.descriptors[ov.action_idx as usize];
                        let action2 = Descriptor {
                            op: Op::Layer,
                            data: DescriptorData::Layer(DescLayer { idx: ov.layer_idx }),
                        };
                        let expiration = time + ov.timeout as i64;
                        self.pending_overload = Some(OverloadState {
                            code,
                            dl: layer as u8,
                            expiration,
                            resolve_on_interrupt: if d.op == Op::OverloadTimeoutTap { 1 } else { 0 },
                            queue: [KeyEvent { code: 0, pressed: 0, timestamp: 0 }; 32],
                            queue_sz: 0,
                            action1,
                            action2,
                        });
                        self.schedule_timeout(expiration);
                    }
                }
            }

            Op::OverloadIdleTimeout => {
                if let DescriptorData::OverloadIdle(ov) = d.data {
                    if pressed != 0 {
                        let idle = time - self.last_simple_key_time;
                        let action = if idle >= ov.timeout as i64 {
                            self.config.descriptors[ov.action2_idx as usize]
                        } else {
                            self.config.descriptors[ov.action1_idx as usize]
                        };
                        self.execute_descriptor(output, action, code, layer, 1, time);
                        // Patch cache so release uses the resolved action.
                        for i in 0..16 {
                            if let Some(ref mut ce) = self.cache[i] {
                                if ce.code == code { ce.d = action; break; }
                            }
                        }
                    }
                }
            }

            _ => {
                log::warn!(
                    "Unimplemented op {:?} for key {}, passthrough",
                    d.op,
                    KEYCODE_TABLE[code as usize].name.unwrap_or("UNKNOWN")
                );
                if pressed != 0 {
                    self.send_key(output, code, 1);
                } else {
                    self.send_key(output, code, 0);
                }
            }
        }

        // Track the last physically pressed key (used by inhibit_modifier_guard, overload tap, etc.)
        if pressed != 0 {
            self.last_pressed_code = code;
        }
    }
}
