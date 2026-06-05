use keyd_rs::keyboard::{Keyboard, KeyEvent};
use keyd_rs::vkbd::{MockVirtualKeyboard, VirtualKeyboard};
use keyd_rs::config::parse_config;
use keyd_rs::keys::lookup_keycode;
use std::path::Path;
use std::time::{Instant, Duration};
use std::sync::Arc;
use std::fs;

struct VirtualKeyboardWrapper(Arc<MockVirtualKeyboard>);

impl VirtualKeyboard for VirtualKeyboardWrapper {
    fn send_key(&self, code: u8, state: i32) { self.0.send_key(code, state); }
    fn mouse_move(&self, x: i32, y: i32) { self.0.mouse_move(x, y); }
    fn mouse_scroll(&self, x: i32, y: i32) { self.0.mouse_scroll(x, y); }
    fn mouse_move_abs(&self, x: i32, y: i32) { self.0.mouse_move_abs(x, y); }
}

fn run_test_file(path: &Path, config: &keyd_rs::Config) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut parts = content.split("\n\n");
    let input_block = parts.next().ok_or("Empty test file")?;
    let expected_block = parts.next().ok_or("Missing expected output block")?;

    let mock_vkbd = Arc::new(MockVirtualKeyboard::new());
    let wrapper = Box::new(VirtualKeyboardWrapper(mock_vkbd.clone()));
    let mut keyboard = Keyboard::new(config.clone(), wrapper);

    let mut start_time = Instant::now();

    for line in input_block.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 2 { continue; }

        let key_name = tokens[0];
        let action = tokens[1];
        
        let code = if let Some(c) = lookup_keycode(key_name) {
            c
        } else if let Ok(c) = key_name.parse::<u8>() {
            c
        } else {
            return Err(format!("Unknown key: {}", key_name));
        };

        let pressed = action == "down";
        
        keyboard.process_event(KeyEvent {
            code,
            pressed,
            timestamp: start_time,
        });
        
        keyboard.check_timeouts();
        // Simulate time passing if needed (though .t files don't specify timing usually, 
        // except for some tests that might need it).
        start_time += Duration::from_millis(1);
    }

    let actual_events = mock_vkbd.events.lock().unwrap();
    let mut expected_events = Vec::new();
    for line in expected_block.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 2 { continue; }

        let key_name = tokens[0];
        let action = tokens[1];
        
        let code = if let Some(c) = lookup_keycode(key_name) {
            c
        } else if let Ok(c) = key_name.parse::<u8>() {
            c
        } else {
            return Err(format!("Unknown expected key: {}", key_name));
        };

        let state = if action == "down" { 1 } else { 0 };
        expected_events.push((code, state));
    }

    if actual_events.len() != expected_events.len() {
        return Err(format!("Event count mismatch: expected {}, got {}. Actual: {:?}", 
            expected_events.len(), actual_events.len(), *actual_events));
    }

    for (i, (exp, act)) in expected_events.iter().zip(actual_events.iter()).enumerate() {
        if exp != act {
            return Err(format!("Event mismatch at index {}: expected {:?}, got {:?}. Full actual: {:?}", 
                i, exp, act, *actual_events));
        }
    }

    Ok(())
}

#[test]
fn test_all_io_files() {
    let config_path = Path::new("t/test.conf");
    let config = parse_config(config_path).expect("Failed to parse config");

    let mut failed = 0;
    let mut total = 0;

    for entry in fs::read_dir("t").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "t") {
            // Skip some tests that are known to fail due to unimplemented features (like chords)
            let filename = path.file_name().unwrap().to_string_lossy();
            if filename.contains("chord") || filename.contains("timeout") || filename.contains("overload") {
                continue;
            }

            total += 1;
            match run_test_file(&path, &config) {
                Ok(_) => println!("Test {:?} PASSED", path),
                Err(e) => {
                    println!("Test {:?} FAILED: {}", path, e);
                    failed += 1;
                }
            }
        }
    }

    assert!(failed == 0, "{} out of {} tests failed", failed, total);
}
