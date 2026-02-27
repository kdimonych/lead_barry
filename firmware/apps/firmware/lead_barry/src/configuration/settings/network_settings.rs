use super::wifi_ap_settings::WiFiApSettings;
use super::wifi_settings::WiFiSettings;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
#[defmt_or_log::derive_format_or_debug]
#[non_exhaustive]
pub struct NetworkSettings {
    pub wifi_settings: WiFiSettings,
    pub wifi_ap_settings: WiFiApSettings,
}

#[allow(dead_code)]
impl NetworkSettings {
    pub const fn new() -> Self {
        Self {
            wifi_settings: WiFiSettings::new(),
            wifi_ap_settings: WiFiApSettings::new(),
        }
    }
}
