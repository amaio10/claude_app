mod claude;
mod fs;
mod pty;
mod state;
mod transcribe;

use std::net::SocketAddr;

use axum::{
    extract::{Multipart, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

use state::AppState;

#[derive(Deserialize)]
struct PromptBody {
    prompt: String,
    cwd: Option<String>,
    session_id: Option<String>,
}

#[derive(Serialize)]
struct HealthResp {
    ok: bool,
    version: &'static str,
    groq_configured: bool,
    default_cwd: String,
}

async fn health(State(state): State<AppState>) -> Json<HealthResp> {
    Json(HealthResp {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
        groq_configured: !state.groq_api_key.is_empty(),
        default_cwd: state.default_cwd.clone(),
    })
}

async fn prompt_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| claude::handle_ws(socket, state))
}

async fn pty_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<pty::PtyQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| pty::handle_ws_with_query(socket, state, q))
}

async fn list_pty_sessions(
    State(state): State<AppState>,
) -> Json<Vec<String>> {
    Json(state.pty_sessions.iter().map(|e| e.key().clone()).collect())
}

async fn kill_pty_session(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    if let Some((_, h)) = state.pty_sessions.remove(&id) {
        let _ = h.writer_tx_kill();
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn transcribe_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<transcribe::TranscribeResp>, (StatusCode, String)> {
    let mut audio_bytes: Option<Vec<u8>> = None;
    let mut filename = String::from("audio.webm");
    let mut mime = String::from("audio/webm");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart: {e}")))?
    {
        if field.name() == Some("audio") {
            if let Some(name) = field.file_name() {
                filename = name.to_string();
            }
            if let Some(ct) = field.content_type() {
                mime = ct.to_string();
            }
            let bytes = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("read audio: {e}")))?;
            audio_bytes = Some(bytes.to_vec());
        }
    }

    let audio = audio_bytes
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing audio field".to_string()))?;

    info!(
        bytes = audio.len(),
        filename = %filename,
        mime = %mime,
        "received audio for transcription"
    );

    let resp = transcribe::transcribe_groq(&state.groq_api_key, audio, &filename, &mime)
        .await
        .map_err(|e| {
            error!(error = %e, "transcription failed");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("transcribe: {e}"))
        })?;

    Ok(Json(resp))
}

async fn prompt_handler(
    State(state): State<AppState>,
    Json(body): Json<PromptBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let cwd = body.cwd.unwrap_or_else(|| state.default_cwd.clone());
    let session_id = body.session_id;
    info!(
        prompt_preview = %body.prompt.chars().take(80).collect::<String>(),
        cwd = %cwd,
        session_id = ?session_id,
        "prompt (non-streaming) received"
    );
    let response = claude::run_once(&body.prompt, &cwd, session_id.as_deref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("claude: {e}")))?;
    Ok(Json(response))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let _ = dotenvy::from_filename("../.env");

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,tower_http=warn")),
        )
        .with_target(false)
        .with_level(true)
        .init();

    let state = AppState::new()?;
    info!(
        cwd = %state.default_cwd,
        groq_configured = %(!state.groq_api_key.is_empty()),
        "app state initialized"
    );

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/prompt", post(prompt_handler))
        .route("/api/transcribe", post(transcribe_handler))
        .route("/api/fs/list", get(fs::list_dir))
        .route("/api/fs/home", get(fs::home_dir))
        .route("/api/fs/read", get(fs::read_file))
        .route("/api/fs/raw", get(fs::raw_file))
        .route("/api/fs/write", post(fs::write_file))
        .route("/ws", get(prompt_ws))
        .route("/pty", get(pty_ws))
        .route("/api/pty/sessions", get(list_pty_sessions))
        .route("/api/pty/:id", axum::routing::delete(kill_pty_session))
        .layer(CorsLayer::very_permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = "127.0.0.1:7777".parse()?;
    info!(%addr, "claude_app backend listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
