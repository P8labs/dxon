use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::commands::open;
use crate::config::Config;
use crate::container::store::ContainerStore;

pub const CONTAINER_SOCKET_PATH: &str = "/run/dxon.sock";

#[derive(Debug, Serialize, Deserialize)]
struct OpenRequest {
    method: String,
    container: String,
    path: String,
    #[serde(default)]
    editor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenResponse {
    ok: bool,
    message: String,
}

pub struct HostSocketServer {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl HostSocketServer {
    pub fn start(containers_dir: PathBuf) -> Result<Self> {
        let socket_path = host_socket_path_from_containers_base(&containers_dir);

        if let Some(parent) = socket_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if socket_path.exists() {
            let _ = fs::remove_file(&socket_path);
        }

        let listener = UnixListener::bind(&socket_path)?;
        listener.set_nonblocking(true)?;

        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let socket_for_cleanup = socket_path.clone();

        let handle = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Err(e) = handle_connection(stream, &containers_dir) {
                            eprintln!("warn: dxon IPC request failed: {e}");
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(80));
                    }
                    Err(err) => {
                        eprintln!("warn: dxon IPC accept error: {err}");
                        thread::sleep(Duration::from_millis(120));
                    }
                }
            }

            let _ = fs::remove_file(socket_for_cleanup);
        });

        Ok(Self {
            stop,
            handle: Some(handle),
        })
    }
}

impl Drop for HostSocketServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn host_socket_path_from_containers_base(containers_base: &Path) -> PathBuf {
    if containers_base
        .file_name()
        .is_some_and(|n| n == "containers")
    {
        if let Some(parent) = containers_base.parent() {
            return parent.join("dxon.sock");
        }
    }

    containers_base.join("dxon.sock")
}

pub fn send_request<TReq, TRes>(socket_path: &Path, request: &TReq) -> Result<TRes>
where
    TReq: Serialize,
    TRes: DeserializeOwned,
{
    let mut stream = UnixStream::connect(socket_path).map_err(|e| {
        anyhow::anyhow!(
            "failed to connect to host IPC socket at {}: {}",
            socket_path.display(),
            e
        )
    })?;

    let raw = serde_json::to_vec(request)?;
    stream.write_all(&raw)?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut line)?;
    if line.trim().is_empty() {
        anyhow::bail!("empty response from host IPC server");
    }

    Ok(serde_json::from_str(line.trim_end())?)
}

fn handle_connection(stream: UnixStream, containers_dir: &Path) -> Result<()> {
    let mut line = String::new();
    let mut reader = BufReader::new(stream.try_clone()?);
    reader.read_line(&mut line)?;

    let req: OpenRequest = serde_json::from_str(line.trim_end())?;

    let response = match req.method.as_str() {
        "open" => handle_open(req, containers_dir),
        other => OpenResponse {
            ok: false,
            message: format!("unsupported RPC method '{other}'"),
        },
    };

    let mut writer = stream;
    writer.write_all(serde_json::to_string(&response)?.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn handle_open(req: OpenRequest, containers_dir: &Path) -> OpenResponse {
    let mut cfg = match Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            return OpenResponse {
                ok: false,
                message: format!("failed to load host config: {e}"),
            }
        }
    };

    let store = match ContainerStore::new(containers_dir.to_path_buf()) {
        Ok(s) => s,
        Err(e) => {
            return OpenResponse {
                ok: false,
                message: format!("failed to open container store: {e}"),
            }
        }
    };

    match open::run_from_rpc(
        &store,
        &mut cfg,
        &req.container,
        &req.path,
        req.editor.as_deref(),
    ) {
        Ok(()) => OpenResponse {
            ok: true,
            message: format!(
                "opened host editor for {}/{}",
                req.container,
                req.path.trim_start_matches('/')
            ),
        },
        Err(e) => OpenResponse {
            ok: false,
            message: e.to_string(),
        },
    }
}
