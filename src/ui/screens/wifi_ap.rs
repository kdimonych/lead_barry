use core::any::Any;

use common::any_string::AnyString;

use super::common::{ScStatus, TrStatus};

// struct InternalState
// {
//     heapless::String<64>
// }

pub struct ScvCredentials {
    pub ssid: heapless::String<32>,
    pub password: heapless::String<64>,
}

pub struct ScvClientInfo {
    pub ip: embassy_net::Ipv4Address,
    pub mac: [u8; 6],
}

pub enum ScWifiApData {
    NotReady,
    WaitingForClient(ScvCredentials),
    Connected(ScvClientInfo),
}

struct Internals {
    status_str: heapless::String<64>,
    detail_str: heapless::String<64>,
}

impl TrStatus for ScWifiApData {
    fn title<const SIZE: usize>(&self) -> AnyString<SIZE> {
        match self {
            ScWifiApData::NotReady => "WiFi AP".into(),
            ScWifiApData::WaitingForClient(_) => "WiFi AP Ready".into(),
            ScWifiApData::Connected(_) => "Client Connected".into(),
        }
    }

    fn status<const SIZE: usize>(&self) -> AnyString<SIZE> {
        match self {
            ScWifiApData::NotReady => "Initializing...".into(),
            ScWifiApData::WaitingForClient(credentials) => {
                let mut status_str = heapless::String::<SIZE>::new();
                core::fmt::write(
                    &mut status_str,
                    format_args!("SSID: {}", credentials.ssid.as_str()),
                )
                .ok();
                status_str.into()
            }
            ScWifiApData::Connected(client_info) => {
                let mut status_str = heapless::String::<SIZE>::new();
                core::fmt::write(&mut status_str, format_args!("IP: {}", client_info.ip)).ok();
                status_str.into()
            }
        }
    }
    fn detail<const SIZE: usize>(&self) -> Option<AnyString<SIZE>> {
        match self {
            ScWifiApData::NotReady => None,
            ScWifiApData::WaitingForClient(credentials) => {
                let mut status_str = heapless::String::<SIZE>::new();
                core::fmt::write(
                    &mut status_str,
                    format_args!("Psw: {}", credentials.password.as_str()),
                )
                .ok();
                Some(status_str.into())
            }
            ScWifiApData::Connected(client_info) => {
                let mut status_str = heapless::String::<SIZE>::new();
                let &mac = &client_info.mac;
                core::fmt::write(
                    &mut status_str,
                    format_args!(
                        "MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
                    ),
                )
                .ok();
                Some(status_str.into())
            }
        }
    }
}

pub type ScWifiAp = ScStatus<ScWifiApData>;
