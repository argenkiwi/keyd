use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};

pub const SOCKET_PATH: &str = "/var/run/keyd.socket";

/// IPC message type — numeric values match the C enum exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IpcMessageType {
    Success     = 0,
    Fail        = 1,
    Bind        = 2,
    Input       = 3,
    Macro       = 4,
    Reload      = 5,
    LayerListen = 6,
}

impl TryFrom<u32> for IpcMessageType {
    type Error = u32;
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Success),
            1 => Ok(Self::Fail),
            2 => Ok(Self::Bind),
            3 => Ok(Self::Input),
            4 => Ok(Self::Macro),
            5 => Ok(Self::Reload),
            6 => Ok(Self::LayerListen),
            x => Err(x),
        }
    }
}

/// Binary-compatible with C's `struct ipc_message` (64-bit layout):
///   u32 type | u32 timeout | u8[4096] data | u64 sz
#[repr(C)]
pub struct IpcMessage {
    pub msg_type: u32,
    pub timeout:  u32,
    pub data:     [u8; 4096],
    pub sz:       u64,
}

impl IpcMessage {
    pub fn new(msg_type: IpcMessageType, timeout: u32) -> Self {
        let mut m: Self = unsafe { std::mem::zeroed() };
        m.msg_type = msg_type as u32;
        m.timeout  = timeout;
        m
    }

    pub fn set_data(&mut self, src: &[u8]) {
        let sz = src.len().min(self.data.len());
        self.data[..sz].copy_from_slice(&src[..sz]);
        self.sz = sz as u64;
    }

    /// Borrow the payload as a str (up to sz bytes).
    pub fn data_str(&self) -> &str {
        let sz = (self.sz as usize).min(4096);
        std::str::from_utf8(&self.data[..sz]).unwrap_or("")
    }

    /// Write the entire fixed-size struct to a writer.
    pub fn write_to(&self, w: &mut dyn Write) -> io::Result<()> {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                self as *const _ as *const u8,
                std::mem::size_of::<Self>(),
            )
        };
        w.write_all(bytes)
    }

    /// Read a complete fixed-size struct from a reader.
    pub fn read_from(r: &mut dyn Read) -> io::Result<Self> {
        let mut m: Self = unsafe { std::mem::zeroed() };
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(
                &mut m as *mut _ as *mut u8,
                std::mem::size_of::<Self>(),
            )
        };
        r.read_exact(bytes)?;
        Ok(m)
    }
}

/// Create the IPC server socket, holding an exclusive lock file to prevent
/// duplicate daemons.  Returns `(listener, lock_file)` — caller must keep
/// `lock_file` alive for the lifetime of the server.
pub fn ipc_create_server() -> io::Result<(UnixListener, fs::File)> {
    let lock_path = format!("{}.lock", SOCKET_PATH);
    let lock_file = fs::OpenOptions::new()
        .read(true)
        .create(true)
        .open(&lock_path)?;

    // Non-blocking exclusive lock — fails immediately if another daemon owns it.
    let rc = unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }

    let _ = fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH)?;
    let mut perms = fs::metadata(SOCKET_PATH)?.permissions();
    perms.set_mode(0o660);
    fs::set_permissions(SOCKET_PATH, perms)?;

    Ok((listener, lock_file))
}

pub fn ipc_connect() -> io::Result<UnixStream> {
    UnixStream::connect(SOCKET_PATH)
}

/// Send one IPC message and return the daemon's response data string,
/// or an Err with the failure description if the server responds with Fail.
pub fn ipc_send_recv(
    msg_type: IpcMessageType,
    data: &[u8],
    timeout: u32,
) -> Result<String, String> {
    let mut stream = ipc_connect().map_err(|e| e.to_string())?;

    let mut msg = IpcMessage::new(msg_type, timeout);
    msg.set_data(data);
    msg.write_to(&mut stream).map_err(|e| e.to_string())?;

    let resp = IpcMessage::read_from(&mut stream).map_err(|e| e.to_string())?;
    let body = resp.data_str().to_string();
    match IpcMessageType::try_from(resp.msg_type) {
        Ok(IpcMessageType::Success) => Ok(body),
        _ => Err(body),
    }
}
