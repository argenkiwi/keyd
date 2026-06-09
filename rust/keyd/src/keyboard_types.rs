use crate::config::*;

#[derive(Debug, Clone, Copy)]
pub struct CacheEntry {
    pub code: u8,
    pub d: Descriptor,
    pub dl: i32,
    pub layer: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub code: u8,
    pub pressed: u8,
    pub timestamp: i32,
}

pub trait Output {
    fn send_key(&mut self, code: u8, state: u8);
    fn on_layer_change(&mut self, kbd: &Keyboard, layer_idx: usize, active: u8);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChordStatus {
    Resolving,
    Inactive,
    PendingDisambiguation,
    PendingHoldTimeout,
}

pub struct ChordState {
    pub queue: [KeyEvent; 32],
    pub queue_sz: usize,
    pub match_idx: Option<usize>,
    pub match_layer: i32,
    pub start_code: u8,
    pub last_code_time: i64,
    pub state: ChordStatus,
}

pub struct TimeoutState {
    pub code: u8,
    pub dl: u8,
    pub spontaneous: u8,
    pub expiration: i64,
    pub activation_time: i64,
    pub action1: Descriptor,
    pub action2: Descriptor,
}

pub struct OverloadState {
    pub code: u8,
    pub dl: u8,
    pub expiration: i64,
    pub resolve_on_interrupt: i32,
    pub queue: [KeyEvent; 32],
    pub queue_sz: usize,
    pub action1: Descriptor,
    pub action2: Descriptor,
}

pub struct MacroPlayState {
    pub active_idx: Option<usize>,
    pub layer: i32,
    pub timeout: i64,
    pub repeat_interval: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct LayerState {
    pub activation_time: i64,
    pub active: u8,
    pub toggled: u8,
    pub oneshot_depth: u8,
}

#[derive(Clone, Copy)]
pub struct ActiveChord {
    pub active: u8,
    pub chord: Chord,
    pub layer: i32,
}

pub struct ScrollState {
    pub x: i32,
    pub y: i32,
    pub sensitivity: i32,
    pub active: i32,
}

pub struct Keyboard {
    pub config: Config,
    // Output trait object or similar will be needed.
    // For now, let's keep the fields.
    pub cache: [Option<CacheEntry>; 16],
    pub last_pressed_output_code: u8,
    pub last_pressed_code: u8,
    pub oneshot_latch: u8,
    pub inhibit_modifier_guard: u8,
    pub macro_play: MacroPlayState,
    pub overload_last_layer_code: i32,
    pub oneshot_timeout: i64,
    pub overload_start_time: i64,
    pub last_simple_key_time: i64,
    pub timeouts: [i64; 128],
    pub nr_timeouts: usize,
    pub active_chords: Vec<ActiveChord>,
    pub chord: ChordState,
    pub pending_timeout: Option<TimeoutState>,
    pub pending_overload: Option<OverloadState>,
    pub layer_state: [LayerState; MAX_LAYERS],
    pub last_repeatable_action: Descriptor,
    pub keystate: [u8; 256],
    pub scroll: ScrollState,
}
