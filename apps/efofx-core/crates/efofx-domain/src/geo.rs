use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Geographic region used for reference-class selection and cost adjustment.
///
/// Wire format matches `efofx_shared.core.constants.Region` — display strings
/// with hyphens and spaces, not snake_case (e.g. `"SoCal - Coastal"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum Region {
    #[serde(rename = "SoCal - Coastal")]
    SoCalCoastal,
    #[serde(rename = "SoCal - Inland")]
    SoCalInland,
    #[serde(rename = "NorCal - Bay Area")]
    NorCalBayArea,
    #[serde(rename = "NorCal - Central")]
    NorCalCentral,
    #[serde(rename = "Arizona - Phoenix")]
    ArizonaPhoenix,
    #[serde(rename = "Arizona - Tucson")]
    ArizonaTucson,
    #[serde(rename = "Nevada - Las Vegas")]
    NevadaLasVegas,
    #[serde(rename = "Nevada - Reno")]
    NevadaReno,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_wire_format_matches_fastapi() {
        assert_eq!(
            serde_json::to_string(&Region::SoCalCoastal).unwrap(),
            "\"SoCal - Coastal\""
        );
        let parsed: Region = serde_json::from_str("\"Nevada - Las Vegas\"").unwrap();
        assert_eq!(parsed, Region::NevadaLasVegas);
    }
}
