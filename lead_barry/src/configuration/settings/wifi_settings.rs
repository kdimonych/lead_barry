use super::static_ip_config::StaticIpConfig;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct WiFiSettings {
    pub ssid: heapless::String<32>,
    pub password: Option<heapless::String<64>>,
    pub use_static_ip_config: bool,
    pub static_ip_config: Option<StaticIpConfig>,
}

impl WiFiSettings {
    pub const fn new() -> Self {
        Self {
            ssid: heapless::String::new(),
            password: None,
            use_static_ip_config: false,
            static_ip_config: None,
        }
    }
}

impl Default for WiFiSettings {
    fn default() -> Self {
        Self {
            ssid: heapless::String::from_str(option_env!("DBG_WIFI_SSID").unwrap_or("")).unwrap(),
            password: option_env!("DBG_WIFI_PASSWORD")
                .map(|s| heapless::String::from_str(s).unwrap_or_default()),
            use_static_ip_config: option_env!("DBG_USE_STATIC_IP_CONFIG")
                .map(|str| str.parse().unwrap_or(false))
                .unwrap_or(false),
            static_ip_config: None,
        }
    }
}
