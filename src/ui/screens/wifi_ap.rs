use super::common::{DetailString, ScStatusImpl, StatusString, TitleString, TrStatus};

pub struct ScvCredentials {
    pub ssid: heapless::String<32>,
    pub password: heapless::String<64>,
}

pub struct ScvClientInfo {
    pub ip: embassy_net::Ipv4Address,
    pub mac: Option<[u8; 6]>,
}

pub enum ScWifiApData {
    NotReady,
    LinkUp,
    ConfigUp,
    WaitingForClient(ScvCredentials),
    Connected(ScvClientInfo),
}

impl TrStatus for ScWifiApData {
    fn title(&'_ self) -> TitleString {
        match self {
            ScWifiApData::NotReady => TitleString::from_str("WiFi AP"),
            ScWifiApData::LinkUp => TitleString::from_str("WiFi AP Init"),
            ScWifiApData::ConfigUp => TitleString::from_str("WiFi AP Init"),
            ScWifiApData::WaitingForClient(_) => TitleString::from_str("WiFi AP Ready"),
            ScWifiApData::Connected(_) => TitleString::from_str("New Client"),
        }
    }

    fn status(&'_ self) -> StatusString {
        match self {
            ScWifiApData::NotReady => StatusString::from_str("Initializing..."),
            ScWifiApData::LinkUp => StatusString::from_str("AP Link Up..."),
            ScWifiApData::ConfigUp => StatusString::from_str("AP Config Up..."),
            ScWifiApData::WaitingForClient(credentials) => {
                let mut status_str = StatusString::complimentary_str();
                core::fmt::write(
                    &mut status_str,
                    format_args!("SSID: {}", credentials.ssid.as_str()),
                )
                .ok();
                status_str.into()
            }
            ScWifiApData::Connected(client_info) => {
                let mut status_str = StatusString::complimentary_str();
                core::fmt::write(&mut status_str, format_args!("IP: {}", client_info.ip)).ok();
                status_str.into()
            }
        }
    }
    fn detail(&'_ self) -> Option<DetailString> {
        match self {
            ScWifiApData::NotReady => None,
            ScWifiApData::LinkUp => None,
            ScWifiApData::ConfigUp => None,
            ScWifiApData::WaitingForClient(credentials) => {
                let mut status_str = DetailString::complimentary_str();
                core::fmt::write(
                    &mut status_str,
                    format_args!("Psw: {}", credentials.password.as_str()),
                )
                .ok();
                Some(status_str.into())
            }
            ScWifiApData::Connected(client_info) => {
                if let Some(mac) = client_info.mac {
                    let mut status_str = DetailString::complimentary_str();
                    core::fmt::write(
                        &mut status_str,
                        format_args!(
                            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
                        ),
                    )
                    .ok();
                    Some(status_str.into())
                } else {
                    None
                }
            }
        }
    }
}

pub type ScWifiAp = ScStatusImpl<ScWifiApData>;
