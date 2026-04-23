use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Allowed widget analytics event types. Any write that does not match one
/// of these is rejected with `widget.invalid_event_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEventType {
    WidgetView,
    ChatStart,
    EstimateComplete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytics_wire_format() {
        assert_eq!(
            serde_json::to_string(&AnalyticsEventType::WidgetView).unwrap(),
            "\"widget_view\""
        );
        let parsed: AnalyticsEventType = serde_json::from_str("\"estimate_complete\"").unwrap();
        assert_eq!(parsed, AnalyticsEventType::EstimateComplete);
    }
}
