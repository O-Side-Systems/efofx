//! Application services — small structs that own a slice of functionality
//! used by handlers. Services depend on repositories (in `efofx-storage`)
//! and pure-function crates (in `efofx-crypto`), and are themselves
//! composed into `AppState`.

pub mod byok;

pub use byok::{ByokError, ByokService};
