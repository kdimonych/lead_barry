use core::str::FromStr;

use embassy_net::Ipv4Address;
use serde::{Deserialize, Serialize};

const DEFAULT_AP_IP: Ipv4Address = Ipv4Address::new(192, 168, 1, 1);
const DEFAULT_WIFI_AP_PREFIX_LEN: u8 = 24;
const DEFAULT_AP_SSID: &str = "LeadBarry";
const DEFAULT_AP_CHANNEL: u8 = 6;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format)]
#[non_exhaustive]
pub struct WiFiApSettings {
    pub ssid: heapless::String<32>,
    pub password: Option<heapless::String<64>>,
    pub channel: u8,
    pub ip: u32,
    pub prefix_len: u8,
}

impl WiFiApSettings {
    pub const fn new() -> Self {
        Self {
            ssid: heapless::String::new(),
            password: None, // None means generate a random password
            channel: DEFAULT_AP_CHANNEL,
            ip: DEFAULT_AP_IP.to_bits(),
            prefix_len: 24,
        }
    }
}

impl Default for WiFiApSettings {
    fn default() -> Self {
        Self {
            ssid: heapless::String::from_str(
                option_env!("DBG_WIFI_AP_SSID").unwrap_or(DEFAULT_AP_SSID),
            )
            .unwrap(),
            password: option_env!("DBG_WIFI_AP_PASSWORD")
                .map(|str| heapless::String::from_str(str).unwrap()),
            channel: option_env!("DBG_WIFI_AP_CHANNEL")
                .map(|str| str.parse().unwrap_or(DEFAULT_AP_CHANNEL))
                .unwrap_or(DEFAULT_AP_CHANNEL),
            ip: option_env!("DBG_WIFI_AP_IP")
                .map(|str| str.parse().unwrap_or(DEFAULT_AP_IP.to_bits()))
                .unwrap_or(DEFAULT_AP_IP.to_bits()),
            prefix_len: option_env!("DBG_WIFI_AP_PREFIX_LEN")
                .map(|str| str.parse().unwrap_or(24))
                .unwrap_or(DEFAULT_WIFI_AP_PREFIX_LEN),
        }
    }
}
