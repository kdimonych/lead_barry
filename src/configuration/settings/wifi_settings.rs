use core::str::FromStr;

use embassy_net::Ipv4Address;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format)]
#[non_exhaustive]
pub struct WiFiSettings {
    pub ssid: heapless::String<32>,
    pub password: heapless::String<64>,
}

impl WiFiSettings {
    pub const fn new() -> Self {
        Self {
            ssid: heapless::String::new(),
            password: heapless::String::new(),
        }
    }
}

impl Default for WiFiSettings {
    fn default() -> Self {
        Self {
            ssid: heapless::String::from_str(option_env!("DBG_WIFI_SSID").unwrap_or("")).unwrap(),
            password: heapless::String::from_str(option_env!("DBG_WIFI_PASSWORD").unwrap_or(""))
                .unwrap(),
        }
    }
}
