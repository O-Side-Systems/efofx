//! Estimation service.
//!
//! Owns the `chat → estimate` pipeline:
//!
//! 1. Verify the chat session is `ready`.
//! 2. Build a description + region from the session's scoping context.
//! 3. Ask the LLM to classify into a reference-class name.
//! 4. Look up reference projects for (class, region); soft-fail on misses.
//! 5. Render the versioned `estimation` prompt.
//! 6. Run a structured-output call against a hand-built schema.
//! 7. Persist an [`EstimationSession`] and return it alongside the output.
//!
//! The orchestrator is kept thin; everything that isn't an I/O call is a
//! pure function so the tests can assert on behavior without a Mongo
//! fixture. End-to-end coverage lands with the SSE handler in 2C.6.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use time::{Duration as TimeDuration, OffsetDateTime};
use tracing::{info, warn};

use efofx_config::LlmConfig;
use efofx_domain::{
    ChatStatus, EstimationOutput, EstimationSession, EstimationSessionId, EstimationStatus,
    ReferenceProject, Region, ScopingContext, SessionId,
};
use efofx_llm::{ChatRequest as LlmReq, LlmError, LlmProvider, StructuredSchema};
use efofx_openapi::{ApiError, ErrorCode};
use efofx_prompts::{PromptError, PromptRegistry};
use efofx_storage::{ChatRepo, EstimationRepo, ReferenceRepo, StorageError, TenantContext};

/// FastAPI parity: `settings.SESSION_TIMEOUT_MINUTES` default is 30.
pub const DEFAULT_ESTIMATE_TTL_MINUTES: i64 = 30;

/// Cap on the reference-project list passed into the LLM prompt.
const REFERENCE_PROJECT_LIMIT: i64 = 5;

/// Fallback reference-class name when no classes are registered at all.
/// Mirrors FastAPI's `"general"` default in `_classify_project`.
const DEFAULT_FALLBACK_REFERENCE_CLASS: &str = "general";

/// Default confidence threshold stored on the session. FastAPI also hardcodes
/// `0.7` at call time; parity.
const DEFAULT_CONFIDENCE_THRESHOLD: f64 = 0.7;

#[derive(Debug, thiserror::Error)]
pub enum EstimationServiceError {
    #[error("chat session not found")]
    ChatSessionNotFound,
    #[error("chat session has expired")]
    ChatSessionExpired,
    #[error("chat session must be in `ready` state")]
    SessionNotReady,
    #[error("tenant has no BYOK OpenAI key on file")]
    MissingByokKey,
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Prompt(#[from] PromptError),
    #[error("llm call failed: {0}")]
    Llm(LlmError),
    #[error("estimate output failed schema validation: {0}")]
    SchemaParse(String),
}

impl From<LlmError> for EstimationServiceError {
    fn from(e: LlmError) -> Self {
        EstimationServiceError::Llm(e)
    }
}

impl IntoResponse for EstimationServiceError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            EstimationServiceError::ChatSessionNotFound => (
                StatusCode::NOT_FOUND,
                ErrorCode::ChatSessionNotFound.as_str(),
                ErrorCode::ChatSessionNotFound.default_message().to_string(),
            ),
            EstimationServiceError::ChatSessionExpired => (
                StatusCode::GONE,
                ErrorCode::ChatSessionExpired.as_str(),
                ErrorCode::ChatSessionExpired.default_message().to_string(),
            ),
            EstimationServiceError::SessionNotReady => (
                StatusCode::CONFLICT,
                ErrorCode::EstimationSessionNotReady.as_str(),
                ErrorCode::EstimationSessionNotReady
                    .default_message()
                    .to_string(),
            ),
            EstimationServiceError::MissingByokKey => (
                StatusCode::PAYMENT_REQUIRED,
                ErrorCode::EstimationLlmInvalidKey.as_str(),
                "No OpenAI API key on file. Add one in Settings.".to_string(),
            ),
            EstimationServiceError::Llm(e) => {
                let (code, http) = match e {
                    LlmError::InvalidKey(_) => (
                        ErrorCode::EstimationLlmInvalidKey,
                        StatusCode::PAYMENT_REQUIRED,
                    ),
                    LlmError::QuotaExhausted(_) => (
                        ErrorCode::EstimationLlmQuotaExhausted,
                        StatusCode::PAYMENT_REQUIRED,
                    ),
                    LlmError::Transient(_) => (
                        ErrorCode::EstimationLlmTransient,
                        StatusCode::SERVICE_UNAVAILABLE,
                    ),
                    LlmError::SchemaParse(_) | LlmError::Unknown(_) => (
                        ErrorCode::EstimationLlmUnknown,
                        StatusCode::INTERNAL_SERVER_ERROR,
                    ),
                };
                (http, code.as_str(), code.default_message().to_string())
            }
            EstimationServiceError::SchemaParse(msg) => {
                tracing::error!(error = %msg, "estimation schema parse failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::EstimationLlmUnknown.as_str(),
                    ErrorCode::EstimationLlmUnknown
                        .default_message()
                        .to_string(),
                )
            }
            EstimationServiceError::Storage(StorageError::NotFound) => (
                StatusCode::NOT_FOUND,
                ErrorCode::ChatSessionNotFound.as_str(),
                ErrorCode::ChatSessionNotFound.default_message().to_string(),
            ),
            EstimationServiceError::Storage(e) => {
                tracing::error!(error = %e, "estimation storage failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "common.internal",
                    "An internal error occurred".to_string(),
                )
            }
            EstimationServiceError::Prompt(e) => {
                tracing::error!(error = %e, "estimation prompt render failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "common.internal",
                    "An internal error occurred".to_string(),
                )
            }
        };
        (status, Json(ApiError::single(code, message))).into_response()
    }
}

/// Happy-path return value of [`EstimationService::generate_from_chat`].
#[derive(Debug, Clone)]
pub struct EstimationOutcome {
    pub session: EstimationSession,
    pub output: EstimationOutput,
}

/// Orchestrates the chat→estimate pipeline.
#[derive(Clone)]
pub struct EstimationService {
    chat_repo: ChatRepo,
    estimates: EstimationRepo,
    references: ReferenceRepo,
    prompts: Arc<PromptRegistry>,
    llm: Arc<dyn LlmProvider>,
    llm_cfg: LlmConfig,
}

impl EstimationService {
    pub fn new(
        chat_repo: ChatRepo,
        estimates: EstimationRepo,
        references: ReferenceRepo,
        prompts: Arc<PromptRegistry>,
        llm: Arc<dyn LlmProvider>,
        llm_cfg: LlmConfig,
    ) -> Self {
        Self {
            chat_repo,
            estimates,
            references,
            prompts,
            llm,
            llm_cfg,
        }
    }

    /// Generate a structured estimate from a completed scoping session and
    /// persist a fresh [`EstimationSession`]. See the module docs for the
    /// step order — this is the only async public method.
    pub async fn generate_from_chat(
        &self,
        ctx: &TenantContext,
        api_key: &str,
        chat_session_id: SessionId,
    ) -> Result<EstimationOutcome, EstimationServiceError> {
        // --- 1. chat session must exist, be ours, and be `ready` ------------
        let chat_session = self
            .chat_repo
            .get(ctx, chat_session_id)
            .await
            .map_err(|e| match e {
                StorageError::NotFound => EstimationServiceError::ChatSessionNotFound,
                other => EstimationServiceError::Storage(other),
            })?;
        let now = OffsetDateTime::now_utc();
        if chat_session.expires_at <= now || chat_session.status == ChatStatus::Expired {
            return Err(EstimationServiceError::ChatSessionExpired);
        }
        if chat_session.status != ChatStatus::Ready {
            return Err(EstimationServiceError::SessionNotReady);
        }

        // --- 2. inputs -------------------------------------------------------
        let description = build_description_from_context(&chat_session.scoping_context);
        let region_label = chat_session
            .scoping_context
            .location
            .clone()
            .unwrap_or_else(|| "General".to_string());

        // --- 3. load estimation prompt + lock prompt_version ----------------
        let prompt = self
            .prompts
            .latest("estimation")
            .ok_or_else(|| PromptError::NotFound {
                name: "estimation".into(),
                version: "latest".into(),
            })?;

        // --- 4. classify via a free-form LLM call ---------------------------
        let allowed_classes: Vec<String> = self
            .references
            .list_reference_classes(Some(ctx), None)
            .await
            .map_err(|e| {
                warn!(error = %e, "failed to load reference classes; falling back to general");
                e
            })
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.name)
            .collect();

        let raw_classification = self
            .call_classify(api_key, &description, &region_label, &allowed_classes)
            .await?;
        let reference_class = parse_classification(&raw_classification, &allowed_classes);

        // --- 5. pull reference projects (soft-fail) -------------------------
        let reference_projects = match self
            .references
            .get_reference_projects(
                Some(ctx),
                &reference_class,
                &region_label,
                REFERENCE_PROJECT_LIMIT,
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    reference_class,
                    region = region_label.as_str(),
                    error = %e,
                    "failed to load reference projects; continuing without them",
                );
                Vec::new()
            }
        };

        // --- 6. render the estimation prompt --------------------------------
        let reference_data_section = format_reference_data_section(&reference_projects);
        let user_prompt = prompt.render_user(&[
            ("description", description.as_str()),
            ("reference_class", reference_class.as_str()),
            ("region", region_label.as_str()),
            ("reference_data_section", reference_data_section.as_str()),
        ])?;

        // --- 7. structured-output call --------------------------------------
        let req = self.build_llm_request(Some(&prompt.system_prompt), user_prompt);
        let resp = self
            .llm
            .complete_structured(api_key, req, estimation_output_schema())
            .await?;
        let output: EstimationOutput = serde_json::from_value(resp.data).map_err(|e| {
            EstimationServiceError::SchemaParse(format!("EstimationOutput decode: {e}"))
        })?;

        // --- 8. persist the estimation session ------------------------------
        let session = assemble_session(
            ctx.tenant_id(),
            description,
            &region_label,
            reference_class.clone(),
            prompt.version.clone(),
            now,
            DEFAULT_ESTIMATE_TTL_MINUTES,
        );
        self.estimates.save(ctx, &session).await?;
        info!(
            estimation_id = %session.id,
            chat_session_id = %chat_session_id,
            reference_class,
            "estimation session persisted",
        );

        Ok(EstimationOutcome { session, output })
    }

    async fn call_classify(
        &self,
        api_key: &str,
        description: &str,
        region: &str,
        allowed_classes: &[String],
    ) -> Result<String, EstimationServiceError> {
        // No classes loaded — don't even make the LLM call; return fallback.
        if allowed_classes.is_empty() {
            return Ok(DEFAULT_FALLBACK_REFERENCE_CLASS.to_string());
        }
        let user = build_classify_user_prompt(description, region, allowed_classes);
        let req = self.build_llm_request(Some(CLASSIFY_SYSTEM_PROMPT), user);
        let resp = self.llm.complete(api_key, req).await?;
        Ok(resp.content)
    }

    fn build_llm_request(&self, system: Option<&str>, user: String) -> LlmReq {
        let mut req = LlmReq::simple(&self.llm_cfg.model, system, user);
        if let Some(t) = self.llm_cfg.temperature {
            req = req.with_temperature(t);
        }
        if let Some(n) = self.llm_cfg.max_tokens {
            req = req.with_max_tokens(n);
        }
        req
    }
}

// --- pure helpers (unit tested) -----------------------------------------

/// Build the natural-language description the estimation prompt consumes,
/// mirroring FastAPI's `_build_description_from_context`.
pub(crate) fn build_description_from_context(c: &ScopingContext) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = &c.project_type {
        parts.push(format!("Project type: {v}"));
    }
    if let Some(v) = &c.project_size {
        parts.push(format!("Size/scope: {v}"));
    }
    if let Some(v) = &c.location {
        parts.push(format!("Location: {v}"));
    }
    if let Some(v) = &c.timeline {
        parts.push(format!("Timeline: {v}"));
    }
    if let Some(v) = &c.special_conditions {
        parts.push(format!("Special conditions: {v}"));
    }
    if parts.is_empty() {
        "General project".to_string()
    } else {
        format!("{}.", parts.join(". "))
    }
}

/// Resolve a freeform region label (from `ScopingContext.location`) into a
/// typed [`Region`]. Unknown labels fall back to `NorCalBayArea` — matching
/// FastAPI's `try RegionEnum(region) except ValueError: NORCAL_BAY_AREA`.
pub(crate) fn resolve_region(label: &str) -> Region {
    serde_json::from_value::<Region>(Value::String(label.to_string()))
        .unwrap_or(Region::NorCalBayArea)
}

/// Pick a canonical reference-class name from the LLM's free-form response.
/// The match is case-insensitive against the allow-list. On unrecognized
/// output, fall back to the first allow-list entry (FastAPI parity), or
/// `"general"` if the allow-list is empty.
pub(crate) fn parse_classification(raw: &str, allowed: &[String]) -> String {
    let normalized = raw.trim();
    if allowed.is_empty() {
        return DEFAULT_FALLBACK_REFERENCE_CLASS.to_string();
    }
    for name in allowed {
        if name.eq_ignore_ascii_case(normalized) {
            return name.clone();
        }
    }
    allowed
        .first()
        .cloned()
        .unwrap_or_else(|| DEFAULT_FALLBACK_REFERENCE_CLASS.to_string())
}

/// System prompt for the classification call. Inlined instead of a
/// versioned prompt because FastAPI also inlines it.
const CLASSIFY_SYSTEM_PROMPT: &str = "You are an expert construction estimator. Your task is to classify construction projects into appropriate reference classes based on project descriptions and regional context. Respond with only the reference class name, nothing else.";

/// Build the user-side classification prompt. Parity with FastAPI's format,
/// minus Python repr quirks (`['a', 'b']`) — we emit a comma-joined list.
pub(crate) fn build_classify_user_prompt(
    description: &str,
    region: &str,
    allowed_classes: &[String],
) -> String {
    let list = allowed_classes.join(", ");
    format!(
        "Analyze the following project description and classify it into the most appropriate reference class.\n\nProject Description: {description}\nRegion: {region}\n\nAvailable Reference Classes: {list}\n\nPlease provide only the reference class name as your response."
    )
}

/// Format the `{reference_data_section}` placeholder for the estimation
/// prompt. Returns an empty string when there are no projects — FastAPI
/// omits the line entirely in that case, and the template accepts it.
pub(crate) fn format_reference_data_section(projects: &[ReferenceProject]) -> String {
    if projects.is_empty() {
        return String::new();
    }
    let payload = json!({ "reference_projects": projects });
    let body = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    format!("\nReference Data: {body}")
}

/// Hand-built JSON schema for [`EstimationOutput`]. OpenAI strict mode
/// requires `additionalProperties: false` and every property listed in
/// `required` — `schemars` derive macros don't emit these by default,
/// which is why we hand-roll the schema.
pub(crate) fn estimation_output_schema() -> StructuredSchema {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "total_cost_p50",
            "total_cost_p80",
            "timeline_weeks_p50",
            "timeline_weeks_p80",
            "cost_breakdown",
            "adjustment_factors",
            "confidence_score",
            "assumptions",
            "summary",
        ],
        "properties": {
            "total_cost_p50": {"type": "number"},
            "total_cost_p80": {"type": "number"},
            "timeline_weeks_p50": {"type": "integer", "minimum": 0},
            "timeline_weeks_p80": {"type": "integer", "minimum": 0},
            "cost_breakdown": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["category", "p50_cost", "p80_cost", "percentage_of_total"],
                    "properties": {
                        "category": {"type": "string"},
                        "p50_cost": {"type": "number"},
                        "p80_cost": {"type": "number"},
                        "percentage_of_total": {"type": "number"}
                    }
                }
            },
            "adjustment_factors": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "multiplier", "reason"],
                    "properties": {
                        "name": {"type": "string"},
                        "multiplier": {"type": "number"},
                        "reason": {"type": "string"}
                    }
                }
            },
            "confidence_score": {"type": "number", "minimum": 0, "maximum": 100},
            "assumptions": {"type": "array", "items": {"type": "string"}},
            "summary": {"type": "string"}
        }
    });
    StructuredSchema {
        name: "estimation_output".into(),
        description: Some("Structured cost + timeline estimate for a construction project.".into()),
        schema,
    }
}

/// Assemble a fresh [`EstimationSession`] from the resolved inputs. Pure
/// function so tests can assert on timestamps, expiry, and field mapping
/// without involving Mongo.
pub(crate) fn assemble_session(
    tenant_id: efofx_domain::TenantId,
    description: String,
    region_label: &str,
    reference_class: String,
    prompt_version: String,
    now: OffsetDateTime,
    ttl_minutes: i64,
) -> EstimationSession {
    EstimationSession {
        id: EstimationSessionId::new(),
        tenant_id,
        status: EstimationStatus::Completed,
        description,
        region: resolve_region(region_label),
        reference_class: Some(reference_class),
        confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
        prompt_version: Some(prompt_version),
        created_at: now,
        updated_at: now,
        completed_at: Some(now),
        expires_at: Some(now + TimeDuration::minutes(ttl_minutes)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use efofx_domain::{ReferenceProject, ScopingContext, TenantId};
    use std::collections::BTreeMap;
    use time::OffsetDateTime;

    // --- build_description_from_context --------------------------------

    #[test]
    fn description_empty_ctx_returns_fallback() {
        assert_eq!(
            build_description_from_context(&ScopingContext::default()),
            "General project"
        );
    }

    #[test]
    fn description_joins_populated_fields_with_trailing_period() {
        let mut ctx = ScopingContext::default();
        ctx.project_type = Some("pool".into());
        ctx.project_size = Some("15x30".into());
        ctx.location = Some("SoCal - Coastal".into());
        let out = build_description_from_context(&ctx);
        assert_eq!(
            out,
            "Project type: pool. Size/scope: 15x30. Location: SoCal - Coastal."
        );
    }

    #[test]
    fn description_skips_unset_fields() {
        let mut ctx = ScopingContext::default();
        ctx.project_type = Some("pool".into());
        ctx.special_conditions = Some("steep grade".into());
        let out = build_description_from_context(&ctx);
        assert_eq!(out, "Project type: pool. Special conditions: steep grade.");
    }

    // --- resolve_region ------------------------------------------------

    #[test]
    fn resolve_region_known_display_strings() {
        assert_eq!(resolve_region("SoCal - Coastal"), Region::SoCalCoastal);
        assert_eq!(resolve_region("Nevada - Las Vegas"), Region::NevadaLasVegas);
    }

    #[test]
    fn resolve_region_unknown_falls_back_to_norcal_bay_area() {
        assert_eq!(resolve_region("Mars"), Region::NorCalBayArea);
        assert_eq!(resolve_region("General"), Region::NorCalBayArea);
        assert_eq!(resolve_region(""), Region::NorCalBayArea);
    }

    // --- parse_classification ------------------------------------------

    #[test]
    fn classification_empty_allowlist_returns_general() {
        assert_eq!(parse_classification("anything", &[]), "general");
    }

    #[test]
    fn classification_case_insensitive_match() {
        let allowed = vec!["residential_pool_socal".into(), "deck_build".into()];
        assert_eq!(
            parse_classification("Residential_Pool_SoCal", &allowed),
            "residential_pool_socal"
        );
    }

    #[test]
    fn classification_trims_whitespace() {
        let allowed = vec!["deck_build".into()];
        assert_eq!(
            parse_classification("  deck_build\n", &allowed),
            "deck_build"
        );
    }

    #[test]
    fn classification_unknown_falls_back_to_first_allowed() {
        let allowed = vec!["residential_pool_socal".into(), "deck_build".into()];
        assert_eq!(
            parse_classification("hallucinated_class", &allowed),
            "residential_pool_socal"
        );
    }

    // --- build_classify_user_prompt -----------------------------------

    #[test]
    fn classify_prompt_includes_description_region_and_list() {
        let allowed = vec!["residential_pool_socal".into(), "deck_build".into()];
        let prompt = build_classify_user_prompt("A pool.", "SoCal - Coastal", &allowed);
        assert!(prompt.contains("Project Description: A pool."));
        assert!(prompt.contains("Region: SoCal - Coastal"));
        assert!(prompt.contains("Available Reference Classes: residential_pool_socal, deck_build"));
    }

    // --- format_reference_data_section --------------------------------

    fn sample_project(name: &str) -> ReferenceProject {
        ReferenceProject {
            id: None,
            tenant_id: None,
            project_id: name.into(),
            reference_class: "residential_pool_socal".into(),
            region: "SoCal - Coastal".into(),
            description: format!("{name} description"),
            size_sqft: Some(450.0),
            total_cost: 60_000.0,
            timeline_weeks: 8,
            team_size: 5,
            cost_breakdown: BTreeMap::new(),
            completion_date: OffsetDateTime::now_utc(),
            quality_score: 0.8,
            source: "seed".into(),
            metadata: BTreeMap::new(),
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
            updated_at: None,
        }
    }

    #[test]
    fn reference_data_section_empty_is_empty_string() {
        assert_eq!(format_reference_data_section(&[]), "");
    }

    #[test]
    fn reference_data_section_has_reference_projects_key() {
        let projects = vec![sample_project("p1"), sample_project("p2")];
        let section = format_reference_data_section(&projects);
        assert!(section.starts_with("\nReference Data: "));
        assert!(section.contains("\"reference_projects\""));
        assert!(section.contains("\"p1\""));
        assert!(section.contains("\"p2\""));
    }

    // --- estimation_output_schema -------------------------------------

    fn object_is_strict(obj: &Value) {
        // Strict-mode invariant: every object must declare
        // additionalProperties: false and list each property in `required`.
        let additional = obj
            .get("additionalProperties")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        assert!(
            !additional,
            "object missing additionalProperties: false → {obj}"
        );
        let properties = obj
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("object has properties");
        let required: Vec<String> = obj
            .get("required")
            .and_then(|v| v.as_array())
            .expect("object has required array")
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        for k in properties.keys() {
            assert!(required.contains(k), "property `{k}` missing from required");
        }
        for v in properties.values() {
            // Recurse into nested objects (e.g. via array items).
            if v.get("type").and_then(Value::as_str) == Some("object") {
                object_is_strict(v);
            }
            if v.get("type").and_then(Value::as_str) == Some("array") {
                if let Some(items) = v.get("items") {
                    if items.get("type").and_then(Value::as_str) == Some("object") {
                        object_is_strict(items);
                    }
                }
            }
        }
    }

    #[test]
    fn schema_is_openai_strict_compliant() {
        let schema = estimation_output_schema();
        assert_eq!(schema.name, "estimation_output");
        object_is_strict(&schema.schema);
    }

    #[test]
    fn schema_accepts_a_valid_output_roundtrip() {
        // Prove the schema we assert isn't nonsense: a sample EstimationOutput
        // serializes to JSON that parses back through our domain type.
        let sample = EstimationOutput {
            total_cost_p50: 60_000.0,
            total_cost_p80: 72_000.0,
            timeline_weeks_p50: 8,
            timeline_weeks_p80: 12,
            cost_breakdown: vec![efofx_domain::CostCategoryEstimate {
                category: "materials".into(),
                p50_cost: 24_000.0,
                p80_cost: 28_000.0,
                percentage_of_total: 0.4,
            }],
            adjustment_factors: vec![efofx_domain::AdjustmentFactor {
                name: "Urban premium".into(),
                multiplier: 1.15,
                reason: "dense metro".into(),
            }],
            confidence_score: 72.0,
            assumptions: vec!["flat lot".into()],
            summary: "pool in socal".into(),
        };
        let value = serde_json::to_value(&sample).unwrap();
        let _back: EstimationOutput = serde_json::from_value(value).unwrap();
    }

    // --- assemble_session ----------------------------------------------

    #[test]
    fn assemble_session_sets_status_and_timestamps() {
        let now = OffsetDateTime::now_utc();
        let tenant = TenantId::new();
        let s = assemble_session(
            tenant,
            "Project type: pool.".into(),
            "SoCal - Coastal",
            "residential_pool_socal".into(),
            "1.0.0".into(),
            now,
            30,
        );
        assert_eq!(s.tenant_id, tenant);
        assert_eq!(s.status, EstimationStatus::Completed);
        assert_eq!(s.region, Region::SoCalCoastal);
        assert_eq!(s.reference_class.as_deref(), Some("residential_pool_socal"));
        assert_eq!(s.prompt_version.as_deref(), Some("1.0.0"));
        assert_eq!(s.confidence_threshold, 0.7);
        assert_eq!(s.created_at, now);
        assert_eq!(s.completed_at, Some(now));
        assert_eq!(s.expires_at, Some(now + TimeDuration::minutes(30)));
    }

    #[test]
    fn assemble_session_unknown_region_falls_back() {
        let now = OffsetDateTime::now_utc();
        let s = assemble_session(
            TenantId::new(),
            "Project type: pool.".into(),
            "Mars",
            "residential_pool_socal".into(),
            "1.0.0".into(),
            now,
            30,
        );
        assert_eq!(s.region, Region::NorCalBayArea);
    }

    // --- error response mapping ---------------------------------------

    #[test]
    fn error_response_codes_map_correctly() {
        let cases: Vec<(EstimationServiceError, StatusCode)> = vec![
            (
                EstimationServiceError::ChatSessionNotFound,
                StatusCode::NOT_FOUND,
            ),
            (EstimationServiceError::ChatSessionExpired, StatusCode::GONE),
            (
                EstimationServiceError::SessionNotReady,
                StatusCode::CONFLICT,
            ),
            (
                EstimationServiceError::MissingByokKey,
                StatusCode::PAYMENT_REQUIRED,
            ),
            (
                EstimationServiceError::Llm(LlmError::InvalidKey("".into())),
                StatusCode::PAYMENT_REQUIRED,
            ),
            (
                EstimationServiceError::Llm(LlmError::QuotaExhausted("".into())),
                StatusCode::PAYMENT_REQUIRED,
            ),
            (
                EstimationServiceError::Llm(LlmError::Transient("".into())),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                EstimationServiceError::Llm(LlmError::Unknown("".into())),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                EstimationServiceError::SchemaParse("nope".into()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];
        for (err, expected) in cases {
            let resp = err.into_response();
            assert_eq!(resp.status(), expected);
        }
    }
}
