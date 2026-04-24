//! Shared test utilities for the Phase 2A contract-test suite.
//!
//! Spins up a wiremock JWKS endpoint backed by a generated RSA key pair,
//! builds an in-process axum `Router` with a fake Mongo handle (never
//! actually queried in rejection tests), and provides a JWT signer so
//! tests can mint valid and deliberately-malformed tokens.

#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use efofx_auth::{middleware::AuthState, JwksCache};
use efofx_config::SupabaseConfig;
use efofx_core::{
    build_router,
    services::{ByokService, ChatService, EstimationService},
    AppState,
};
use efofx_llm::MockLlmProvider;
use efofx_prompts::PromptRegistry;
use efofx_storage::auth::{ApiKeyAuth, MasterKey, TenantResolver};
use efofx_storage::{ChatRepo, EstimationRepo, MongoAdapter, ReferenceRepo, TenantRepo};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const TEST_AUDIENCE: &str = "authenticated";
pub const TEST_KID: &str = "test-kid-1";

/// A signer bundling the RSA private key and the JWT `kid` / algorithm
/// needed to mint tokens accepted by our middleware.
pub struct TestSigner {
    pub private_key: RsaPrivateKey,
    pub public_key: RsaPublicKey,
    pub kid: String,
}

impl TestSigner {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("generate 2048-bit key");
        let public_key = RsaPublicKey::from(&private_key);
        Self {
            private_key,
            public_key,
            kid: TEST_KID.to_string(),
        }
    }

    /// Serialize the public key as a Supabase-shaped JWKS document.
    pub fn jwks_document(&self) -> serde_json::Value {
        let n_b64 = URL_SAFE_NO_PAD.encode(self.public_key.n().to_bytes_be());
        let e_b64 = URL_SAFE_NO_PAD.encode(self.public_key.e().to_bytes_be());
        json!({
            "keys": [{
                "kty": "RSA",
                "kid": self.kid,
                "use": "sig",
                "alg": "RS256",
                "n": n_b64,
                "e": e_b64,
            }]
        })
    }

    /// Mint a signed JWT with the supplied claims. `kid` is set to this
    /// signer's configured value — use [`Self::sign_with_kid`] for
    /// unknown-kid cases.
    pub fn sign(&self, claims: &serde_json::Value) -> String {
        self.sign_with_kid(&self.kid, claims)
    }

    pub fn sign_with_kid(&self, kid: &str, claims: &serde_json::Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());
        let der = self
            .private_key
            .to_pkcs1_der()
            .expect("pkcs1 der")
            .to_bytes();
        let enc = EncodingKey::from_rsa_der(&der);
        jsonwebtoken::encode(&header, claims, &enc).expect("sign jwt")
    }
}

/// Test harness: mock JWKS server + wired router + config knobs the test
/// can twist to force failure modes.
pub struct TestHarness {
    pub mock_server: MockServer,
    pub signer: TestSigner,
    pub router: Router,
    pub supabase: SupabaseConfig,
}

impl TestHarness {
    pub async fn new() -> Self {
        let signer = TestSigner::new();
        let mock_server = MockServer::start().await;
        let jwks_doc = signer.jwks_document();

        Mock::given(method("GET"))
            .and(path("/auth/v1/.well-known/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(jwks_doc))
            .mount(&mock_server)
            .await;

        let supabase = SupabaseConfig {
            url: mock_server.uri(),
            issuer: format!("{}/auth/v1", mock_server.uri()),
            audience: TEST_AUDIENCE.to_string(),
            jwks_refresh_seconds: 3600,
        };

        let mongo = MongoAdapter::for_tests_without_ping("mongodb://127.0.0.1:1/", "efofx_test")
            .await
            .expect("build test mongo adapter");
        let tenants = TenantRepo::new(mongo.clone());
        let resolver = TenantResolver::new(mongo.clone());
        let master_key = MasterKey::from_bytes(vec![0x13; 32]).unwrap();
        let api_key_auth = ApiKeyAuth::new(master_key.clone());
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap();
        let byok = ByokService::new(tenants.clone(), Arc::new(master_key), http);

        let chat_repo = ChatRepo::new(mongo.clone());
        let estimates_repo = EstimationRepo::new(mongo.clone());
        let reference_repo = ReferenceRepo::new(mongo.clone());
        // Auth contract tests never reach the chat / estimation handlers,
        // but AppState requires fully-wired services. Use an empty prompt
        // registry and a mock LLM so construction succeeds without
        // touching real Mongo / OpenAI state.
        let prompts = Arc::new(PromptRegistry::default());
        let llm: Arc<dyn efofx_llm::LlmProvider> = Arc::new(MockLlmProvider::new());
        let chat = ChatService::new(
            chat_repo.clone(),
            Arc::clone(&prompts),
            Arc::clone(&llm),
            efofx_config::LlmConfig::default(),
        );
        let estimation = EstimationService::new(
            chat_repo,
            estimates_repo,
            reference_repo,
            prompts,
            llm,
            efofx_config::LlmConfig::default(),
        );

        let jwks = JwksCache::bootstrap(&supabase.url, Duration::from_secs(3600))
            .await
            .expect("jwks bootstrap");

        let auth = AuthState {
            jwks,
            supabase: Arc::new(supabase.clone()),
            resolver,
            api_key: api_key_auth.clone(),
        };

        let state = Arc::new(AppState {
            mongo,
            tenants,
            byok,
            chat,
            estimation,
            api_key_auth,
            auth,
        });
        let router = build_router(state);

        Self {
            mock_server,
            signer,
            router,
            supabase,
        }
    }

    /// Convenience — mint a JWT whose claims would otherwise verify, so
    /// tests only need to override the fields they care about.
    pub fn valid_claims(&self, sub: &str) -> serde_json::Value {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        json!({
            "iss": self.supabase.issuer,
            "aud": self.supabase.audience,
            "sub": sub,
            "email": format!("{sub}@example.com"),
            "exp": now + 3600,
            "nbf": now - 60,
            "iat": now,
            "user_metadata": { "company_name": "Test Co" },
        })
    }
}
