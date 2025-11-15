use core::fmt;

use heapless::Vec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format)]
pub struct StaticIpConfig {
    pub ip: u32,
    pub gateway: Option<u32>,
    pub prefix_len: u8,
    pub dns_servers: Vec<u32, 3>, // Optional DNS server
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format)]
pub struct Settings {
    pub wifi_ssid: heapless::String<32>,
    pub wifi_password: heapless::String<64>,
    pub use_static_ip_config: bool,
    pub static_ip_config: Option<StaticIpConfig>,
    pub settings_version: u32,
}

impl Settings {
    pub const fn new() -> Self {
        Self {
            wifi_ssid: heapless::String::new(),
            wifi_password: heapless::String::new(),
            settings_version: 1,
            use_static_ip_config: false,
            static_ip_config: None,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl StaticIpConfig {
    pub const fn new() -> Self {
        Self {
            ip: 0u32,
            gateway: None,
            prefix_len: 0u8,
            dns_servers: Vec::new(),
        }
    }
}

impl Default for StaticIpConfig {
    fn default() -> Self {
        Self::new()
    }
}
