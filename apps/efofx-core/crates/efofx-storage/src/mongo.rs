use efofx_config::MongoConfig;
use mongodb::{options::ClientOptions, Client, Database};
use tracing::info;

use crate::StorageError;

/// Thin wrapper around a Mongo [`Client`]. Collection handles are exposed
/// only through repository structs defined in their bounded-area crates.
#[derive(Debug, Clone)]
pub struct MongoAdapter {
    client: Client,
    db_name: String,
}

impl MongoAdapter {
    pub async fn connect(cfg: &MongoConfig) -> Result<Self, StorageError> {
        let mut opts = ClientOptions::parse(&cfg.uri).await?;
        opts.app_name = Some("efofx-core".into());
        let client = Client::with_options(opts)?;
        // Touch the server so a bad URI fails fast at startup.
        client
            .database(&cfg.db_name)
            .run_command(mongodb::bson::doc! { "ping": 1 })
            .await?;
        info!(db = %cfg.db_name, "mongo connected");
        Ok(Self {
            client,
            db_name: cfg.db_name.clone(),
        })
    }

    pub fn database(&self) -> Database {
        self.client.database(&self.db_name)
    }

    pub async fn ping(&self) -> Result<(), StorageError> {
        self.database()
            .run_command(mongodb::bson::doc! { "ping": 1 })
            .await?;
        Ok(())
    }
}
