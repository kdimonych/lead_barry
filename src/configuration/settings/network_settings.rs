use super::static_ip_config::StaticIpConfig;
use super::wifi_ap_settings::WiFiApSettings;
use super::wifi_settings::WiFiSettings;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format)]
#[non_exhaustive]
pub struct NetworkSettings {
    pub wifi_settings: WiFiSettings,
    pub wifi_ap_settings: WiFiApSettings,

    pub use_static_ip_config: bool,
    pub static_ip_config: Option<StaticIpConfig>,
}

impl NetworkSettings {
    pub const fn new() -> Self {
        Self {
            wifi_settings: WiFiSettings::new(),
            wifi_ap_settings: WiFiApSettings::new(),
            use_static_ip_config: false,
            static_ip_config: None,
        }
    }
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            wifi_settings: WiFiSettings::default(),
            wifi_ap_settings: WiFiApSettings::default(),
            use_static_ip_config: option_env!("DBG_USE_STATIC_IP_CONFIG")
                .map(|str| str.parse().unwrap_or(false))
                .unwrap_or(false),
            static_ip_config: None,
        }
    }
}
