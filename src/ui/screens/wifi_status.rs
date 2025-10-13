use super::common::{DetailString, ScStatusImpl, StatusString, TitleString, TrStatus};

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
    fn title(&'_ self) -> TitleString {
        TitleString::from_str("WiFi Status")
    }

    fn status(&'_ self) -> StatusString {
        match self.wifi_state {
            ScvState::Disconnected => StatusString::from_str("Disconnected"),
            ScvState::Connecting => StatusString::from_str("Connecting to:"),
            ScvState::Dhcp => StatusString::from_str("Getting IP..."),
            ScvState::Connected => StatusString::from_str("Connected to:"),
        }
    }
    fn detail(&'_ self) -> Option<DetailString> {
        self.wifi_network_name
            .as_ref()
            .map(|name| DetailString::from_str_truncate(name.as_str()))
    }
}

pub type ScWifiStats = ScStatusImpl<ScWifiStatsData>;
