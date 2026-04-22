use anyhow::{bail, Context};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

const GROQ_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const MODEL: &str = "whisper-large-v3-turbo";

#[derive(Serialize)]
pub struct TranscribeResp {
    pub text: String,
    pub model: String,
    pub ms: u128,
}

#[derive(Deserialize)]
struct GroqResp {
    text: String,
}

pub async fn transcribe_groq(
    api_key: &str,
    audio: Vec<u8>,
    filename: &str,
    mime: &str,
) -> anyhow::Result<TranscribeResp> {
    if api_key.is_empty() {
        bail!("GROQ_API_KEY missing — set it in .env");
    }

    let start = std::time::Instant::now();
    debug!(size = audio.len(), %filename, %mime, "calling Groq Whisper");

    let part = multipart::Part::bytes(audio)
        .file_name(filename.to_string())
        .mime_str(mime)
        .context("invalid mime")?;

    let form = multipart::Form::new()
        .part("file", part)
        .text("model", MODEL)
        .text("response_format", "json")
        .text("temperature", "0");

    let client = reqwest::Client::new();
    let resp = client
        .post(GROQ_URL)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .context("groq request failed")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        bail!("groq returned {status}: {body}");
    }

    let parsed: GroqResp =
        serde_json::from_str(&body).with_context(|| format!("parse groq json: {body}"))?;

    let ms = start.elapsed().as_millis();
    info!(chars = parsed.text.len(), %ms, "transcription done");

    Ok(TranscribeResp {
        text: parsed.text,
        model: MODEL.to_string(),
        ms,
    })
}
