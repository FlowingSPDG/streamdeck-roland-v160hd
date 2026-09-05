//! Action settings as persisted by Stream Deck.
//!
//! This struct is only a serde view of `payload.settings` on each event.
//! Do not store it in a HashMap keyed by context.

use serde::{Deserialize, Serialize};

fn default_password() -> String {
    "0000".to_string()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ActionSettings {
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_password")]
    pub password: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub switch: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub input_assign: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub output_assign: String,
    #[serde(default)]
    pub aux_bus: String,
    #[serde(default)]
    pub aux_op: String,
    #[serde(default)]
    pub link_mode: String,
    #[serde(default)]
    pub mix_layer: String,
    #[serde(default)]
    pub pinp_key: String,
    #[serde(default)]
    pub pinp_op: String,
    #[serde(default)]
    pub bus: String,
    #[serde(default)]
    pub pinp_type: String,
    #[serde(default)]
    pub shape: String,
    #[serde(default)]
    pub border_color: String,
    #[serde(default)]
    pub chroma_color: String,
    #[serde(default)]
    pub dsk_ch: String,
    #[serde(default)]
    pub dsk_op: String,
    #[serde(default)]
    pub dsk_type: String,
    #[serde(default)]
    pub trans_op: String,
    #[serde(default)]
    pub trans_time: String,
    #[serde(default)]
    pub tenths: String,
    #[serde(default)]
    pub trans_type: String,
    #[serde(default)]
    pub mix_type: String,
    #[serde(default)]
    pub wipe_type: String,
    #[serde(default)]
    pub wipe_direction: String,
    #[serde(default)]
    pub mem_op: String,
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub freeze_op: String,
    #[serde(default)]
    pub freeze_type: String,
    #[serde(default)]
    pub freeze_input: String,
    #[serde(default)]
    pub macro_n: String,
    #[serde(default)]
    pub camera_id: String,
    #[serde(default)]
    pub cam_op: String,
    #[serde(default)]
    pub preset: String,
    #[serde(default)]
    pub pan: String,
    #[serde(default)]
    pub tilt: String,
    #[serde(default)]
    pub zoom: String,
    #[serde(default)]
    pub focus: String,
    #[serde(default)]
    pub pt_speed: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub linked: bool,
    #[serde(default)]
    pub auto_focus: bool,
    #[serde(default)]
    pub exposure_auto: bool,
    #[serde(default)]
    pub tally_check: String,
    /// `manual` shows Host / Password / Test connection. `saved` uses an existing endpoint.
    #[serde(default)]
    pub connection_mode: String,
    /// `"true"` after a successful Test connection or saved-endpoint pick.
    /// `"false"` while the user is editing a new host. Empty means legacy settings.
    #[serde(default)]
    pub connection_verified: String,
}

impl ActionSettings {
    pub fn password(&self) -> &str {
        if self.password.is_empty() {
            "0000"
        } else {
            &self.password
        }
    }

    pub fn host_trimmed(&self) -> &str {
        self.host.trim()
    }

    /// Persistent TCP is opened only for a saved endpoint, a successful test,
    /// or legacy settings that already stored a host.
    pub fn should_connect(&self) -> bool {
        if self.host_trimmed().is_empty() {
            return false;
        }
        match self.connection_verified.as_str() {
            "true" => true,
            "false" => false,
            _ => true,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PiMessage {
    #[serde(default)]
    pub property_inspector: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointInfo {
    pub host: String,
    pub password: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct PiOut {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endpoints: Vec<EndpointInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tested: Option<bool>,
}

impl PiOut {
    pub fn state(status: impl Into<String>, endpoints: Vec<EndpointInfo>) -> Self {
        Self {
            status: status.into(),
            endpoints,
            tested: None,
        }
    }

    pub fn test_result(status: impl Into<String>, ok: bool) -> Self {
        Self {
            status: status.into(),
            endpoints: Vec::new(),
            tested: Some(ok),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_message_reads_test_connection() {
        let msg: PiMessage = serde_json::from_str(
            r#"{"command":"test_connection","host":"192.168.0.10","password":"1234"}"#,
        )
        .unwrap();
        assert_eq!(msg.command.as_deref(), Some("test_connection"));
        assert_eq!(msg.host.as_deref(), Some("192.168.0.10"));
        assert_eq!(msg.password.as_deref(), Some("1234"));
    }

    #[test]
    fn pi_out_omits_empty_endpoints() {
        let json = serde_json::to_value(PiOut::state("Connected", Vec::new())).unwrap();
        assert_eq!(json["status"], "Connected");
        assert!(json.get("endpoints").is_none());
        assert!(json.get("tested").is_none());
        assert!(json.get("log_path").is_none());
        assert!(json.get("logs").is_none());
    }

    #[test]
    fn pi_out_test_result_marks_success() {
        let json =
            serde_json::to_value(PiOut::test_result("Connected (V-160HD 1.10)", true)).unwrap();
        assert_eq!(json["tested"], true);
        assert!(json.get("endpoints").is_none());
    }

    #[test]
    fn should_connect_waits_for_test_unless_legacy() {
        let editing = ActionSettings {
            host: "192.168.0.10".into(),
            connection_verified: "false".into(),
            ..ActionSettings::default()
        };
        assert!(!editing.should_connect());

        let tested = ActionSettings {
            host: "192.168.0.10".into(),
            connection_verified: "true".into(),
            ..ActionSettings::default()
        };
        assert!(tested.should_connect());

        let legacy = ActionSettings {
            host: "192.168.0.10".into(),
            ..ActionSettings::default()
        };
        assert!(legacy.should_connect());

        let empty = ActionSettings::default();
        assert!(!empty.should_connect());
    }
}
