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
use efofx_storage::auth::{ApiKeyAuth, MasterKey, TenantResolver};
use efofx_storage::{HealthStatus, MongoAdapter, TenantRepo};

pub mod api;
pub mod openapi;
pub mod services;

use openapi::ApiDoc;
use services::ByokService;

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Composed application state. Every handler reads the slice of this
/// struct it needs.
#[derive(Clone)]
pub struct AppState {
    pub mongo: MongoAdapter,
    pub tenants: TenantRepo,
    pub byok: ByokService,
    pub api_key_auth: ApiKeyAuth,
    pub auth: AuthState,
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

    let byok = ByokService::new(tenants.clone(), master_key_arc, http);

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

    Ok(AppState {
        mongo,
        tenants,
        byok,
        api_key_auth,
        auth,
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
    axum::serve(listener, app).await?;
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
