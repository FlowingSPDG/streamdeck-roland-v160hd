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
}

#[derive(Debug, Deserialize)]
pub struct PiMessage {
    #[serde(default)]
    pub property_inspector: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PiOut {
    pub status: String,
}

impl PiOut {
    pub fn status(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
        }
    }
}
