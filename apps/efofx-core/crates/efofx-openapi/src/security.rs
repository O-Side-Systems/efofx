//! Security-scheme modifier for the OpenAPI document.
//!
//! Declares both auth surfaces the API accepts:
//! - `supabase_jwt` — Bearer JWT, verified via JWKS
//! - `widget_api_key` — long-lived per-tenant API key sent as `X-Api-Key`

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify,
};

/// Apply with `#[openapi(modifiers(&efofx_openapi::security::SecurityAddon))]`
/// on the binary crate's `ApiDoc`.
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "supabase_jwt",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some(
                            "Supabase-issued JWT. Verified against the Supabase \
                             project JWKS. Carries tenant identity via the `sub` claim.",
                        ))
                        .build(),
                ),
            );
            components.add_security_scheme(
                "widget_api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                    "X-Api-Key",
                    "Long-lived per-tenant API key used by the embeddable widget. \
                     Bound to a single tenant; rotate via `POST /v1/me/api-keys:rotate`.",
                ))),
            );
        }
    }
}
