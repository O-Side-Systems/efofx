//! Partner / contractor-directory integrations (Phase 2F).

use std::sync::Arc;

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_auth::middleware::widget_api_key;
use efofx_domain::{EstimationOutput, EstimationSessionId, ScopingContext};
use efofx_openapi::{ApiError, ErrorCode};
use efofx_storage::TenantContext;

use crate::services::{routing, EstimationServiceError};
use crate::AppState;

/// Inbound override fields (`project_type`, `location`) cap. Mirrors the
/// chat-message validation pattern: small, predictable bound.
const OVERRIDE_MAX_CHARS: usize = 64;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ContractorMatchRequest {
    pub estimation_session_id: String,
    /// Optional override of the inferred project type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    /// Optional location hint (city, ZIP, or region label).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContractorMatchResponse {
    /// Structured tags the partner directory can filter on.
    pub tags: Vec<String>,
    /// Region label, if resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Estimated cost tier for the target project, e.g. `"mid"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_tier: Option<String>,
    /// URL with partner filters applied, ready for redirect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory_url: Option<String>,
}

/// Produce structured routing data from a completed estimation session.
/// Partners call this to filter their directory by the session's project.
#[utoipa::path(
    post,
    path = "/v1/integration/contractor-match",
    tag = "integration",
    request_body = ContractorMatchRequest,
    security(("widget_api_key" = [])),
    responses(
        (status = 200, description = "Routing data", body = ContractorMatchResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "API key invalid", body = ApiError),
        (status = 404, description = "Session not found", body = ApiError),
    ),
)]
pub async fn contractor_match(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<ContractorMatchRequest>,
) -> Response {
    if let Err(resp) = validate_request(&body) {
        return resp;
    }

    // Tenant-scoped session lookup. NotFound surfaces as the 404 envelope
    // partners expect; everything else maps to its own error envelope.
    let session_id = EstimationSessionId(body.estimation_session_id.clone());
    let session = match state.estimation.get_session(&ctx, &session_id).await {
        Ok(s) => s,
        Err(err) => {
            if !matches!(err, EstimationServiceError::EstimationSessionNotFound) {
                tracing::error!(error = %err, "contractor-match: session lookup failed");
            }
            return err.into_response();
        }
    };

    // Routing config drives both tag-emission gating and URL templating.
    // A read failure should never blow up the partner call — fall through
    // to "routing disabled" semantics (empty tags, no directory URL).
    let routing_cfg = match state.tenants.fetch_routing_config(&ctx).await {
        Ok(cfg) => cfg,
        Err(err) => {
            tracing::warn!(
                tenant_id = %ctx.tenant_id(),
                error = %err,
                "fetch_routing_config failed; treating as disabled",
            );
            None
        }
    };

    // Scoping comes from the request overrides only. The plan permits
    // chat-session lookup to fail and fall through with empty scoping —
    // we have no FK from estimation → chat session in storage today, so
    // overrides are the only signal. Region still drives the `region:*`
    // tag from the loaded EstimationSession.
    let scoping = scoping_from_overrides(&body);

    // Cost tier: EstimationOutput is not persisted alongside the session
    // in the current schema (see api/estimation.rs:71). Until that gap
    // closes (Phase 4), bucket the tier as `Low` deterministically by
    // passing a zero-cost synthetic output. The wire shape stays stable
    // and partners can still filter on `region:*` / `type:*` /
    // `location:*`.
    let synthetic = synthetic_zero_output();
    let data = routing::derive(&session, &scoping, &synthetic, routing_cfg.as_ref());
    let directory_url = routing::render_directory_url(&data, routing_cfg.as_ref());

    let response = ContractorMatchResponse {
        tags: data.tags,
        region: data.region,
        estimated_cost_tier: Some(data.cost_tier.as_str().to_string()),
        directory_url,
    };
    (StatusCode::OK, Json(response)).into_response()
}

#[allow(clippy::result_large_err)]
fn validate_request(body: &ContractorMatchRequest) -> Result<(), Response> {
    if body.estimation_session_id.trim().is_empty() {
        return Err(validation_error("estimation_session_id must not be empty"));
    }
    if let Some(pt) = body.project_type.as_deref() {
        if pt.chars().count() > OVERRIDE_MAX_CHARS {
            return Err(validation_error("project_type must be ≤ 64 chars"));
        }
    }
    if let Some(loc) = body.location.as_deref() {
        if loc.chars().count() > OVERRIDE_MAX_CHARS {
            return Err(validation_error("location must be ≤ 64 chars"));
        }
    }
    Ok(())
}

fn validation_error(msg: &'static str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError::single(ErrorCode::ValidationFailed.as_str(), msg)),
    )
        .into_response()
}

fn scoping_from_overrides(body: &ContractorMatchRequest) -> ScopingContext {
    let mut scoping = ScopingContext::default();
    if let Some(pt) = body
        .project_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        scoping.project_type = Some(pt.to_string());
    }
    if let Some(loc) = body
        .location
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        scoping.location = Some(loc.to_string());
    }
    scoping
}

/// Zero-cost stand-in for the missing persisted `EstimationOutput`.
/// Buckets as `CostTier::Low`. See the call site for why.
fn synthetic_zero_output() -> EstimationOutput {
    EstimationOutput {
        total_cost_p50: 0.0,
        total_cost_p80: 0.0,
        timeline_weeks_p50: 0,
        timeline_weeks_p80: 0,
        cost_breakdown: Vec::new(),
        adjustment_factors: Vec::new(),
        confidence_score: 0.0,
        assumptions: Vec::new(),
        summary: String::new(),
    }
}

/// Assemble the integration router. Wraps the contractor-match endpoint
/// in the widget-api-key middleware — partners call this server-to-server
/// with the tenant's widget API key, the same credential they'd use for
/// `/v1/widget/leads`.
pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/integration/contractor-match", post(contractor_match))
        .route_layer(from_fn_with_state(state.auth.clone(), widget_api_key))
}
