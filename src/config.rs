use nom::{
    branch::alt,
    bytes::complete::{take_while, take_while1},
    character::complete::{char, space0, space1, line_ending, not_line_ending, alphanumeric1},
    combinator::{recognize, value},
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, separated_pair, tuple},
    IResult,
};
use crate::{Descriptor, Op, Layer, Config, Macro, MacroEntryType, LayerType};
use crate::keys::{lookup_keycode, parse_key_sequence, parse_modset};
use std::fs;
use std::path::Path;

pub struct ParserContext {
    pub config: Config,
    pub current_layer: Option<usize>,
}

impl ParserContext {
    pub fn new() -> Self {
        Self {
            config: Config {
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
            },
            current_layer: None,
        }
    }
}

// Nom parsers

fn comment(input: &str) -> IResult<&str, &str> {
    recognize(preceded(char('#'), not_line_ending))(input)
}

fn whitespace(input: &str) -> IResult<&str, ()> {
    value((), many0(alt((value((), space1), value((), comment), value((), line_ending)))))(input)
}

fn section_name(input: &str) -> IResult<&str, &str> {
    recognize(take_while1(|c: char| c != ']' && c != '\n'))(input)
}

fn section_header(input: &str) -> IResult<&str, &str> {
    delimited(char('['), section_name, char(']'))(input)
}

fn key_name(input: &str) -> IResult<&str, &str> {
    recognize(take_while1(|c: char| !c.is_whitespace() && c != '=' && c != '[' && c != ']' && c != '#'))(input)
}

fn value_str(input: &str) -> IResult<&str, &str> {
    recognize(take_while(|c: char| c != '\n' && c != '#'))(input)
}

fn entry(input: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(key_name, delimited(space0, char('='), space0), value_str)(input)
}

fn parse_arg(input: &str) -> IResult<&str, &str> {
    let mut depth = 0;
    let mut end = 0;
    let chars: Vec<char> = input.chars().collect();
    while end < chars.len() {
        let c = chars[end];
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            if depth == 0 {
                break;
            }
            depth -= 1;
        } else if c == ',' && depth == 0 {
            break;
        }
        end += 1;
    }
    
    let consumed = chars[..end].iter().collect::<String>();
    // We need to return the remaining input slice from the original &str
    // This is tricky with chars().collect(). Let's use byte offsets.
    let mut byte_offset = 0;
    for i in 0..end {
        byte_offset += chars[i].len_utf8();
    }
    
    Ok((&input[byte_offset..], &input[..byte_offset]))
}

fn parse_fn_call(input: &str) -> IResult<&str, (&str, Vec<&str>)> {
    let (input, name) = alphanumeric1(input)?;
    let (input, args) = delimited(
        char('('),
        separated_list0(tuple((space0, char(','), space0)), parse_arg),
        char(')')
    )(input)?;
    Ok((input, (name, args)))
}

fn macro_parse(s: &str, config: &mut Config) -> Result<Macro, String> {
    let mut m = Macro { entries: Vec::new() };
    for tok in s.split_whitespace() {
        if let Some((code, mods)) = parse_key_sequence(tok) {
            m.entries.push(MacroEntryType::KeySequence((mods as u16) << 8 | code as u16));
        } else if tok.contains('+') {
            let mut keys: Vec<u8> = Vec::new();
            for key in tok.split('+') {
                if let Ok(ms) = key.parse::<u16>() {
                    m.entries.push(MacroEntryType::Timeout(ms));
                } else if let Some((code, _)) = parse_key_sequence(key) {
                    m.entries.push(MacroEntryType::Hold(code as u16));
                    keys.push(code);
                }
            }
            m.entries.push(MacroEntryType::Release(0)); // In C it releases all held keys in this + sequence
        } else if let Ok(ms) = tok.parse::<u16>() {
            m.entries.push(MacroEntryType::Timeout(ms));
        } else {
            // Handle plain text characters
            for c in tok.chars() {
                if let Some(idx) = crate::unicode::lookup_index(c as u32) {
                     m.entries.push(MacroEntryType::Unicode(idx as u16));
                } else {
                    // Try to find key by name (e.g. 'a')
                    let mut found = false;
                    for (i, ent) in crate::keys::KEYCODE_TABLE.iter().enumerate() {
                        if let Some(ent) = ent {
                            if ent.name.len() == 1 && ent.name.chars().next().unwrap() == c {
                                m.entries.push(MacroEntryType::KeySequence(i as u16));
                                found = true;
                                break;
                            }
                            if let Some(shifted) = ent.shifted_name {
                                if shifted.len() == 1 && shifted.chars().next().unwrap() == c {
                                    m.entries.push(MacroEntryType::KeySequence((crate::keys::MOD_SHIFT as u16) << 8 | i as u16));
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                    if !found {
                        return Err(format!("Invalid macro token: {}", tok));
                    }
                }
            }
        }
    }
    Ok(m)
}

fn parse_descriptor_internal<'a>(input: &'a str, config: &mut Config) -> Result<Descriptor, String> {
    if input.is_empty() {
        return Ok(Descriptor(Op::Clear));
    }

    if let Some((code, mods)) = parse_key_sequence(input) {
        return Ok(Descriptor(Op::KeySequence { code, mods }));
    }

    if input.starts_with("command(") && input.ends_with(')') {
        let cmd = input[8..input.len()-1].to_string();
        config.commands.push(cmd);
        return Ok(Descriptor(Op::Command { cmd_idx: config.commands.len() - 1 }));
    }

    if input.starts_with("macro(") && input.ends_with(')') {
        let m_str = &input[6..input.len()-1];
        let m = macro_parse(m_str, config)?;
        config.macros.push(m);
        return Ok(Descriptor(Op::Macro { macro_idx: config.macros.len() - 1 }));
    }

    if let Ok(("", (name, args))) = parse_fn_call(input) {
        match name {
            "overload" => {
                if args.len() == 2 {
                    let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                    let action = parse_descriptor_internal(args[1], config)?;
                    config.descriptors.push(action);
                    return Ok(Descriptor(Op::Overload { layer_idx, action_idx: config.descriptors.len() - 1 }));
                }
            }
            "overloadt" | "overloadt2" => {
                if args.len() == 3 {
                    let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                    let action = parse_descriptor_internal(args[1], config)?;
                    let timeout: u16 = args[2].parse().unwrap_or(0);
                    config.descriptors.push(action);
                    return Ok(Descriptor(Op::OverloadTimeout { layer_idx, action_idx: config.descriptors.len() - 1, timeout }));
                }
            }
            "oneshot" => {
                if args.len() >= 1 {
                    let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                    // TODO: Handle second arg (mods) for oneshot
                    return Ok(Descriptor(Op::Oneshot { layer_idx }));
                }
            }
            "oneshotm" => {
                if args.len() == 2 {
                     let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                     let m = parse_descriptor_internal(args[1], config)?;
                     if let Op::Macro { macro_idx } = m.0 {
                         return Ok(Descriptor(Op::OneshotM { layer_idx, macro_idx }));
                     }
                }
            }
            "oneshotk" => {
                if args.len() == 2 {
                     let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                     let action = parse_descriptor_internal(args[1], config)?;
                     config.descriptors.push(action);
                     return Ok(Descriptor(Op::OneshotK { layer_idx, action_idx: config.descriptors.len() - 1 }));
                }
            }
            "layer" => {
                if args.len() == 1 {
                    let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                    return Ok(Descriptor(Op::Layer { layer_idx }));
                }
            }
            "layerm" => {
                if args.len() == 2 {
                    let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                    let m = parse_descriptor_internal(args[1], config)?;
                    if let Op::Macro { macro_idx } = m.0 {
                        return Ok(Descriptor(Op::LayerM { layer_idx, macro_idx }));
                    }
                }
            }
            "toggle" => {
                if args.len() == 1 {
                    let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                    return Ok(Descriptor(Op::Toggle { layer_idx }));
                }
            }
            "togglem" => {
                if args.len() == 2 {
                    let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                    let m = parse_descriptor_internal(args[1], config)?;
                    if let Op::Macro { macro_idx } = m.0 {
                        return Ok(Descriptor(Op::ToggleM { layer_idx, macro_idx }));
                    }
                }
            }
            "swap" => {
                if args.len() == 1 {
                    let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                    return Ok(Descriptor(Op::Swap { layer_idx }));
                }
            }
            "swapm" => {
                if args.len() == 2 {
                    let layer_idx = config.layers.iter().position(|l| l.name == args[0]).ok_or(format!("Unknown layer {}", args[0]))?;
                    let m = parse_descriptor_internal(args[1], config)?;
                    if let Op::Macro { macro_idx } = m.0 {
                        return Ok(Descriptor(Op::SwapM { layer_idx, macro_idx }));
                    }
                }
            }
            "timeout" => {
                if args.len() == 3 {
                    let action1 = parse_descriptor_internal(args[0], config)?;
                    let timeout: u16 = args[1].parse().unwrap_or(0);
                    let action2 = parse_descriptor_internal(args[2], config)?;
                    config.descriptors.push(action1);
                    let action1_idx = config.descriptors.len() - 1;
                    config.descriptors.push(action2);
                    let action2_idx = config.descriptors.len() - 1;
                    return Ok(Descriptor(Op::Timeout { action1_idx, timeout, action2_idx }));
                }
            }
            "clear" => {
                return Ok(Descriptor(Op::Clear));
            }
            "repeat" => {
                return Ok(Descriptor(Op::Repeat));
            }
            _ => {
                return Err(format!("Unknown function {}", name));
            }
        }
    }

    // Fallback: treat as a simple macro if it's not a known key or function
    if let Ok(m) = macro_parse(input, config) {
        config.macros.push(m);
        return Ok(Descriptor(Op::Macro { macro_idx: config.macros.len() - 1 }));
    }

    Err(format!("Invalid descriptor: {}", input))
}

pub fn parse_config(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut ctx = ParserContext::new();
    ctx.config.path = path.to_string_lossy().to_string();

    // Default layers
    let default_layers = ["main", "shift", "control", "meta", "alt", "altgr"];
    for name in default_layers {
        ctx.config.layers.push(Layer {
            name: name.to_string(),
            ..Default::default()
        });
    }

    let mut current_section = String::new();

    // Pass 1: Identify layers
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            if current_section != "ids" && current_section != "aliases" && current_section != "global" {
                if current_section.contains('+') && !current_section.contains('=') {
                     // Composite layer
                     let name = current_section.clone();
                     if !ctx.config.layers.iter().any(|l| l.name == name) {
                         let constituents: Vec<String> = current_section.split('+').map(|s| s.to_string()).collect();
                         ctx.config.layers.push(Layer {
                             name,
                             layer_type: LayerType::Composite,
                             ..Default::default()
                         });
                         // We'll resolve constituent indices in a 3rd pass or at the end of 1st
                     }
                } else {
                    let parts: Vec<&str> = current_section.split(':').collect();
                    let name = parts[0];
                    if let Some(layer_idx) = ctx.config.layers.iter().position(|l| l.name == name) {
                         if parts.len() > 1 {
                             if parts[1] == "layout" {
                                 ctx.config.layers[layer_idx].layer_type = LayerType::Layout;
                             } else {
                                 ctx.config.layers[layer_idx].mods = parse_modset(parts[1]).unwrap_or(0);
                             }
                         }
                    } else {
                        let mut layer = Layer {
                            name: name.to_string(),
                            ..Default::default()
                        };
                        if parts.len() > 1 {
                            if parts[1] == "layout" {
                                layer.layer_type = LayerType::Layout;
                            } else {
                                layer.mods = parse_modset(parts[1]).unwrap_or(0);
                            }
                        }
                        ctx.config.layers.push(layer);
                    }
                }
            }
        }
    }

    // Resolve composite constituents
    for i in 0..ctx.config.layers.len() {
        if ctx.config.layers[i].layer_type == LayerType::Composite {
            let name = ctx.config.layers[i].name.clone();
            let constituents: Vec<usize> = name.split('+')
                .filter_map(|n| ctx.config.layers.iter().position(|l| l.name == n))
                .collect();
            ctx.config.layers[i].constituents = constituents;
        }
    }

    // Pass 2: Populate mappings
    current_section = String::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        if let Some(idx) = line.find('=') {
            let key = line[..idx].trim();
            let val = line[idx + 1..].trim();

            match current_section.as_str() {
                "global" => {
                    match key {
                        "macro_timeout" => ctx.config.macro_timeout = val.parse().unwrap_or(600),
                        "oneshot_timeout" => ctx.config.oneshot_timeout = val.parse().unwrap_or(800),
                        "chord_timeout" => ctx.config.chord_interkey_timeout = val.parse().unwrap_or(50),
                        "chord_hold_timeout" => ctx.config.chord_hold_timeout = val.parse().unwrap_or(0),
                        "default_layout" => ctx.config.default_layout = val.to_string(),
                        _ => {}
                    }
                }
                "aliases" => {
                    if let Some(code) = lookup_keycode(key) {
                        ctx.config.aliases[code as usize] = val.to_string();
                    }
                }
                "ids" => {
                    // TODO: Implement ID matching
                }
                _ => {
                    let parts: Vec<&str> = current_section.split(':').collect();
                    let layer_name = parts[0];
                    if let Some(layer_idx) = ctx.config.layers.iter().position(|l| l.name == layer_name) {
                        if key.contains('+') {
                            let mut chord_keys = Vec::new();
                            for k in key.split('+') {
                                if let Some(code) = lookup_keycode(k) {
                                    chord_keys.push(code);
                                }
                            }
                            if !chord_keys.is_empty() {
                                match parse_descriptor_internal(val, &mut ctx.config) {
                                    Ok(desc) => ctx.config.layers[layer_idx].chords.push(crate::Chord { keys: chord_keys, descriptor: desc }),
                                    Err(e) => eprintln!("Error parsing chord descriptor {}: {}", val, e),
                                }
                            }
                        } else if let Some(code) = lookup_keycode(key) {
                            match parse_descriptor_internal(val, &mut ctx.config) {
                                Ok(desc) => ctx.config.layers[layer_idx].keymap[code as usize] = Some(desc),
                                Err(e) => eprintln!("Error parsing descriptor {}: {}", val, e),
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ctx.config)
}
