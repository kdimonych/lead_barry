use super::common::{DataModelStatus, DetailString, StatusString, SvStatusImpl, TitleString};

pub struct DmWifiApCredentials {
    pub ssid: heapless::String<32>,
    pub password: heapless::String<64>,
}

pub struct DmWifiApClientInfo {
    pub ip: embassy_net::Ipv4Address,
    pub mac: Option<[u8; 6]>,
}

pub enum DmWifiAp {
    NotReady,
    WaitingForClient(DmWifiApCredentials),
    Connected(DmWifiApClientInfo),
}

impl DataModelStatus for DmWifiAp {
    fn title<'b>(&'b self) -> TitleString<'b> {
        match self {
            DmWifiAp::NotReady => TitleString::from_str("WiFi AP"),
            DmWifiAp::WaitingForClient(_) => TitleString::from_str("WiFi AP Ready"),
            DmWifiAp::Connected(_) => TitleString::from_str("New Client"),
        }
    }

    fn status<'b>(&'b self) -> StatusString<'b> {
        match self {
            DmWifiAp::NotReady => StatusString::from_str("Initializing..."),
            DmWifiAp::WaitingForClient(credentials) => {
                let mut status_str = StatusString::complimentary_str();
                core::fmt::write(&mut status_str, format_args!("SSID: {}", credentials.ssid.as_str())).ok();
                status_str.into()
            }
            DmWifiAp::Connected(client_info) => {
                let mut status_str = StatusString::complimentary_str();
                core::fmt::write(&mut status_str, format_args!("IP: {}", client_info.ip)).ok();
                status_str.into()
            }
        }
    }
    fn detail<'b>(&'b self) -> Option<DetailString<'b>> {
        match self {
            DmWifiAp::NotReady => None,
            DmWifiAp::WaitingForClient(credentials) => {
                let mut status_str = DetailString::complimentary_str();
                core::fmt::write(&mut status_str, format_args!("Psw: {}", credentials.password.as_str())).ok();
                Some(status_str.into())
            }
            DmWifiAp::Connected(client_info) => {
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

pub type SvWifiAp = SvStatusImpl<DmWifiAp>;

impl From<DmWifiApCredentials> for SvWifiAp {
    fn from(value: DmWifiApCredentials) -> Self {
        let wifi_ap_data = DmWifiAp::WaitingForClient(value);
        SvStatusImpl::new(wifi_ap_data)
    }
}

impl From<DmWifiApClientInfo> for SvWifiAp {
    fn from(value: DmWifiApClientInfo) -> Self {
        let wifi_ap_data = DmWifiAp::Connected(value);
        SvStatusImpl::new(wifi_ap_data)
    }
}
