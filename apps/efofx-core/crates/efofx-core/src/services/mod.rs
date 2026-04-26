//! Application services — small structs that own a slice of functionality
//! used by handlers. Services depend on repositories (in `efofx-storage`)
//! and pure-function crates (in `efofx-crypto`), and are themselves
//! composed into `AppState`.

pub mod byok;
pub mod calibration;
pub mod chat;
pub mod estimation;
pub mod routing;

pub use byok::{ByokError, ByokService};
pub use calibration::{CalibrationService, CalibrationServiceError};
pub use chat::{AppendOutcome, ChatService, ChatServiceError};
pub use estimation::{EstimationOutcome, EstimationService, EstimationServiceError};
pub use routing::{derive as derive_routing, render_directory_url, CostTier, RoutingData};
