use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

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

use efofx_config::AppConfig;
use efofx_storage::{HealthStatus, MongoAdapter};
use utoipa::OpenApi;

// Phase 1.2 stub handlers don't deserialize request bodies (they return 501
// before touching them). Lift this attribute once Phase 2A wires real
// handlers that consume the DTO fields.
#[allow(dead_code)]
mod api;
mod openapi;

use openapi::ApiDoc;

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone)]
pub struct AppState {
    mongo: MongoAdapter,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let cfg = AppConfig::load()?;
    info!(
        host = %cfg.server.host,
        port = cfg.server.port,
        db = %cfg.mongo.db_name,
        supabase_issuer = %cfg.supabase.issuer,
        "starting efofx-core"
    );

    let mongo = MongoAdapter::connect(&cfg.mongo).await?;

    let state = Arc::new(AppState { mongo });

    let app = build_router(state);

    let addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,efofx_core=debug"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json().with_target(true))
        .init();
}

fn build_router(state: Arc<AppState>) -> Router {
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
        .merge(api::router())
        .with_state(state)
        .layer(SetRequestIdLayer::new(
            REQUEST_ID_HEADER.clone(),
            MakeRequestUuid,
        ))
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER.clone()))
        .layer(trace)
        .layer(cors)
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses(
        (status = 200, description = "Service healthy", body = HealthStatus),
    ),
)]
async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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
async fn openapi_json() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}
