mod network_settings;
mod static_ip_config;

use core::str::FromStr;
use serde::{Deserialize, Serialize};

pub use network_settings::*;
pub use static_ip_config::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format)]
#[non_exhaustive]
pub struct Settings {
    pub network_settings: NetworkSettings,
    pub settings_version: u32,
}

impl Settings {
    pub const fn new() -> Self {
        Self {
            network_settings: NetworkSettings::new(),
            settings_version: 1,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature_use_debug_settings)]
pub fn debug_settings() -> Option<Settings> {
    defmt::info!("Current Settings:");
    defmt::info!("WiFi SSID: {}", env!("DBG_WIFI_SSID"));
    defmt::info!("WiFi Password: ********");

    let static_ip_config = debug_static_ip_config();

    Some(Settings {
        network_settings: NetworkSettings {
            wifi_ssid: heapless::String::from_str(env!("DBG_WIFI_SSID")).unwrap(),
            wifi_password: heapless::String::from_str(env!("DBG_WIFI_PASSWORD")).unwrap(),
            ap_channel: env!("DBG_AP_CHANNEL").parse().unwrap_or(DEFAULT_AP_CHANNEL),
            use_static_ip_config: static_ip_config.is_some(),
            static_ip_config,
        },
        settings_version: 1,
    })
}

#[cfg(not(feature_use_debug_settings))]
pub fn debug_settings() -> Option<Settings> {
    Some(Settings::new())
}
