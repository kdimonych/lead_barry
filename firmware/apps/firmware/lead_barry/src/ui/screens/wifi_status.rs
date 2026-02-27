#![allow(dead_code)]

use super::common::{DataModelStatus, DetailString, StatusString, SvStatusImpl, TitleString};

#[derive(Debug, Copy, Clone)]
#[defmt_or_log::derive_format_or_debug]
pub enum DmWifiStatusState {
    Disconnected,
    Connecting,
    Dhcp,
    Connected,
}

/// WiFi Network name
pub type DmWifiStatusNetworkName = heapless::String<32>;

/// Data model for WiFi status screen
/// Includes connection state and optionally the network name if connected or connecting
/// The network name is not available when disconnected, so it is wrapped in an Option
/// The screen will display the network name when connecting or connected, and no network name when disconnected
/// The detail string will show the network name if available, otherwise it will be empty
pub struct DmWifiStatus {
    wifi_network_name: Option<DmWifiStatusNetworkName>,
    wifi_state: DmWifiStatusState,
}

impl DmWifiStatus {
    pub const fn new(wifi_state: DmWifiStatusState, wifi_network_name: Option<DmWifiStatusNetworkName>) -> Self {
        Self {
            wifi_network_name,
            wifi_state,
        }
    }
}

impl DataModelStatus for DmWifiStatus {
    fn title<'b>(&'b self) -> TitleString<'b> {
        TitleString::from_str("WiFi Status")
    }

    fn status<'b>(&'b self) -> StatusString<'b> {
        match self.wifi_state {
            DmWifiStatusState::Disconnected => StatusString::from_str("Disconnected"),
            DmWifiStatusState::Connecting => StatusString::from_str("Connecting to:"),
            DmWifiStatusState::Dhcp => StatusString::from_str("Getting IP..."),
            DmWifiStatusState::Connected => StatusString::from_str("Connected to:"),
        }
    }
    fn detail<'b>(&'b self) -> Option<DetailString<'b>> {
        self.wifi_network_name
            .as_ref()
            .map(|name| DetailString::from_str_truncate(name.as_str()))
    }
}

/// The screen view for WiFi status, which uses the DmWifiStatus data model to display the current WiFi
/// connection status and network name if available.
///
/// The screen will show "WiFi Status" as the title, the connection status (Disconnected, Connecting to:,
/// Getting IP..., Connected to:) as the status, and the network name in the detail if available. If the network
/// name is not available (e.g. when disconnected), the detail will be empty.
///
/// This screen can be used to provide feedback to the user about the current WiFi connection status and which
/// network they are connecting to or connected to. It is a simple status screen that can be used in conjunction
/// with other screens like the WiFi AP screen to show the overall WiFi state of the device.
/// For example, when the device is trying to connect to a WiFi network, the screen will show "Connecting to:" as
/// the status and the network name in the detail. Once connected, it will show "Connected to:" and the network
/// name. If the connection is lost, it will show "Disconnected" and no network name. This provides a clear and
/// concise way for users to understand their WiFi connection status at a glance.
///
/// The screen view is implemented as a generic SvStatusImpl that takes the DmWifiStatus data model, allowing it
/// to leverage the common status screen layout and styling while providing specific data for the WiFi status context.
/// The SvStatusImpl will handle the drawing of the title, status, and detail on the screen based on the data
/// provided by the DmWifiStatus data model, following the conventions established in the common status screen implementation.
/// Overall, this screen provides a user-friendly way to display WiFi connection status and network information
/// using the established patterns for screen views and data models in the UI framework.
///
pub type SvWifiStatus = SvStatusImpl<DmWifiStatus>;
