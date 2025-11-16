use embassy_net::{Ipv4Address, Ipv4Cidr};
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

impl From<&StaticIpConfig> for embassy_net::StaticConfigV4 {
    fn from(static_ip_config: &StaticIpConfig) -> Self {
        Self {
            address: Ipv4Cidr::new(
                Ipv4Address::from_bits(static_ip_config.ip),
                static_ip_config.prefix_len,
            ),
            dns_servers: static_ip_config
                .dns_servers
                .iter()
                .map(|dns_ip_bits| Ipv4Address::from_bits(*dns_ip_bits))
                .collect(),
            gateway: static_ip_config.gateway.map(Ipv4Address::from_bits),
        }
    }
}

impl From<StaticIpConfig> for embassy_net::StaticConfigV4 {
    fn from(static_ip_config: StaticIpConfig) -> Self {
        embassy_net::StaticConfigV4::from(&static_ip_config)
    }
}

// #[cfg(env = "USE_STATIC_IP_CONFIG=true")]
// fn debug_static_ip_config() -> Option<StaticIpConfig> {
//     defmt::info!("Use Static IP Config");
//     defmt::info!("Static IP Address: {}", env!("STATIC_IP_ADDRESS"));
//     defmt::info!("Static IP Gateway: {}", env!("STATIC_IP_GATEWAY"));
//     defmt::info!("Static IP Prefix Length: {}", env!("STATIC_IP_PREFIX_LEN"));
//     defmt::info!("Static IP DNS 1: {}", env!("STATIC_IP_DNS_1"));
//     defmt::info!("Static IP DNS 2: {}", env!("STATIC_IP_DNS_2"));
//     defmt::info!("Static IP DNS 3: {}", env!("STATIC_IP_DNS_3"));

//     Some(StaticIpConfig {
//         ip: Ipv4Addr::from_str(env!("STATIC_IP_ADDRESS"))
//             .unwrap()
//             .to_bits(),
//         gateway: Some(
//             Ipv4Addr::from_str(env!("STATIC_IP_GATEWAY"))
//                 .unwrap()
//                 .to_bits(),
//         ),
//         prefix_len: env!("STATIC_IP_PREFIX_LEN").parse().unwrap_or(24),
//         dns_servers: {
//             let mut dns_vec: Vec<u32, 3> = Vec::new();
//             if let Ok(dns1) = Ipv4Addr::from_str(env!("STATIC_IP_DNS_1")) {
//                 dns_vec.push(dns1.to_bits()).ok();
//             }
//             if let Ok(dns2) = Ipv4Addr::from_str(env!("STATIC_IP_DNS_2")) {
//                 dns_vec.push(dns2.to_bits()).ok();
//             }
//             if let Ok(dns3) = Ipv4Addr::from_str(env!("STATIC_IP_DNS_3")) {
//                 dns_vec.push(dns3.to_bits()).ok();
//             }
//             dns_vec
//         },
//     })
// }

// #[cfg(env = "USE_STATIC_IP_CONFIG=false")]
// fn debug_static_ip_config() -> Option<StaticIpConfig> {
//     None
// }

// pub fn debug_settings() -> Settings {
//     defmt::info!("Current Settings:");
//     defmt::info!("WiFi SSID: {}", env!("WIFI_SSID"));
//     defmt::info!("WiFi Password: ********");
//     defmt::info!("Use Static IP Config: {}", env!("USE_STATIC_IP"));
//     defmt::info!("Static IP Address: {}", env!("STATIC_IP_ADDRESS"));
//     defmt::info!("Static IP Gateway: {}", env!("STATIC_IP_GATEWAY"));
//     defmt::info!("Static IP Prefix Length: {}", env!("STATIC_IP_PREFIX_LEN"));
//     defmt::info!("Static IP DNS 1: {}", env!("STATIC_IP_DNS_1"));
//     defmt::info!("Static IP DNS 2: {}", env!("STATIC_IP_DNS_2"));
//     defmt::info!("Static IP DNS 3: {}", env!("STATIC_IP_DNS_3"));

//     Settings {
//         wifi_ssid: heapless::String::from(env!("WIFI_SSID")),
//         wifi_password: heapless::String::from(env!("WIFI_PASSWORD")),

//         use_static_ip_config: env!("USE_STATIC_IP") == "true",
//         static_ip_config: Some(StaticIpConfig {
//             ip: Ipv4Addr::from_str(env!("STATIC_IP_ADDRESS"))
//                 .unwrap()
//                 .to_bits(),
//             gateway: Some(
//                 Ipv4Addr::from_str(env!("STATIC_IP_GATEWAY"))
//                     .unwrap()
//                     .to_bits(),
//             ),
//             prefix_len: env!("STATIC_IP_PREFIX_LEN").parse().unwrap_or(24),
//             dns_servers: {
//                 let mut dns_vec: Vec<u32, 3> = Vec::new();
//                 if let Ok(dns1) = Ipv4Addr::from_str(env!("STATIC_IP_DNS_1")) {
//                     dns_vec.push(dns1.to_bits()).ok();
//                 }
//                 if let Ok(dns2) = Ipv4Addr::from_str(env!("STATIC_IP_DNS_2")) {
//                     dns_vec.push(dns2.to_bits()).ok();
//                 }
//                 if let Ok(dns3) = Ipv4Addr::from_str(env!("STATIC_IP_DNS_3")) {
//                     dns_vec.push(dns3.to_bits()).ok();
//                 }
//                 dns_vec
//             },
//         }),
//         settings_version: 1,
//     }
// }
