use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

use crate::state::AppState;

const BUFFER_CAP: usize = 64 * 1024;
const BROADCAST_CAP: usize = 256;

pub struct PtyHandle {
    writer_tx: mpsc::UnboundedSender<WriterCmd>,
    broadcast: broadcast::Sender<String>,
    buffer: Arc<Mutex<VecDeque<u8>>>,
    cwd: String,
}

impl PtyHandle {
    pub fn writer_tx_kill(&self) -> Result<(), ()> {
        self.writer_tx.send(WriterCmd::Kill).map_err(|_| ())
    }
}

enum WriterCmd {
    Data(Vec<u8>),
    Resize { cols: u16, rows: u16 },
    Kill,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum PtyIn {
    #[serde(rename = "data")]
    Data { data: String },
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
    #[serde(rename = "kill")]
    Kill,
}

#[derive(Deserialize)]
pub struct PtyQuery {
    pub session: Option<String>,
    pub cwd: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub shell: Option<String>,
}

pub async fn handle_ws_with_query(
    ws: WebSocket,
    state: AppState,
    q: PtyQuery,
) {
    let session_id = q
        .session
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let handle = match get_or_create_session(&state, &session_id, &q) {
        Ok(h) => h,
        Err(e) => {
            error!(error = %e, "failed to init pty session");
            return;
        }
    };
    info!(session = %session_id, cwd = %handle.cwd, "pty WS attached");

    let (mut ws_sender, mut ws_receiver) = ws.split();

    // replay buffer
    let initial = {
        let b = handle.buffer.lock().unwrap();
        b.iter().copied().collect::<Vec<u8>>()
    };
    if !initial.is_empty() {
        let s = String::from_utf8_lossy(&initial).to_string();
        if ws_sender.send(Message::Text(s)).await.is_err() {
            return;
        }
    }

    let mut rx = handle.broadcast.subscribe();
    let writer_tx = handle.writer_tx.clone();

    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    if ws_sender.send(Message::Text(chunk)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(skipped = n, "pty broadcast lagged");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    while let Some(msg) = ws_receiver.next().await {
        let Ok(msg) = msg else { break };
        match msg {
            Message::Text(t) => match serde_json::from_str::<PtyIn>(&t) {
                Ok(PtyIn::Data { data }) => {
                    let _ = writer_tx.send(WriterCmd::Data(data.into_bytes()));
                }
                Ok(PtyIn::Resize { cols, rows }) => {
                    let _ = writer_tx.send(WriterCmd::Resize { cols, rows });
                }
                Ok(PtyIn::Kill) => {
                    let _ = writer_tx.send(WriterCmd::Kill);
                    state.pty_sessions.remove(&session_id);
                    break;
                }
                Err(e) => warn!(error = %e, "pty bad payload"),
            },
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    info!(session = %session_id, "pty WS detached (process keeps running)");
}

fn get_or_create_session(
    state: &AppState,
    session_id: &str,
    q: &PtyQuery,
) -> anyhow::Result<Arc<PtyHandle>> {
    if let Some(h) = state.pty_sessions.get(session_id) {
        return Ok(h.clone());
    }

    let cwd = q.cwd.clone().unwrap_or_else(|| state.default_cwd.clone());
    let cols = q.cols.unwrap_or(100);
    let rows = q.rows.unwrap_or(30);
    let shell = q
        .shell
        .clone()
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/bash".to_string());

    let handle = spawn_pty(session_id.to_string(), cwd, cols, rows, shell)?;
    let arc = Arc::new(handle);
    state.pty_sessions.insert(session_id.to_string(), arc.clone());
    Ok(arc)
}

fn spawn_pty(
    session_id: String,
    cwd: String,
    cols: u16,
    rows: u16,
    shell: String,
) -> anyhow::Result<PtyHandle> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(&cwd);
    cmd.env(
        "TERM",
        std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".into()),
    );
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }

    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let writer = pair.master.take_writer()?;
    let master: Arc<Mutex<Box<dyn MasterPty + Send>>> = Arc::new(Mutex::new(pair.master));
    let reader = master.lock().unwrap().try_clone_reader()?;

    info!(%session_id, %cwd, %shell, cols, rows, "pty spawned");

    let (bc_tx, _) = broadcast::channel::<String>(BROADCAST_CAP);
    let buffer = Arc::new(Mutex::new(VecDeque::<u8>::with_capacity(BUFFER_CAP)));

    // reader thread
    {
        let tx = bc_tx.clone();
        let buf = buffer.clone();
        let sid = session_id.clone();
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut tmp = [0u8; 4096];
            loop {
                match reader.read(&mut tmp) {
                    Ok(0) => break,
                    Ok(n) => {
                        // ring buffer
                        {
                            let mut b = buf.lock().unwrap();
                            for &byte in &tmp[..n] {
                                if b.len() == BUFFER_CAP {
                                    b.pop_front();
                                }
                                b.push_back(byte);
                            }
                        }
                        let chunk = String::from_utf8_lossy(&tmp[..n]).to_string();
                        let _ = tx.send(chunk);
                    }
                    Err(_) => break,
                }
            }
            debug!(%sid, "pty reader exited");
        });
    }

    // writer task
    let (writer_tx, mut writer_rx) = mpsc::unbounded_channel::<WriterCmd>();
    {
        let master = master.clone();
        let sid = session_id.clone();
        std::thread::spawn(move || {
            let mut writer = writer;
            while let Some(cmd) = writer_rx.blocking_recv() {
                match cmd {
                    WriterCmd::Data(bytes) => {
                        if writer.write_all(&bytes).is_err() {
                            break;
                        }
                        let _ = writer.flush();
                    }
                    WriterCmd::Resize { cols, rows } => {
                        if let Ok(m) = master.lock() {
                            let _ = m.resize(PtySize {
                                cols,
                                rows,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        }
                    }
                    WriterCmd::Kill => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
            }
            debug!(%sid, "pty writer exited");
        });
    }

    Ok(PtyHandle {
        writer_tx,
        broadcast: bc_tx,
        buffer,
        cwd,
    })
}
