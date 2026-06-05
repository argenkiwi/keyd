pub mod keys;
pub mod unicode;
pub mod vkbd;
pub mod device;
pub mod config;
pub mod keyboard;
pub mod ipc;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    KeySequence { code: u8, mods: u8 },
    Oneshot { layer_idx: usize },
    OneshotMulti { layer_indices: [usize; 3] },
    OneshotM { layer_idx: usize, macro_idx: usize },
    OneshotK { layer_idx: usize, action_idx: usize },
    LayerM { layer_idx: usize, macro_idx: usize },
    Swap { layer_idx: usize },
    SwapM { layer_idx: usize, macro_idx: usize },
    Layer { layer_idx: usize },
    Layout { layer_idx: usize },
    Clear,
    ClearM { macro_idx: usize },
    Overload { layer_idx: usize, action_idx: usize },
    OverloadTimeout { layer_idx: usize, action_idx: usize, timeout: u16 },
    OverloadTimeoutTap { layer_idx: usize, action_idx: usize, timeout: u16 },
    OverloadIdleTimeout { action1_idx: usize, action2_idx: usize, timeout: u16 },
    Toggle { layer_idx: usize },
    ToggleM { layer_idx: usize, macro_idx: usize },
    Repeat,
    Macro { macro_idx: usize },
    Macro2 { delay: u16, interval: u16, macro_idx: usize },
    Command { cmd_idx: usize },
    Timeout { action1_idx: usize, timeout: u16, action2_idx: usize },
    Unicode { codepoint: u32 },
    ScrollToggleOn { sensitivity: i16 },
    ScrollToggleOff,
    ScrollToggle { sensitivity: i16 },
    Scroll { sensitivity: i16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Descriptor(pub Op);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroEntryType {
    KeySequence(u16), // code | (mods << 8)
    Hold(u16),
    Release(u16),
    Unicode(u16),
    Timeout(u16),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Macro {
    pub entries: Vec<MacroEntryType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chord {
    pub keys: Vec<u8>,
    pub descriptor: Descriptor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    Normal,
    Layout,
    Composite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub layer_type: LayerType,
    pub mods: u8,
    pub keymap: Vec<Option<Descriptor>>, // 256 entries
    pub chords: Vec<Chord>,
    pub constituents: Vec<usize>, // For composite layers
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub path: String,
    pub layers: Vec<Layer>,
    pub descriptors: Vec<Descriptor>,
    pub macros: Vec<Macro>,
    pub commands: Vec<String>,
    pub aliases: Vec<String>, // 256 entries, each 32 chars in C

    pub macro_timeout: u64,
    pub macro_sequence_timeout: u64,
    pub macro_repeat_timeout: u64,
    pub oneshot_timeout: u64,
    pub overload_tap_timeout: u64,
    pub chord_interkey_timeout: u64,
    pub chord_hold_timeout: u64,

    pub layer_indicator: bool,
    pub disable_modifier_guard: bool,
    pub default_layout: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: String::new(),
            layers: Vec::new(),
            descriptors: Vec::new(),
            macros: Vec::new(),
            commands: Vec::new(),
            aliases: vec![String::new(); 256],
            macro_timeout: 600,
            macro_sequence_timeout: 0,
            macro_repeat_timeout: 50,
            oneshot_timeout: 800,
            overload_tap_timeout: 0,
            chord_interkey_timeout: 50,
            chord_hold_timeout: 0,
            layer_indicator: false,
            disable_modifier_guard: false,
            default_layout: String::new(),
        }
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            name: String::new(),
            layer_type: LayerType::Normal,
            mods: 0,
            keymap: vec![None; 256],
            chords: Vec::new(),
            constituents: Vec::new(),
        }
    }
}
