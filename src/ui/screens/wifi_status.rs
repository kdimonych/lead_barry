use common::any_string::AnyString;

use super::common::{ScStatus, TrStatus};

pub enum ScvState {
    Disconnected,
    Connecting,
    Dhcp,
    Connected,
}

pub struct ScWifiStatsData {
    wifi_network_name: Option<heapless::String<32>>,
    wifi_state: ScvState,
}

impl ScWifiStatsData {
    pub const fn new(
        wifi_state: ScvState,
        wifi_network_name: Option<heapless::String<32>>,
    ) -> Self {
        Self {
            wifi_network_name,
            wifi_state,
        }
    }
}

impl TrStatus for ScWifiStatsData {
    fn title<const SIZE: usize>(&self) -> AnyString<SIZE> {
        "WiFi Status".into()
    }
    fn status<const SIZE: usize>(&self) -> AnyString<SIZE> {
        match self.wifi_state {
            ScvState::Disconnected => "Disconnected".into(),
            ScvState::Connecting => "Connecting to:".into(),
            ScvState::Dhcp => "Getting IP...".into(),
            ScvState::Connected => "Connected to:".into(),
        }
    }
    fn detail<const SIZE: usize>(&self) -> Option<AnyString<SIZE>> {
        self.wifi_network_name
            .as_ref()
            .map(|name| name.as_str().into())
    }
}

pub type ScWifiStats = ScStatus<ScWifiStatsData>;
