//! SecretProvider chain — env | file | vault.
//!
//! Resolves secrets (DATABASE_URL, JWT_SECRET, …) from a provider chain so
//! the binary runs identically in dev (env), container (/run/secrets), and
//! production (Vault scaffold).

use async_trait::async_trait;
use std::path::PathBuf;
use tracing::warn;
use zeroize::Zeroizing;

/// A secret value that zeroizes its buffer on drop.
#[derive(Debug, Clone)]
pub struct Secret(pub Zeroizing<String>);

impl Secret {
    /// Returns the inner secret string (borrowed).
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Secret provider trait.
#[async_trait]
pub trait SecretProvider: Send + Sync {
    /// Returns the secret value for `key`, or `Err` when missing.
    async fn get(&self, key: &str) -> Result<Secret, String>;
    /// Returns the provider name (logged once at boot).
    fn name(&self) -> &'static str;
}

/// Env-var provider.
pub struct EnvSecretProvider;
#[async_trait]
impl SecretProvider for EnvSecretProvider {
    async fn get(&self, key: &str) -> Result<Secret, String> {
        std::env::var(key)
            .map(|v| Secret(Zeroizing::new(v)))
            .map_err(|_| format!("env var {key} not set"))
    }
    fn name(&self) -> &'static str {
        "env"
    }
}

/// File-based provider (`/run/secrets/<key>`).
pub struct FileSecretProvider {
    dir: PathBuf,
}
impl FileSecretProvider {
    /// Constructs a provider rooted at `dir`.
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}
#[async_trait]
impl SecretProvider for FileSecretProvider {
    async fn get(&self, key: &str) -> Result<Secret, String> {
        let path = self.dir.join(key);
        tokio::fs::read_to_string(&path)
            .await
            .map(|v| Secret(Zeroizing::new(v.trim().to_string())))
            .map_err(|e| format!("file secret {key}: {e}"))
    }
    fn name(&self) -> &'static str {
        "file"
    }
}

/// Selector for the chain.
pub enum SecretProviderKind {
    Env,
    File,
}

impl SecretProviderKind {
    /// Picks a provider based on `SECRET_PROVIDER` env (env | file | vault).
    #[must_use]
    pub fn from_env() -> Self {
        match std::env::var("SECRET_PROVIDER").unwrap_or_default().as_str() {
            "file" => Self::File,
            _ => Self::Env,
        }
    }
    /// Builds the provider.
    #[must_use]
    pub fn provider(&self) -> Box<dyn SecretProvider> {
        match self {
            Self::Env => Box::new(EnvSecretProvider),
            Self::File => Box::new(FileSecretProvider::new(PathBuf::from("/run/secrets"))),
        }
    }
}
