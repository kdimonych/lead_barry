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

impl StaticIpConfig {
    pub const fn new() -> Self {
        Self {
            ip: Ipv4Address::UNSPECIFIED.to_bits(),
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

#[cfg(feature_use_static_ip_config)]
pub fn debug_static_ip_config() -> Option<StaticIpConfig> {
    defmt::info!("Use Static IP Config");
    defmt::info!("Static IP Address: {}", env!("DBG_STATIC_IP_ADDRESS"));
    defmt::info!("Static IP Gateway: {}", env!("DBG_STATIC_IP_GATEWAY"));
    defmt::info!(
        "Static IP Prefix Length: {}",
        env!("DBG_STATIC_IP_PREFIX_LEN")
    );
    defmt::info!("Static IP DNS 1: {}", env!("DBG_STATIC_IP_DNS_1"));
    defmt::info!("Static IP DNS 2: {}", env!("DBG_STATIC_IP_DNS_2"));
    defmt::info!("Static IP DNS 3: {}", env!("DBG_STATIC_IP_DNS_3"));

    Some(StaticIpConfig {
        ip: Ipv4Address::from_str(env!("DBG_STATIC_IP_ADDRESS"))
            .unwrap()
            .to_bits(),
        gateway: Some(
            Ipv4Address::from_str(env!("DBG_STATIC_IP_GATEWAY"))
                .unwrap()
                .to_bits(),
        ),
        prefix_len: env!("DBG_STATIC_IP_PREFIX_LEN").parse().unwrap_or(24),
        dns_servers: {
            let mut dns_vec: Vec<u32, 3> = Vec::new();
            if let Ok(dns1) = Ipv4Address::from_str(env!("DBG_STATIC_IP_DNS_1")) {
                dns_vec.push(dns1.to_bits()).ok();
            }
            if let Ok(dns2) = Ipv4Address::from_str(env!("DBG_STATIC_IP_DNS_2")) {
                dns_vec.push(dns2.to_bits()).ok();
            }
            if let Ok(dns3) = Ipv4Address::from_str(env!("DBG_STATIC_IP_DNS_3")) {
                dns_vec.push(dns3.to_bits()).ok();
            }
            dns_vec
        },
    })
}

#[cfg(not(feature_use_static_ip_config))]
pub fn debug_static_ip_config() -> Option<StaticIpConfig> {
    None
}
