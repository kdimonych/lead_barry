use super::static_ip_config::StaticIpConfig;
use serde::{Deserialize, Serialize};

pub const DEFAULT_AP_CHANNEL: u8 = 6;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format)]
#[non_exhaustive]
pub struct NetworkSettings {
    pub wifi_ssid: heapless::String<32>,
    pub wifi_password: heapless::String<64>,
    pub ap_channel: u8,
    pub use_static_ip_config: bool,
    pub static_ip_config: Option<StaticIpConfig>,
}

impl NetworkSettings {
    pub const fn new() -> Self {
        Self {
            wifi_ssid: heapless::String::new(),
            wifi_password: heapless::String::new(),
            ap_channel: DEFAULT_AP_CHANNEL,
            use_static_ip_config: false,
            static_ip_config: None,
        }
    }
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self::new()
    }
}
