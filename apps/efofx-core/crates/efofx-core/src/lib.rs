//! efofx platform core — library entry point.
//!
//! `src/main.rs` is a thin wrapper that loads config and calls
//! [`run`]; everything meaningful lives here so integration tests can
//! build an in-process router via [`build_router`].

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::{
    extract::State,
    http::{HeaderName, Request},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{info, Span};
use utoipa::OpenApi;

use efofx_auth::{middleware::AuthState, JwksCache};
use efofx_config::AppConfig;
use efofx_email::{EmailSender, NoopSender, ResendSender};
use efofx_llm::{LlmProvider, OpenAiProvider};
use efofx_prompts::PromptRegistry;
use efofx_storage::auth::{ApiKeyAuth, MasterKey, TenantResolver};
use efofx_storage::{
    ChatRepo, EstimationRepo, HealthStatus, MongoAdapter, ReferenceRepo, TenantRepo, WidgetLeadRepo,
};

pub mod api;
pub mod middleware;
pub mod openapi;
pub mod services;

use middleware::rate_limit::IpRateLimiter;
use openapi::ApiDoc;
use services::{ByokService, ChatService, EstimationService};

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Rate limits for the widget public surface. Per-IP per-minute quotas
/// enforced by [`middleware::rate_limit`]. Tweakable via config later;
/// the current values mirror FastAPI (`@limiter.limit("30/minute")` on
/// branding, `10/minute` on analytics read).
const BRANDING_RPM: u32 = 30;
const ANALYTICS_READ_RPM: u32 = 10;

/// Composed application state. Every handler reads the slice of this
/// struct it needs.
#[derive(Clone)]
pub struct AppState {
    pub mongo: MongoAdapter,
    pub tenants: TenantRepo,
    pub widget_leads: WidgetLeadRepo,
    pub byok: ByokService,
    pub chat: ChatService,
    pub estimation: EstimationService,
    pub api_key_auth: ApiKeyAuth,
    pub auth: AuthState,
    pub branding_rate_limiter: IpRateLimiter,
    pub analytics_rate_limiter: IpRateLimiter,
    /// Outbound mailer. `Arc<dyn EmailSender>` so the consultation handler
    /// can fire-and-forget through whichever backend `email.resend_api_key`
    /// selected at boot — Resend in prod, [`NoopSender`] when unset.
    pub email: Arc<dyn EmailSender>,
    /// `From:` address used on every outbound email. Cached on `AppState`
    /// so handlers don't need to plumb [`AppConfig`] through.
    pub email_from: Arc<str>,
}

/// Build the full application state by wiring every service, repo, and
/// middleware dependency. Called from `main` at startup and from
/// integration tests with a custom [`AppConfig`].
pub async fn build_app_state(cfg: &AppConfig) -> anyhow::Result<AppState> {
    let mongo = MongoAdapter::connect(&cfg.mongo).await?;

    let master_raw = cfg
        .crypto
        .master_key
        .as_deref()
        .context("crypto.master_key is required when BYOK endpoints are live")?;
    let master_key = MasterKey::from_config_str(master_raw)
        .context("crypto.master_key must be at least 32 bytes")?;
    let master_key_arc = Arc::new(master_key.clone());

    let api_key_auth = ApiKeyAuth::new(master_key);
    let tenants = TenantRepo::new(mongo.clone());
    let resolver = TenantResolver::new(mongo.clone());

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("build http client")?;

    let byok = ByokService::new(tenants.clone(), master_key_arc, http.clone());

    let email: Arc<dyn EmailSender> = match cfg
        .email
        .resend_api_key
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty())
    {
        Some(key) => {
            tracing::info!("email sender: resend");
            Arc::new(ResendSender::new(http.clone(), key.to_string()))
        }
        None => {
            tracing::warn!(
                "email.resend_api_key absent — using NoopSender; consultation \
                 emails will be skipped (lead is still persisted)"
            );
            Arc::new(NoopSender)
        }
    };
    let email_from: Arc<str> = Arc::from(cfg.email.from_address.clone().into_boxed_str());

    let chat_repo = ChatRepo::new(mongo.clone());
    chat_repo
        .ensure_indexes()
        .await
        .context("ensure chat_sessions indexes")?;

    let estimates_repo = EstimationRepo::new(mongo.clone());
    estimates_repo
        .ensure_indexes()
        .await
        .context("ensure estimates indexes")?;

    let reference_repo = ReferenceRepo::new(mongo.clone());
    reference_repo
        .ensure_indexes()
        .await
        .context("ensure reference_classes/projects indexes")?;

    let widget_leads = WidgetLeadRepo::new(mongo.clone());
    widget_leads
        .ensure_indexes()
        .await
        .context("ensure widget_leads indexes")?;

    let prompts_dir =
        std::env::var("EFOFX_PROMPTS_DIR").unwrap_or_else(|_| "config/prompts".into());
    let prompts = Arc::new(
        PromptRegistry::load_from_dir(&prompts_dir)
            .with_context(|| format!("load prompts from {prompts_dir}"))?,
    );

    let llm: Arc<dyn LlmProvider> = Arc::new(
        OpenAiProvider::new()
            .with_request_timeout(Duration::from_millis(cfg.llm.request_timeout_ms)),
    );

    let chat = ChatService::new(
        chat_repo.clone(),
        Arc::clone(&prompts),
        Arc::clone(&llm),
        cfg.llm.clone(),
    );
    let estimation = EstimationService::new(
        chat_repo,
        estimates_repo,
        reference_repo,
        prompts,
        llm,
        cfg.llm.clone(),
    );

    let jwks = JwksCache::bootstrap(
        &cfg.supabase.url,
        Duration::from_secs(cfg.supabase.jwks_refresh_seconds),
    )
    .await
    .context("bootstrap Supabase JWKS cache")?;

    let auth = AuthState {
        jwks,
        supabase: Arc::new(cfg.supabase.clone()),
        resolver,
        api_key: api_key_auth.clone(),
    };

    let branding_rate_limiter = IpRateLimiter::per_minute(BRANDING_RPM);
    let analytics_rate_limiter = IpRateLimiter::per_minute(ANALYTICS_READ_RPM);

    Ok(AppState {
        mongo,
        tenants,
        widget_leads,
        byok,
        chat,
        estimation,
        api_key_auth,
        auth,
        branding_rate_limiter,
        analytics_rate_limiter,
        email,
        email_from,
    })
}

/// Wire the HTTP router with trace, request-id, and CORS layers applied.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);

    let trace = TraceLayer::new_for_http()
        .make_span_with(|req: &Request<_>| {
            let request_id = req
                .headers()
                .get(&REQUEST_ID_HEADER)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("-")
                .to_string();
            tracing::info_span!(
                "http",
                method = %req.method(),
                uri = %req.uri(),
                request_id = %request_id,
            )
        })
        .on_response(
            |resp: &axum::response::Response, latency: Duration, _span: &Span| {
                tracing::info!(status = resp.status().as_u16(), ?latency, "response");
            },
        );

    let system = Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi_json));

    system
        .merge(api::router(Arc::clone(&state)))
        .with_state(state)
        .layer(SetRequestIdLayer::new(
            REQUEST_ID_HEADER.clone(),
            MakeRequestUuid,
        ))
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER.clone()))
        .layer(trace)
        .layer(cors)
}

/// Start the server. Blocks until shutdown.
pub async fn run(cfg: AppConfig) -> anyhow::Result<()> {
    info!(
        host = %cfg.server.host,
        port = cfg.server.port,
        db = %cfg.mongo.db_name,
        supabase_issuer = %cfg.supabase.issuer,
        "starting efofx-core"
    );

    let state = Arc::new(build_app_state(&cfg).await?);
    state.auth.jwks.spawn_refresh();

    let app = build_router(Arc::clone(&state));

    let addr: std::net::SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "listening");
    // `into_make_service_with_connect_info` is required so the
    // rate-limit middleware can read the peer socket as a fallback IP
    // key when `x-forwarded-for` is absent.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses(
        (status = 200, description = "Service healthy", body = HealthStatus),
    ),
)]
pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(HealthStatus::probe(&state.mongo).await)
}

#[utoipa::path(
    get,
    path = "/openapi.json",
    tag = "system",
    responses(
        (status = 200, description = "OpenAPI 3 document for this service"),
    ),
)]
pub async fn openapi_json() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}
