use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use std::os::unix::fs::PermissionsExt;
use std::fs;
use tracing::{info, error};

#[derive(Debug, Serialize, Deserialize)]
pub enum IpcRequest {
    Eval(String),
    Reload,
    Monitor,
    LayerListen,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IpcResponse {
    Success(String),
    Error(String),
}

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if fs::metadata(path).is_ok() {
            fs::remove_file(path)?;
        }

        let listener = UnixListener::bind(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o660))?;
        
        // TODO: Handle group ownership (keyd group)

        Ok(Self { listener })
    }

    pub async fn run(self, tx: tokio::sync::mpsc::Sender<ControlMessage>) {
        loop {
            match self.listener.accept().await {
                Ok((mut stream, _)) => {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, tx).await {
                            error!("IPC client error: {}", e);
                        }
                    });
                }
                Err(e) => error!("IPC accept error: {}", e),
            }
        }
    }
}

pub enum ControlMessage {
    Eval(String, tokio::sync::oneshot::Sender<IpcResponse>),
    Reload,
}

async fn handle_client(mut stream: UnixStream, tx: tokio::sync::mpsc::Sender<ControlMessage>) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await?;
    if n == 0 { return Ok(()); }

    // Simplified protocol: assume JSON for now (C version uses raw structs)
    if let Ok(req) = serde_json::from_slice::<IpcRequest>(&buf[..n]) {
        match req {
            IpcRequest::Eval(exp) => {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                tx.send(ControlMessage::Eval(exp, reply_tx)).await?;
                if let Ok(res) = reply_rx.await {
                    let resp_buf = serde_json::to_vec(&res)?;
                    stream.write_all(&resp_buf).await?;
                }
            }
            IpcRequest::Reload => {
                tx.send(ControlMessage::Reload).await?;
                let resp = IpcResponse::Success("Reloading".to_string());
                let resp_buf = serde_json::to_vec(&resp)?;
                stream.write_all(&resp_buf).await?;
            }
            _ => {
                let resp = IpcResponse::Error("Not implemented".to_string());
                let resp_buf = serde_json::to_vec(&resp)?;
                stream.write_all(&resp_buf).await?;
            }
        }
    }

    Ok(())
}
