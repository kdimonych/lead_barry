use core::u32;

use crate::configuration::*;
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
        let mut ip_config = StaticIpConfig::default();
        ip_config.ip = u32::from_be_bytes(http_static_ip_config.ip);
        ip_config.gateway = http_static_ip_config.gateway.map(u32::from_be_bytes);
        ip_config.prefix_len = http_static_ip_config.prefix_len;
        ip_config.dns_servers = http_static_ip_config
            .dns
            .iter()
            .map(|&dns| u32::from_be_bytes(dns))
            .collect();
        ip_config
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
            ssid: settings.network_settings.wifi_settings.ssid.clone(),
            password: settings.network_settings.wifi_settings.password.clone(),
            use_static_ip: settings.network_settings.use_static_ip_config,
            static_ip_config: settings
                .network_settings
                .static_ip_config
                .as_ref()
                .map(HttpStaticIpConfig::from),
        }
    }
}

impl From<Settings> for HttpConfiguration {
    fn from(settings: Settings) -> Self {
        Self {
            ssid: settings.network_settings.wifi_settings.ssid.clone(),
            password: settings.network_settings.wifi_settings.password.clone(),
            use_static_ip: settings.network_settings.use_static_ip_config,
            static_ip_config: settings
                .network_settings
                .static_ip_config
                .map(HttpStaticIpConfig::from),
        }
    }
}

impl From<&HttpConfiguration> for Settings {
    fn from(http_config: &HttpConfiguration) -> Self {
        let mut network_settings = NetworkSettings::default();
        network_settings.wifi_settings.ssid = http_config.ssid.clone();
        network_settings.wifi_settings.password = http_config.password.clone();
        network_settings.use_static_ip_config = http_config.use_static_ip;
        network_settings.static_ip_config = http_config
            .static_ip_config
            .as_ref()
            .map(StaticIpConfig::from);

        let mut settings = Settings::default();
        settings.network_settings = network_settings;
        settings
    }
}

impl From<HttpConfiguration> for Settings {
    fn from(http_config: HttpConfiguration) -> Self {
        let mut network_settings = NetworkSettings::default();
        network_settings.wifi_settings.ssid = http_config.ssid;
        network_settings.wifi_settings.password = http_config.password;
        network_settings.use_static_ip_config = http_config.use_static_ip;
        network_settings.static_ip_config = http_config.static_ip_config.map(StaticIpConfig::from);

        let mut settings = Settings::default();
        settings.network_settings = network_settings;
        settings
    }
}
