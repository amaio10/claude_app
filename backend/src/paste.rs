use std::path::PathBuf;

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use tracing::info;

use crate::state::AppState;

#[derive(Serialize)]
pub struct PasteResp {
    pub path: String,
    pub abs_path: String,
    pub bytes: usize,
    pub mime: String,
}

pub async fn paste_image(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<PasteResp>, (StatusCode, String)> {
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut mime = String::from("image/png");
    let mut cwd_override: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart: {e}")))?
    {
        match field.name() {
            Some("image") => {
                if let Some(ct) = field.content_type() {
                    mime = ct.to_string();
                }
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, format!("read image: {e}")))?;
                image_bytes = Some(bytes.to_vec());
            }
            Some("cwd") => {
                let t = field
                    .text()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, format!("read cwd: {e}")))?;
                if !t.is_empty() {
                    cwd_override = Some(t);
                }
            }
            _ => {}
        }
    }

    let bytes = image_bytes
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing image field".to_string()))?;

    let cwd = cwd_override.unwrap_or_else(|| state.default_cwd.clone());
    let ext = ext_from_mime(&mime);
    let id = uuid::Uuid::new_v4().to_string();
    let rel = format!("pastes/{id}.{ext}");
    let abs = PathBuf::from(&cwd).join(&rel);

    if let Some(parent) = abs.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("mkdir: {e}")))?;
    }
    tokio::fs::write(&abs, &bytes)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("write: {e}")))?;

    info!(
        bytes = bytes.len(),
        mime = %mime,
        path = %abs.display(),
        "saved pasted image"
    );

    Ok(Json(PasteResp {
        path: rel,
        abs_path: abs.display().to_string(),
        bytes: bytes.len(),
        mime,
    }))
}

fn ext_from_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "png",
    }
}
