use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use axum::{
    body::Body,
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio_util::io::ReaderStream;
use tracing::{info, warn};

const MAX_LOG_BYTES: usize = 256 * 1024;
const MAX_PDF_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Deserialize)]
pub struct CompileBody {
    pub path: String,
}

#[derive(Serialize)]
pub struct CompileResp {
    pub ok: bool,
    pub pdf_path: Option<String>,
    pub log: String,
}

#[derive(Deserialize)]
pub struct PdfQuery {
    pub path: String,
}

fn expand(input: &str) -> PathBuf {
    if let Some(rest) = input.strip_prefix("~") {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let mut p = home;
            let trimmed = rest.trim_start_matches('/');
            if !trimmed.is_empty() {
                p.push(trimmed);
            }
            return p;
        }
    }
    PathBuf::from(input)
}

fn check_tex(canon: &Path) -> Result<(), (StatusCode, String)> {
    let ext = canon
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    if ext.as_deref() != Some("tex") {
        return Err((StatusCode::BAD_REQUEST, "not a .tex file".into()));
    }
    Ok(())
}

fn cache_dir_for(canon: &Path) -> PathBuf {
    let mut h = DefaultHasher::new();
    canon.hash(&mut h);
    let key = format!("{:016x}", h.finish());
    let stem = canon
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("doc");
    std::env::temp_dir()
        .join("claude_app_tex")
        .join(format!("{stem}-{key}"))
}

fn pdf_path_for(canon: &Path) -> PathBuf {
    let dir = cache_dir_for(canon);
    let stem = canon
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("doc");
    dir.join(format!("{stem}.pdf"))
}

fn truncate_log(s: String) -> String {
    if s.len() <= MAX_LOG_BYTES {
        return s;
    }
    let mut out = String::with_capacity(MAX_LOG_BYTES + 32);
    out.push_str("…(log truncated)…\n");
    out.push_str(&s[s.len() - MAX_LOG_BYTES..]);
    out
}

pub async fn compile(
    Json(body): Json<CompileBody>,
) -> Result<Json<CompileResp>, (StatusCode, String)> {
    let target = expand(&body.path);
    let canon = target
        .canonicalize()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("resolve: {e}")))?;

    check_tex(&canon)?;

    let meta = std::fs::metadata(&canon)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("metadata: {e}")))?;
    if !meta.is_file() {
        return Err((StatusCode::BAD_REQUEST, "not a file".into()));
    }

    let outdir = cache_dir_for(&canon);
    std::fs::create_dir_all(&outdir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("mkdir cache: {e}")))?;

    let source_parent = canon
        .parent()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "no parent dir".into()))?;

    info!(
        tex = %canon.display(),
        outdir = %outdir.display(),
        "compiling .tex via tectonic"
    );

    let out = Command::new("tectonic")
        .arg("--chatter")
        .arg("minimal")
        .arg("--keep-logs")
        .arg("--outdir")
        .arg(&outdir)
        .arg(&canon)
        .current_dir(source_parent)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("spawn tectonic: {e} (is it on PATH?)"),
            )
        })?;

    let mut log = String::new();
    log.push_str(&String::from_utf8_lossy(&out.stdout));
    if !out.stderr.is_empty() {
        if !log.is_empty() && !log.ends_with('\n') {
            log.push('\n');
        }
        log.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    let log = truncate_log(log);

    let pdf = pdf_path_for(&canon);
    let ok = out.status.success() && pdf.exists();

    if !ok {
        warn!(status = ?out.status.code(), "tectonic failed");
        return Ok(Json(CompileResp {
            ok: false,
            pdf_path: None,
            log,
        }));
    }

    Ok(Json(CompileResp {
        ok: true,
        pdf_path: Some(pdf.to_string_lossy().to_string()),
        log,
    }))
}

pub async fn pdf(Query(q): Query<PdfQuery>) -> Result<Response, (StatusCode, String)> {
    let target = expand(&q.path);
    let canon = target
        .canonicalize()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("resolve: {e}")))?;
    check_tex(&canon)?;

    let pdf = pdf_path_for(&canon);
    let meta = std::fs::metadata(&pdf).map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            "pdf not compiled yet — hit Recompile".to_string(),
        )
    })?;
    let size = meta.len();
    if size > MAX_PDF_BYTES {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "pdf too large".into()));
    }

    let file = tokio::fs::File::open(&pdf)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("open pdf: {e}")))?;
    let stream = ReaderStream::new(file);

    let filename = canon
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(header::CONTENT_LENGTH, size)
        .header(header::CACHE_CONTROL, "no-cache")
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{filename}.pdf\""),
        )
        .body(Body::from_stream(stream))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("response: {e}")))?
        .into_response())
}
