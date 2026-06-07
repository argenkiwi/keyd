use crate::config::*;
use crate::config_parse::*;
use crate::ini::*;
use crate::keys::*;

pub fn config_get_layer_index(config: &Config, name: &str) -> Option<usize> {
    config.layers.iter().position(|l| l.name == name)
}

fn create_layer(config: &mut Config, name: &str, layer_type: LayerType) -> usize {
    if let Some(idx) = config_get_layer_index(config, name) {
        return idx;
    }
    let idx = config.layers.len();
    config.layers.push(Layer::new(name.to_string()));
    config.layers[idx].layer_type = layer_type;
    idx
}

pub fn config_parse_string(config: &mut Config, content: &str) -> Result<usize, String> {
    let ini = ini_parse_string(content, None).ok_or("Failed to parse INI")?;
    let mut ctx = ParseCtx::new();

    // First pass: identify layers
    for section in &ini.sections {
        if section.name == "ids" || section.name == "global" || section.name == "aliases" {
            continue;
        }

        if section.name.contains('+') {
             create_layer(config, &section.name, LayerType::Composite);
        } else {
             create_layer(config, &section.name, LayerType::Normal);
        }
    }

    // Default layers
    create_layer(config, "main", LayerType::Normal);
    let control_idx = create_layer(config, "control", LayerType::Normal);
    config.layers[control_idx].mods = MOD_CTRL;
    let shift_idx = create_layer(config, "shift", LayerType::Normal);
    config.layers[shift_idx].mods = MOD_SHIFT;
    let alt_idx = create_layer(config, "alt", LayerType::Normal);
    config.layers[alt_idx].mods = MOD_ALT;
    let meta_idx = create_layer(config, "meta", LayerType::Normal);
    config.layers[meta_idx].mods = MOD_SUPER;
    let altgr_idx = create_layer(config, "altgr", LayerType::Normal);
    config.layers[altgr_idx].mods = MOD_ALT_GR;

    // Second pass: parse content
    for section in &ini.sections {
        ctx.current_line = section.lnum - 1;

        if section.name == "ids" {
            for entry in &section.entries {
                let id = entry.key.clone();
                let mut _flags = ID_KEYBOARD | ID_KEY;
                if id.starts_with('-') {
                    _flags = ID_EXCLUDED;
                }
                config.ids.push(ConfigId { id, flags: _flags });
            }
        } else if section.name == "global" {
            for entry in &section.entries {
                if let Some(ref val) = entry.val {
                    match entry.key.as_str() {
                        "macro_timeout" => config.macro_timeout = val.parse().unwrap_or(0),
                        "macro_sequence_timeout" => config.macro_sequence_timeout = val.parse().unwrap_or(0),
                        "macro_repeat_timeout" => config.macro_repeat_timeout = val.parse().unwrap_or(0),
                        "oneshot_timeout" => config.oneshot_timeout = val.parse().unwrap_or(0),
                        "overload_tap_timeout" => config.overload_tap_timeout = val.parse().unwrap_or(0),
                        "chord_interkey_timeout" => config.chord_interkey_timeout = val.parse().unwrap_or(0),
                        "chord_hold_timeout" => config.chord_hold_timeout = val.parse().unwrap_or(0),
                        "layer_indicator" => config.layer_indicator = val.parse().unwrap_or(0),
                        "disable_modifier_guard" => config.disable_modifier_guard = val.parse().unwrap_or(0),
                        "default_layout" => config.default_layout = val.clone(),
                        _ => config_warn(&mut ctx, &format!("Unknown global option: {}", entry.key)),
                    }
                }
            }
        } else if section.name == "aliases" {
            for entry in &section.entries {
                if let Some(ref val) = entry.val {
                    config.aliases.push((entry.key.clone(), val.clone()));
                }
            }
        } else {
            let layer_idx = config_get_layer_index(config, &section.name).unwrap();
            
            // Handle composite layer constituents
            if section.name.contains('+') {
                let parts: Vec<&str> = section.name.split('+').collect();
                let mut constituents = [0; 8];
                for (i, part) in parts.iter().enumerate() {
                    if i >= 8 { break; }
                    if let Some(idx) = config_get_layer_index(config, part) {
                        constituents[i] = idx as i32;
                    } else {
                        constituents[i] = create_layer(config, part, LayerType::Normal) as i32;
                    }
                }
                config.layers[layer_idx].nr_constituents = parts.len();
                config.layers[layer_idx].constituents = constituents;
            }

            for entry in &section.entries {
                ctx.current_line = entry.lnum - 1;
                
                // Handle include directive
                if entry.key == "include" {
                    if let Some(ref _val) = entry.val {
                        // In a real implementation we would load the file.
                    }
                    continue;
                }

                if let Some((code, _)) = parse_key_sequence(&entry.key) {
                    if let Some(ref val) = entry.val {
                        let desc = config_parse_descriptor(val, config, &mut ctx)?;
                        config.layers[layer_idx].keymap[code as usize] = desc;
                    }
                } else if entry.key.contains('+') {
                    // Chord
                    let mut keys = [0u8; 8];
                    let mut sz = 0;
                    for part in entry.key.split('+') {
                        if sz >= 8 { break; }
                        if let Some((code, _)) = parse_key_sequence(part) {
                            keys[sz] = code;
                            sz += 1;
                        }
                    }
                    if let Some(ref val) = entry.val {
                        let desc = config_parse_descriptor(val, config, &mut ctx)?;
                        let nr_chords = config.layers[layer_idx].nr_chords;
                        config.layers[layer_idx].chords[nr_chords] = Chord {
                            keys,
                            sz,
                            d: desc,
                        };
                        config.layers[layer_idx].nr_chords += 1;
                    }
                }
            }
        }
    }

    Ok(ctx.nr_warnings)
}

pub fn config_check_match(config: &Config, id: &str, _flags: u8) -> i32 {
    let mut rank = 0;
    for cfg_id in &config.ids {
        if cfg_id.id == "*" {
            if rank < 1 { rank = 1; }
        } else if cfg_id.id.starts_with('-') && &cfg_id.id[1..] == id {
            return 0;
        } else if cfg_id.id == id {
            rank = 2;
        }
    }
    rank
}

pub fn config_add_entry(config: &mut Config, exp: &str) -> Result<(), String> {
    let parts: Vec<&str> = exp.split('=').collect();
    if parts.len() != 2 {
        return Err("Invalid entry expression".to_string());
    }
    let key_parts: Vec<&str> = parts[0].trim().split('.').collect();
    if key_parts.len() != 2 {
        return Err("Invalid key part".to_string());
    }
    let layer_name = key_parts[0];
    let key_name = key_parts[1];
    let val = parts[1].trim();

    let layer_idx = config_get_layer_index(config, layer_name).ok_or(format!("Layer {} not found", layer_name))?;
    let (code, _) = parse_key_sequence(key_name).ok_or(format!("Invalid key {}", key_name))?;
    
    let mut ctx = ParseCtx::new();
    let desc = config_parse_descriptor(val, config, &mut ctx)?;
    config.layers[layer_idx].keymap[code as usize] = desc;
    Ok(())
}
