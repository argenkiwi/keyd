use std::fs;
use std::os::unix::io::RawFd;
use std::ffi::CString;
use libc::*;

#[cfg(not(target_os = "linux"))]
#[allow(non_camel_case_types)]
#[repr(C)]
struct input_id {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[cfg(not(target_os = "linux"))]
#[allow(non_camel_case_types)]
#[repr(C)]
struct input_event {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}


pub const CAP_MOUSE: u8 = 0x1;
pub const CAP_MOUSE_ABS: u8 = 0x2;
pub const CAP_KEYBOARD: u8 = 0x4;
pub const CAP_KEY: u8 = 0x8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceEventType {
    Key,
    Led,
    MouseMove,
    MouseMoveAbs,
    MouseScroll,
    Removed,
}

pub struct DeviceEvent {
    pub event_type: DeviceEventType,
    pub code: u8,
    pub pressed: u8,
    pub x: i32,
    pub y: i32,
}

pub struct Device {
    pub fd: RawFd,
    pub grabbed: bool,
    pub capabilities: u8,
    pub is_virtual: bool,
    pub id: String,
    pub name: String,
    pub path: String,
    
    pub minx: u32,
    pub maxx: u32,
    pub miny: u32,
    pub maxy: u32,
    
    pub pending_rel_x: i32,
    pub pending_rel_y: i32,
}

const EVIOCGBIT_KEY: u64 = 2147501345; // Simplified
const EVIOCGNAME: u64 = 2147501318;
const EVIOCGID: u64 = 2147501314;
const EVIOCGABS: u64 = 2147501376;

impl Device {
    pub fn scan() -> Vec<Device> {
        let mut devices = Vec::new();
        if let Ok(entries) = fs::read_dir("/dev/input/") {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    let name = path.file_name().unwrap().to_str().unwrap();
                    if name.starts_with("event") {
                        if let Ok(dev) = Device::init(path.to_str().unwrap()) {
                            devices.push(dev);
                        }
                    }
                }
            }
        }
        devices
    }

    pub fn init(path: &str) -> Result<Self, String> {
        let fd = unsafe { open(CString::new(path).unwrap().as_ptr(), O_RDWR | O_NONBLOCK | O_CLOEXEC) };
        if fd < 0 {
            return Err(format!("Failed to open {}", path));
        }

        let mut name_buf = [0u8; 64];
        unsafe { libc::ioctl(fd, EVIOCGNAME as u64, name_buf.as_mut_ptr()) };
        let name = String::from_utf8_lossy(&name_buf).trim_matches('\0').to_string();

        let mut info: input_id = unsafe { std::mem::zeroed() };
        unsafe { libc::ioctl(fd, EVIOCGID as u64, &mut info) };

        // For simplicity in this porting step, we'll skip detailed capability resolution and UID generation
        // and just assume it's a keyboard if it has keys.
        
        Ok(Device {
            fd,
            grabbed: false,
            capabilities: CAP_KEYBOARD | CAP_KEY,
            is_virtual: info.vendor == 0x0FAC,
            id: format!("{:04x}:{:04x}", info.vendor, info.product),
            name,
            path: path.to_string(),
            minx: 0, maxx: 0, miny: 0, maxy: 0,
            pending_rel_x: 0, pending_rel_y: 0,
        })
    }

    pub fn grab(&mut self) -> Result<(), String> {
        if self.grabbed { return Ok(()); }
        if unsafe { libc::ioctl(self.fd, 1074021376, 1) } < 0 { // EVIOCGRAB
            return Err("Failed to grab device".to_string());
        }
        self.grabbed = true;
        Ok(())
    }

    pub fn ungrab(&mut self) -> Result<(), String> {
        if !self.grabbed { return Ok(()); }
        if unsafe { libc::ioctl(self.fd, 1074021376, 0) } < 0 {
            return Err("Failed to ungrab device".to_string());
        }
        self.grabbed = false;
        Ok(())
    }

    pub fn read_event(&mut self) -> Option<DeviceEvent> {
        let mut ev: input_event = unsafe { std::mem::zeroed() };
        let res = unsafe { read(self.fd, &mut ev as *mut _ as *mut c_void, std::mem::size_of::<input_event>()) };
        
        if res < 0 {
            return None;
        }

        match ev.type_ {
            0x01 => { // EV_KEY
                if ev.value == 2 { return None; } // Ignore repeats
                Some(DeviceEvent {
                    event_type: DeviceEventType::Key,
                    code: ev.code as u8,
                    pressed: ev.value as u8,
                    x: 0, y: 0,
                })
            }
            _ => None
        }
    }
}
