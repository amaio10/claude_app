use std::sync::Arc;

use anyhow::Context;
use dashmap::DashMap;

use crate::pty::PtyHandle;

#[derive(Clone)]
pub struct AppState {
    pub groq_api_key: String,
    pub default_cwd: String,
    pub sessions: Arc<DashMap<String, String>>,
    pub pty_sessions: Arc<DashMap<String, Arc<PtyHandle>>>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let groq_api_key = std::env::var("GROQ_API_KEY").unwrap_or_default();
        let default_cwd = std::env::var("CLAUDE_APP_DEFAULT_CWD")
            .or_else(|_| std::env::current_dir().map(|p| p.display().to_string()))
            .context("resolve default cwd")?;
        Ok(Self {
            groq_api_key,
            default_cwd,
            sessions: Arc::new(DashMap::new()),
            pty_sessions: Arc::new(DashMap::new()),
        })
    }
}
