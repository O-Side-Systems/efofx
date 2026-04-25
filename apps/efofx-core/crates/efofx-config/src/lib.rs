//! Layered configuration loader.
//!
//! Precedence (highest first):
//!   1. Environment variables prefixed `EFOFX_` (e.g. `EFOFX_SERVER__PORT`)
//!   2. `config/efofx-core.toml` (or path given via `EFOFX_CONFIG_PATH`)
//!   3. Compile-time defaults on each struct
//!
//! Call [`AppConfig::load`] exactly once at startup. The returned value is
//! immutable and should be passed into Axum state.

use std::path::PathBuf;

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("configuration load failed: {0}")]
    Figment(#[from] Box<figment::Error>),
    #[error("configuration validation failed: {0}")]
    Validation(String),
}

impl From<figment::Error> for ConfigError {
    fn from(err: figment::Error) -> Self {
        Self::Figment(Box::new(err))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    pub mongo: MongoConfig,
    pub supabase: SupabaseConfig,
    #[serde(default)]
    pub crypto: CryptoConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub email: EmailConfig,
    #[serde(default)]
    pub features: FeatureFlags,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    8080
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MongoConfig {
    /// Mongo connection URI. Treat as secret. Not logged.
    pub uri: String,
    #[serde(default = "default_db_name")]
    pub db_name: String,
}

fn default_db_name() -> String {
    "efofx".into()
}

/// Supabase is used auth-only. The server verifies JWTs via JWKS; no
/// service_role key is required on the server side.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupabaseConfig {
    /// Project base URL, e.g. `https://qzdgqepcdaoyigrgtlzv.supabase.co`.
    pub url: String,
    /// JWT `iss` claim value we require. Usually `{url}/auth/v1`.
    pub issuer: String,
    /// JWT `aud` claim value we require. Supabase default is `authenticated`.
    #[serde(default = "default_audience")]
    pub audience: String,
    /// How often to refresh cached JWKS. Default 15 min.
    #[serde(default = "default_jwks_refresh")]
    pub jwks_refresh_seconds: u64,
}

fn default_audience() -> String {
    "authenticated".into()
}
fn default_jwks_refresh() -> u64 {
    900
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CryptoConfig {
    /// Root key for BYOK encryption (HKDF input). Required at runtime; marked
    /// Option so the config can load in tooling contexts that don't need it.
    #[serde(default)]
    pub master_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmConfig {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
    #[serde(default = "default_true")]
    pub streaming_enabled: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub budgets: LlmBudgetConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            model: default_llm_model(),
            request_timeout_ms: default_request_timeout_ms(),
            streaming_enabled: true,
            temperature: None,
            max_tokens: None,
            budgets: LlmBudgetConfig::default(),
        }
    }
}

/// Per-session and per-tenant token budgets. `0` disables a limit.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmBudgetConfig {
    /// Max messages allowed on one chat session. FastAPI uses 50; parity.
    #[serde(default = "default_per_session_messages")]
    pub per_session_messages: u32,
    /// Max cumulative prompt+completion tokens per session. `0` = unlimited.
    #[serde(default = "default_per_session_tokens")]
    pub per_session_tokens: u64,
    /// Max cumulative tokens per tenant across all sessions. `0` = unlimited.
    /// Enforcement wiring lands with 2B.6; the config slot exists now so
    /// deployments can prepare values ahead of it.
    #[serde(default)]
    pub per_tenant_tokens: u64,
}

impl Default for LlmBudgetConfig {
    fn default() -> Self {
        Self {
            per_session_messages: default_per_session_messages(),
            per_session_tokens: default_per_session_tokens(),
            per_tenant_tokens: 0,
        }
    }
}

fn default_llm_provider() -> String {
    "openai".into()
}
fn default_llm_model() -> String {
    "gpt-4o-mini".into()
}
fn default_request_timeout_ms() -> u64 {
    30_000
}
fn default_true() -> bool {
    true
}
fn default_per_session_messages() -> u32 {
    50
}
fn default_per_session_tokens() -> u64 {
    100_000
}

/// Outbound email settings. The whole struct is optional — a missing
/// `resend_api_key` means the runtime wires a no-op sender so consultation
/// inserts succeed and the email simply gets logged. Mirrors FastAPI's
/// "skip if MAIL_* unset" behaviour.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailConfig {
    /// Resend API key. Treat as secret. When empty/absent, the no-op
    /// sender is used.
    #[serde(default)]
    pub resend_api_key: Option<String>,
    /// `From:` address on outbound mail. Must be a verified sender on the
    /// Resend account.
    #[serde(default = "default_from_address")]
    pub from_address: String,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            resend_api_key: None,
            from_address: default_from_address(),
        }
    }
}

fn default_from_address() -> String {
    "noreply@efofx.dev".into()
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureFlags {
    #[serde(default)]
    pub contractor_routing: bool,
    #[serde(default)]
    pub platform_key_fallback: bool,
}

impl AppConfig {
    /// Load config from the layered sources. Validate; fail fast.
    pub fn load() -> Result<Self, ConfigError> {
        let path = std::env::var("EFOFX_CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("config/efofx-core.toml"));

        let mut fig = Figment::new();
        if path.exists() {
            fig = fig.merge(Toml::file(&path));
        }
        fig = fig.merge(Env::prefixed("EFOFX_").split("__"));

        let cfg: AppConfig = fig.extract()?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.mongo.uri.trim().is_empty() {
            return Err(ConfigError::Validation("mongo.uri is required".into()));
        }
        if self.supabase.url.trim().is_empty() {
            return Err(ConfigError::Validation("supabase.url is required".into()));
        }
        if self.supabase.issuer.trim().is_empty() {
            return Err(ConfigError::Validation(
                "supabase.issuer is required".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figment::Jail;

    #[test]
    fn loads_from_env_only() {
        Jail::expect_with(|jail| {
            jail.set_env("EFOFX_MONGO__URI", "mongodb://localhost:27017");
            jail.set_env("EFOFX_SUPABASE__URL", "https://x.supabase.co");
            jail.set_env("EFOFX_SUPABASE__ISSUER", "https://x.supabase.co/auth/v1");
            let cfg = AppConfig::load().map_err(|e| e.to_string())?;
            assert_eq!(cfg.mongo.uri, "mongodb://localhost:27017");
            assert_eq!(cfg.supabase.audience, "authenticated");
            assert_eq!(cfg.server.port, 8080);
            Ok(())
        });
    }

    #[test]
    fn rejects_missing_mongo_uri() {
        Jail::expect_with(|jail| {
            // Supply db_name so the `mongo` section exists; omit `uri` so our
            // explicit validation path (not figment deserialize) rejects it.
            jail.set_env("EFOFX_MONGO__DB_NAME", "efofx");
            jail.set_env("EFOFX_MONGO__URI", "");
            jail.set_env("EFOFX_SUPABASE__URL", "https://x.supabase.co");
            jail.set_env("EFOFX_SUPABASE__ISSUER", "https://x.supabase.co/auth/v1");
            let err = AppConfig::load().unwrap_err();
            let msg = err.to_string().to_lowercase();
            assert!(
                msg.contains("mongo") && (msg.contains("uri") || msg.contains("required")),
                "unexpected error: {msg}"
            );
            Ok(())
        });
    }
}
