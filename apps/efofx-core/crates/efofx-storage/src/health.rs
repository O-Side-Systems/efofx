use serde::Serialize;
use utoipa::ToSchema;

use crate::MongoAdapter;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HealthStatus {
    pub status: &'static str,
    pub mongo: &'static str,
}

impl HealthStatus {
    pub async fn probe(mongo: &MongoAdapter) -> Self {
        let mongo_ok = mongo.ping().await.is_ok();
        Self {
            status: if mongo_ok { "ok" } else { "degraded" },
            mongo: if mongo_ok { "ok" } else { "unreachable" },
        }
    }
}
