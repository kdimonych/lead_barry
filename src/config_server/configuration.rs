use core::u32;

use crate::configuration::{ConfigurationStorage, Settings, StaticIpConfig};
use embassy_rp::usb::In;
use heapless::Vec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format, Default)]
pub struct HttpStaticIpConfig {
    pub ip: [u8; 4],
    pub gateway: Option<[u8; 4]>,
    pub prefix_len: u8,
    pub dns: Vec<[u8; 4], 3>, // Optional DNS server
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format, Default)]
struct HttpConfiguration {
    ssid: heapless::String<32>,
    password: heapless::String<64>,
    use_static_ip: bool,
    static_ip_config: Option<HttpStaticIpConfig>,
}

impl HttpStaticIpConfig {
    pub fn to_static_ip_config() {
        // TODO: implement!!
    }

    pub const fn new() -> Self {
        Self {
            ip: [0, 0, 0, 0],
            gateway: None,
            prefix_len: 0,
            dns: Vec::new(),
        }
    }
}

impl HttpConfiguration {
    pub const fn new() -> Self {
        Self {
            ssid: heapless::String::new(),
            password: heapless::String::new(),
            use_static_ip: false,
            static_ip_config: None,
        }
    }
}

impl From<&HttpStaticIpConfig> for StaticIpConfig {
    fn from(http_static_ip_config: &HttpStaticIpConfig) -> Self {
        Self {
            ip: u32::from_be_bytes(http_static_ip_config.ip),
            gateway: http_static_ip_config.gateway.map(u32::from_be_bytes),
            prefix_len: http_static_ip_config.prefix_len,
            dns_servers: http_static_ip_config
                .dns
                .iter()
                .map(|&dns| u32::from_be_bytes(dns))
                .collect(),
        }
    }
}

impl From<HttpStaticIpConfig> for StaticIpConfig {
    fn from(http_static_ip_config: HttpStaticIpConfig) -> Self {
        Self::from(&http_static_ip_config)
    }
}

impl From<&StaticIpConfig> for HttpStaticIpConfig {
    fn from(static_ip_config: &StaticIpConfig) -> Self {
        Self {
            ip: static_ip_config.ip.to_be_bytes(),
            gateway: static_ip_config.gateway.map(|g| g.to_be_bytes()),
            prefix_len: static_ip_config.prefix_len,
            dns: static_ip_config
                .dns_servers
                .iter()
                .map(|dns| dns.to_be_bytes())
                .collect(),
        }
    }
}

impl From<StaticIpConfig> for HttpStaticIpConfig {
    fn from(static_ip_config: StaticIpConfig) -> Self {
        Self::from(&static_ip_config)
    }
}

impl From<&Settings> for HttpConfiguration {
    fn from(settings: &Settings) -> Self {
        Self {
            ssid: settings.wifi_ssid.clone(),
            password: settings.wifi_password.clone(),
            use_static_ip: settings.use_static_ip_config,
            static_ip_config: settings
                .static_ip_config
                .as_ref()
                .map(HttpStaticIpConfig::from),
        }
    }
}

impl From<Settings> for HttpConfiguration {
    fn from(settings: Settings) -> Self {
        Self {
            ssid: settings.wifi_ssid.clone(),
            password: settings.wifi_password.clone(),
            use_static_ip: settings.use_static_ip_config,
            static_ip_config: settings.static_ip_config.map(HttpStaticIpConfig::from),
        }
    }
}

impl From<&HttpConfiguration> for Settings {
    fn from(http_config: &HttpConfiguration) -> Self {
        Self {
            wifi_ssid: http_config.ssid.clone(),
            wifi_password: http_config.password.clone(),
            use_static_ip_config: http_config.use_static_ip,
            static_ip_config: http_config
                .static_ip_config
                .as_ref()
                .map(StaticIpConfig::from),
            settings_version: 1,
        }
    }
}

impl From<HttpConfiguration> for Settings {
    fn from(http_config: HttpConfiguration) -> Self {
        Self {
            wifi_ssid: http_config.ssid,
            wifi_password: http_config.password,
            use_static_ip_config: http_config.use_static_ip,
            static_ip_config: http_config.static_ip_config.map(StaticIpConfig::from),
            settings_version: 1,
        }
    }
}
