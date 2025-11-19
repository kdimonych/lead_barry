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
        let mut dns_servers: Vec<u32, 3> = Vec::new();

        if let Some(dns1) = option_env!("DBG_STATIC_IP_DNS_1")
            .map(|str| str.parse().unwrap_or(Ipv4Address::UNSPECIFIED))
            && dns1 != Ipv4Address::UNSPECIFIED
        {
            dns_servers.push(dns1.to_bits()).ok();
        }

        if let Some(dns2) = option_env!("DBG_STATIC_IP_DNS_2")
            .map(|str| str.parse().unwrap_or(Ipv4Address::UNSPECIFIED))
            && dns2 != Ipv4Address::UNSPECIFIED
        {
            dns_servers.push(dns2.to_bits()).ok();
        }

        if let Some(dns3) = option_env!("DBG_STATIC_IP_DNS_3")
            .map(|str| str.parse().unwrap_or(Ipv4Address::UNSPECIFIED))
            && dns3 != Ipv4Address::UNSPECIFIED
        {
            dns_servers.push(dns3.to_bits()).ok();
        }

        Self {
            ip: option_env!("DBG_STATIC_IP_ADDRESS")
                .map(|str| str.parse().unwrap_or(Ipv4Address::UNSPECIFIED).to_bits())
                .unwrap_or(Ipv4Address::UNSPECIFIED.to_bits()),
            gateway: option_env!("DBG_STATIC_IP_ADDRESS")
                .map(|str| str.parse().unwrap_or(Ipv4Address::UNSPECIFIED).to_bits()),
            prefix_len: option_env!("DBG_STATIC_IP_PREFIX_LEN")
                .map(|str| str.parse().unwrap_or(24u8))
                .unwrap_or(0u8),
            dns_servers,
        }
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
