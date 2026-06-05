use crate::vkbd::VirtualKeyboard;
use crate::keys::KEYCODE_TABLE;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, RelativeAxisCode, AbsoluteAxisCode, InputEvent, EventType, InputId, BusType, KeyCode};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

pub struct UinputBackend {
    kbd: Mutex<VirtualDevice>,
    ptr: Mutex<VirtualDevice>,
}

impl UinputBackend {
    pub fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut kbd_keys = AttributeSet::<KeyCode>::new();
        for (code, ent) in KEYCODE_TABLE.iter().enumerate() {
            if ent.is_some() {
                kbd_keys.insert(KeyCode::new(code as u16));
            }
        }
        // Add ZOOM (177 in keyd)
        kbd_keys.insert(KeyCode::new(177));

        let kbd = VirtualDevice::builder()?
            .name(name)
            .input_id(InputId::new(BusType::BUS_USB, 0x0FAC, 0x0ADE, 0))
            .with_keys(&kbd_keys)?
            .build()?;

        let mut ptr_keys = AttributeSet::<KeyCode>::new();
        // BTN_LEFT (0x110) to BTN_TASK (0x117)
        for code in 0x110..=0x117 {
            ptr_keys.insert(KeyCode::new(code));
        }

        let mut ptr_rel = AttributeSet::<RelativeAxisCode>::new();
        ptr_rel.insert(RelativeAxisCode::REL_X);
        ptr_rel.insert(RelativeAxisCode::REL_Y);
        ptr_rel.insert(RelativeAxisCode::REL_WHEEL);
        ptr_rel.insert(RelativeAxisCode::REL_HWHEEL);
        ptr_rel.insert(RelativeAxisCode::REL_Z);

        let mut ptr_abs = AttributeSet::<AbsoluteAxisCode>::new();
        ptr_abs.insert(AbsoluteAxisCode::ABS_X);
        ptr_abs.insert(AbsoluteAxisCode::ABS_Y);

        let ptr = VirtualDevice::builder()?
            .name("keyd virtual pointer")
            .input_id(InputId::new(BusType::BUS_USB, 0x0FAC, 0x1ADE, 0))
            .with_keys(&ptr_keys)?
            .with_relative_axes(&ptr_rel)?
            .build()?;

        Ok(Self {
            kbd: Mutex::new(kbd),
            ptr: Mutex::new(ptr),
        })
    }
}

impl VirtualKeyboard for UinputBackend {
    fn send_key(&self, code: u8, state: i32) {
        let ev_code;
        let mut is_btn = true;

        match code {
            249 => ev_code = 0x110, // KEYD_LEFT_MOUSE -> BTN_LEFT
            250 => ev_code = 0x111, // KEYD_MIDDLE_MOUSE -> BTN_MIDDLE
            251 => ev_code = 0x112, // KEYD_RIGHT_MOUSE -> BTN_RIGHT
            252 => ev_code = 0x113, // KEYD_MOUSE_1 -> BTN_SIDE
            253 => ev_code = 0x114, // KEYD_MOUSE_2 -> BTN_EXTRA
            178 => ev_code = 0x115, // KEYD_MOUSE_BACK -> BTN_BACK
            255 => ev_code = 0x116, // KEYD_MOUSE_FORWARD -> BTN_FORWARD
            177 => { ev_code = 177; is_btn = false; }, // KEYD_ZOOM -> KEY_ZOOM
            222 => { ev_code = 222; is_btn = false; }, // KEYD_VOICECOMMAND -> KEY_VOICECOMMAND
            _ => { ev_code = code as u16; is_btn = false; }
        }

        if is_btn {
            sleep(Duration::from_millis(1));
            let mut ptr = self.ptr.lock().unwrap();
            ptr.emit(&[InputEvent::new(EventType::KEY.0, ev_code, state)]).unwrap();
        } else {
            let mut kbd = self.kbd.lock().unwrap();
            kbd.emit(&[InputEvent::new(EventType::KEY.0, ev_code, state)]).unwrap();
        }
    }

    fn mouse_move(&self, x: i32, y: i32) {
        let mut events = Vec::new();
        if x != 0 {
            events.push(InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, x));
        }
        if y != 0 {
            events.push(InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, y));
        }
        if !events.is_empty() {
            let mut ptr = self.ptr.lock().unwrap();
            ptr.emit(&events).unwrap();
        }
    }

    fn mouse_scroll(&self, x: i32, y: i32) {
        let mut ptr = self.ptr.lock().unwrap();
        ptr.emit(&[
            InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_WHEEL.0, y),
            InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_HWHEEL.0, x),
        ]).unwrap();
    }

    fn mouse_move_abs(&self, x: i32, y: i32) {
        let mut events = Vec::new();
        if x != 0 {
            events.push(InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, x));
        }
        if y != 0 {
            events.push(InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, y));
        }
        if !events.is_empty() {
            let mut ptr = self.ptr.lock().unwrap();
            ptr.emit(&events).unwrap();
        }
    }
}
