//! Cursor pagination — base64-encoded opaque cursor.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

/// Cursor query parameters (`?cursor=<b64>&limit=N`).
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CursorPage {
    /// Opaque cursor from a prior response's `Link: <…>?cursor=…>; rel="next"`.
    pub cursor: Option<String>,
    /// Maximum number of items to return (default 20, max 200).
    pub limit: Option<u32>,
}

impl CursorPage {
    /// Decodes the cursor into the inner string.
    pub fn cursor(&self) -> Result<Option<Cursor>, String> {
        let Some(raw) = &self.cursor else {
            return Ok(None);
        };
        let bytes = URL_SAFE_NO_PAD
            .decode(raw)
            .map_err(|e| format!("invalid cursor: {e}"))?;
        let s = String::from_utf8(bytes).map_err(|e| format!("invalid cursor utf8: {e}"))?;
        Ok(Some(Cursor(s)))
    }
    /// Returns the limit, clamped to 1..=200.
    #[must_use]
    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(20).clamp(1, 200)
    }
}

/// Decoded cursor.
#[derive(Debug, Clone, Serialize)]
pub struct Cursor(pub String);

impl Cursor {
    /// Encodes the cursor back into the URL-safe base64 form.
    #[must_use]
    pub fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0.as_bytes())
    }
}
