use std::path::{Path, PathBuf};

use axum::{
    body::Body,
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;
use tracing::{debug, info, warn};

const MAX_READ_BYTES: u64 = 2 * 1024 * 1024; // 2MB
const MAX_RAW_BYTES: u64 = 64 * 1024 * 1024; // 64MB for binary viewers (STL, etc.)

#[derive(Deserialize)]
pub struct ListQuery {
    pub path: Option<String>,
    pub show_hidden: Option<bool>,
    pub files: Option<bool>,
}

#[derive(Serialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<String>,
}

#[derive(Deserialize)]
pub struct ReadQuery {
    pub path: String,
}

#[derive(Serialize)]
pub struct ReadResp {
    pub path: String,
    pub size: u64,
    pub content: String,
    pub truncated: bool,
}

#[derive(Deserialize)]
pub struct WriteBody {
    pub path: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct WriteResp {
    pub path: String,
    pub size: u64,
}

#[derive(Serialize)]
pub struct ListResp {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<Entry>,
}

#[derive(Serialize)]
pub struct HomeResp {
    pub home: String,
    pub quick: Vec<QuickEntry>,
}

#[derive(Serialize)]
pub struct QuickEntry {
    pub label: String,
    pub path: String,
}

fn expand(input: &str) -> PathBuf {
    if let Some(rest) = input.strip_prefix("~") {
        if let Some(home) = dirs_home() {
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

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub async fn list_dir(
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResp>, (StatusCode, String)> {
    let target = match q.path.as_deref() {
        Some(p) if !p.is_empty() => expand(p),
        _ => dirs_home()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "no HOME".into()))?,
    };

    let canon = target
        .canonicalize()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("resolve path: {e}")))?;

    if !canon.is_dir() {
        return Err((StatusCode::BAD_REQUEST, "not a directory".into()));
    }

    debug!(path = %canon.display(), "listing dir");

    let show_hidden = q.show_hidden.unwrap_or(false);
    let include_files = q.files.unwrap_or(false);
    let mut entries: Vec<Entry> = Vec::new();

    let iter = std::fs::read_dir(&canon)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("read_dir: {e}")))?;

    for item in iter.flatten() {
        let name = item.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let ft = item.file_type();
        let is_dir = ft.as_ref().map(|t| t.is_dir() || t.is_symlink()).unwrap_or(false);
        if !is_dir && !include_files {
            continue;
        }
        let path = item.path().to_string_lossy().to_string();
        let (size, ext) = if !is_dir {
            let meta = item.metadata().ok();
            let size = meta.as_ref().map(|m| m.len());
            let ext = item
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase());
            (size, ext)
        } else {
            (None, None)
        };
        entries.push(Entry {
            name,
            path,
            is_dir,
            size,
            ext,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    let parent = canon
        .parent()
        .filter(|p| p.as_os_str() != canon.as_os_str())
        .map(|p| p.to_string_lossy().to_string());

    Ok(Json(ListResp {
        path: canon.to_string_lossy().to_string(),
        parent,
        entries,
    }))
}

pub async fn home_dir() -> Result<Json<HomeResp>, (StatusCode, String)> {
    let home = dirs_home()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "no HOME".into()))?;

    let mut quick: Vec<QuickEntry> = Vec::new();
    let candidates = [
        ("Home", home.clone()),
        ("Projects", home.join("projects")),
        ("Code", home.join("code")),
        ("Documents", home.join("Documents")),
        ("Desktop", home.join("Desktop")),
    ];
    for (label, p) in candidates {
        if p.is_dir() {
            quick.push(QuickEntry {
                label: label.into(),
                path: p.to_string_lossy().to_string(),
            });
        } else {
            debug!(path = %p.display(), "quick entry missing");
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        quick.push(QuickEntry {
            label: "Backend CWD".into(),
            path: cwd.to_string_lossy().to_string(),
        });
    }

    if quick.is_empty() {
        warn!("no quick entries found");
    }

    Ok(Json(HomeResp {
        home: home.to_string_lossy().to_string(),
        quick,
    }))
}

pub async fn read_file(
    Query(q): Query<ReadQuery>,
) -> Result<Json<ReadResp>, (StatusCode, String)> {
    let target = expand(&q.path);
    let canon = target
        .canonicalize()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("resolve: {e}")))?;

    let meta = std::fs::metadata(&canon)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("metadata: {e}")))?;
    if !meta.is_file() {
        return Err((StatusCode::BAD_REQUEST, "not a file".into()));
    }

    let size = meta.len();
    let truncated = size > MAX_READ_BYTES;
    let to_read = std::cmp::min(size, MAX_READ_BYTES) as usize;

    use std::io::Read;
    let mut f = std::fs::File::open(&canon)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("open: {e}")))?;
    let mut buf = vec![0u8; to_read];
    f.read_exact(&mut buf)
        .or_else(|_| f.read(&mut buf).map(|_| ()))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("read: {e}")))?;

    let content = String::from_utf8_lossy(&buf).to_string();

    info!(
        path = %canon.display(),
        bytes = to_read,
        %truncated,
        "read file"
    );

    Ok(Json(ReadResp {
        path: canon.to_string_lossy().to_string(),
        size,
        content,
        truncated,
    }))
}

pub async fn write_file(
    Json(body): Json<WriteBody>,
) -> Result<Json<WriteResp>, (StatusCode, String)> {
    let target = expand(&body.path);
    let canon = target
        .canonicalize()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("resolve: {e}")))?;

    let meta = std::fs::metadata(&canon)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("metadata: {e}")))?;
    if !meta.is_file() {
        return Err((StatusCode::BAD_REQUEST, "not a file".into()));
    }

    let ext = canon
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    match ext.as_deref() {
        Some("md") | Some("markdown") | Some("mdx") => {}
        _ => return Err((StatusCode::BAD_REQUEST, "only markdown files are editable".into())),
    }

    let bytes = body.content.len() as u64;
    if bytes > MAX_READ_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("content too large ({bytes} > {MAX_READ_BYTES})"),
        ));
    }

    // Atomic write: tmp file in same dir + rename.
    let parent = canon
        .parent()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "no parent dir".into()))?;
    let file_name = canon
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("note.md");
    let tmp = parent.join(format!(".{file_name}.tmp.{}", uuid_short()));

    std::fs::write(&tmp, body.content.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("write tmp: {e}")))?;
    if let Err(e) = std::fs::rename(&tmp, &canon) {
        let _ = std::fs::remove_file(&tmp);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("rename: {e}")));
    }

    info!(path = %canon.display(), bytes, "wrote markdown");

    Ok(Json(WriteResp {
        path: canon.to_string_lossy().to_string(),
        size: bytes,
    }))
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}

pub async fn raw_file(Query(q): Query<ReadQuery>) -> Result<Response, (StatusCode, String)> {
    let target = expand(&q.path);
    let canon = target
        .canonicalize()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("resolve: {e}")))?;

    let meta = std::fs::metadata(&canon)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("metadata: {e}")))?;
    if !meta.is_file() {
        return Err((StatusCode::BAD_REQUEST, "not a file".into()));
    }

    let size = meta.len();
    if size > MAX_RAW_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("file too large ({size} > {MAX_RAW_BYTES})"),
        ));
    }

    let ext = canon
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    let mime = match ext.as_deref() {
        Some("stl") => "model/stl",
        Some("obj") => "model/obj",
        Some("ply") => "application/octet-stream",
        Some("glb") => "model/gltf-binary",
        Some("gltf") => "model/gltf+json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    };

    let file = tokio::fs::File::open(&canon)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("open: {e}")))?;
    let stream = ReaderStream::new(file);

    info!(path = %canon.display(), bytes = size, mime, "raw file served");

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, size)
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(stream))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("response: {e}")))?
        .into_response())
}

#[allow(dead_code)]
fn _relative(base: &Path, p: &Path) -> String {
    p.strip_prefix(base)
        .map(|r| r.to_string_lossy().into_owned())
        .unwrap_or_else(|_| p.to_string_lossy().into_owned())
}
