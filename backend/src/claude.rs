use std::process::Stdio;

use anyhow::{Context, Result};
use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{error, info, warn};

use crate::state::AppState;

#[derive(Deserialize)]
struct WsIncoming {
    prompt: String,
    cwd: Option<String>,
    session_id: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum WsOutgoing<'a> {
    #[serde(rename = "start")]
    Start { session_id: &'a str, cwd: &'a str },
    #[serde(rename = "event")]
    Event { data: Value },
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "done")]
    Done {
        exit_code: i32,
        duration_ms: u128,
        session_id: Option<String>,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

pub async fn handle_ws(mut socket: WebSocket, state: AppState) {
    info!("WS connected");
    while let Some(msg) = socket.recv().await {
        let Ok(msg) = msg else {
            warn!("WS recv error, closing");
            break;
        };
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => {
                info!("WS closed by client");
                break;
            }
            Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => continue,
        };

        let parsed: WsIncoming = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                let _ = socket
                    .send(Message::Text(
                        serde_json::to_string(&WsOutgoing::Error {
                            message: format!("bad payload: {e}"),
                        })
                        .unwrap(),
                    ))
                    .await;
                continue;
            }
        };

        let cwd = parsed.cwd.unwrap_or_else(|| state.default_cwd.clone());
        let resume_sid = parsed.session_id.clone();

        info!(
            resume = ?resume_sid.as_deref(),
            cwd = %cwd,
            prompt_chars = parsed.prompt.len(),
            "starting claude"
        );

        if let Err(e) =
            stream_claude(&mut socket, &parsed.prompt, &cwd, resume_sid.as_deref(), &state).await
        {
            error!(error = %e, "claude stream failed");
            let _ = socket
                .send(Message::Text(
                    serde_json::to_string(&WsOutgoing::Error {
                        message: e.to_string(),
                    })
                    .unwrap(),
                ))
                .await;
        }
    }
    info!("WS disconnected");
}

async fn stream_claude(
    socket: &mut WebSocket,
    prompt: &str,
    cwd: &str,
    resume_sid: Option<&str>,
    state: &AppState,
) -> Result<()> {
    let start = std::time::Instant::now();

    socket
        .send(Message::Text(serde_json::to_string(&WsOutgoing::Start {
            session_id: resume_sid.unwrap_or("new"),
            cwd,
        })?))
        .await?;

    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--dangerously-skip-permissions")
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if let Some(rid) = resume_sid {
        info!(resume = %rid, "resuming claude session");
        cmd.arg("--resume").arg(rid);
    } else {
        info!("starting new claude session (no resume)");
    }

    let mut child = cmd.spawn().context("spawn claude")?;
    let stdout = child.stdout.take().context("take stdout")?;
    let stderr = child.stderr.take().context("take stderr")?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut claude_session_id: Option<String> = None;

    loop {
        tokio::select! {
            line = stdout_reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        if l.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<Value>(&l) {
                            Ok(v) => {
                                if let Some(sid) = v.get("session_id").and_then(|s| s.as_str()) {
                                    claude_session_id = Some(sid.to_string());
                                }
                                if let Some(delta) = extract_text_delta(&v) {
                                    socket
                                        .send(Message::Text(serde_json::to_string(
                                            &WsOutgoing::TextDelta { text: delta },
                                        )?))
                                        .await?;
                                }
                                socket
                                    .send(Message::Text(serde_json::to_string(
                                        &WsOutgoing::Event { data: v },
                                    )?))
                                    .await?;
                            }
                            Err(e) => {
                                warn!(line = %l, error = %e, "claude stdout not JSON");
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        warn!(error = %e, "stdout read error");
                        break;
                    }
                }
            }
            line = stderr_reader.next_line() => {
                if let Ok(Some(l)) = line {
                    warn!(claude_stderr = %l);
                }
            }
        }
    }

    let status = child.wait().await.context("wait claude")?;
    let exit_code = status.code().unwrap_or(-1);
    let duration_ms = start.elapsed().as_millis();

    if let Some(sid) = &claude_session_id {
        state.sessions.insert(sid.clone(), sid.clone());
    }

    info!(
        resumed_from = ?resume_sid,
        claude_session = ?claude_session_id,
        exit_code,
        duration_ms,
        "claude finished"
    );

    socket
        .send(Message::Text(serde_json::to_string(&WsOutgoing::Done {
            exit_code,
            duration_ms,
            session_id: claude_session_id,
        })?))
        .await?;

    Ok(())
}

fn extract_text_delta(v: &Value) -> Option<String> {
    let t = v.get("type")?.as_str()?;
    if t != "assistant" {
        return None;
    }
    let message = v.get("message")?;
    let content = message.get("content")?.as_array()?;
    let mut out = String::new();
    for block in content {
        if block.get("type").and_then(|s| s.as_str()) == Some("text") {
            if let Some(text) = block.get("text").and_then(|s| s.as_str()) {
                out.push_str(text);
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub async fn run_once(
    prompt: &str,
    cwd: &str,
    session_id: Option<&str>,
) -> Result<Value> {
    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("json")
        .arg("--dangerously-skip-permissions")
        .current_dir(cwd);
    if let Some(sid) = session_id {
        cmd.arg("--resume").arg(sid);
    }
    let out = cmd.output().await.context("spawn claude")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("claude exit {}: {err}", out.status);
    }
    let body = String::from_utf8_lossy(&out.stdout);
    let v: Value = serde_json::from_str(&body).context("parse claude json")?;
    Ok(v)
}
