//! Dashboard-facing lead management (Phase 2D).
//!
//! New in Rust — FastAPI had no dashboard lead-read path. Tenants manage
//! their captured leads from the dashboard via these routes.
//!
//! **Status: Phase 4.** Every handler below returns `501 Not Implemented`.
//! The routes and OpenAPI schemas are registered now so the external
//! surface is pinned; storage reads and auth land with the dashboard
//! leads page.

use std::sync::Arc;

use axum::{response::IntoResponse, routing::get, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_domain::{Lead, LeadStatus};
use efofx_openapi::ApiError;

use crate::AppState;

use super::not_implemented;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LeadListResponse {
    pub leads: Vec<Lead>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateLeadRequest {
    pub status: LeadStatus,
}

/// List leads for the authenticated tenant. Supports cursor-style offset
/// pagination.
#[utoipa::path(
    get,
    path = "/v1/leads",
    tag = "leads",
    params(
        ("limit" = Option<u32>, Query, description = "Page size (default 20, max 100)"),
        ("offset" = Option<u32>, Query, description = "Row offset (default 0)"),
        ("status" = Option<LeadStatus>, Query, description = "Filter by lead status")
    ),
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Lead page", body = LeadListResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn list_leads() -> impl IntoResponse {
    not_implemented("GET /v1/leads")
}

/// Fetch one lead with full conversation and consultation context.
#[utoipa::path(
    get,
    path = "/v1/leads/{lead_id}",
    tag = "leads",
    params(("lead_id" = String, Path)),
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Lead", body = Lead),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Lead not found", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn get_lead() -> impl IntoResponse {
    not_implemented("GET /v1/leads/{id}")
}

/// Update a lead's status (`new` → `contacted` → `converted` / `closed`).
#[utoipa::path(
    patch,
    path = "/v1/leads/{lead_id}",
    tag = "leads",
    params(("lead_id" = String, Path)),
    request_body = UpdateLeadRequest,
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Updated lead", body = Lead),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Lead not found", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn patch_lead() -> impl IntoResponse {
    not_implemented("PATCH /v1/leads/{id}")
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/leads", get(list_leads))
        .route("/v1/leads/:lead_id", get(get_lead).patch(patch_lead))
}
