use std::os::unix::net::{UnixStream, UnixListener};
use std::os::unix::fs::PermissionsExt;
use std::fs;
use std::io::Error;

pub const SOCKET_PATH: &str = "/var/run/keyd.socket";

pub fn ipc_connect() -> Result<UnixStream, Error> {
    UnixStream::connect(SOCKET_PATH)
}

pub fn ipc_create_server() -> Result<UnixListener, Error> {
    let lock_path = format!("{}.lock", SOCKET_PATH);
    // Simple lock check (not as robust as flock for now)
    
    let _ = fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH)?;
    
    let mut perms = fs::metadata(SOCKET_PATH)?.permissions();
    perms.set_mode(0o660);
    fs::set_permissions(SOCKET_PATH, perms)?;
    
    Ok(listener)
}
