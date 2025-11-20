use super::wifi_ap_settings::WiFiApSettings;
use super::wifi_settings::WiFiSettings;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format)]
#[non_exhaustive]
pub struct NetworkSettings {
    pub wifi_settings: WiFiSettings,
    pub wifi_ap_settings: WiFiApSettings,
}

impl NetworkSettings {
    pub const fn new() -> Self {
        Self {
            wifi_settings: WiFiSettings::new(),
            wifi_ap_settings: WiFiApSettings::new(),
        }
    }
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            wifi_settings: WiFiSettings::default(),
            wifi_ap_settings: WiFiApSettings::default(),
        }
    }
}
