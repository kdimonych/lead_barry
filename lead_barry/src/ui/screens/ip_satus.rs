use super::common::{DetailString, ScStatusImpl, StatusString, TrStatus};

pub use super::common::TitleString as IpTitleString;

pub enum ScvState {
    Disconnected,
    Connecting,
    Dhcp,
    Connected,
}

pub enum ScvIpState {
    GettingIp,
    IpAssigned,
}

pub struct ScIpData {
    pub state: ScvIpState,
    pub ip: embassy_net::Ipv4Address,
    pub mac: Option<[u8; 6]>,
}

impl TrStatus for ScIpData {
    fn title(&'_ self) -> IpTitleString<'_> {
        match self.state {
            ScvIpState::GettingIp => IpTitleString::from_str("Getting IP..."),
            ScvIpState::IpAssigned => IpTitleString::from_str("IP Assigned"),
        }
    }

    fn status(&'_ self) -> StatusString<'_> {
        match self.state {
            ScvIpState::GettingIp => StatusString::from_str("DHCP handshake..."),
            ScvIpState::IpAssigned => {
                let mut status_str = StatusString::complimentary_str();
                core::fmt::write(&mut status_str, format_args!("IP: {}", self.ip)).ok();
                status_str.into()
            }
        }
    }
    fn detail(&'_ self) -> Option<DetailString<'_>> {
        match self.state {
            ScvIpState::GettingIp => None,
            ScvIpState::IpAssigned => {
                if let Some(mac) = self.mac {
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

pub type ScIpStatus = ScStatusImpl<ScIpData>;
