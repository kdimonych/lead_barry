mod network_settings;
mod static_ip_config;
mod wifi_ap_settings;
mod wifi_settings;

use core::str::FromStr;
use serde::{Deserialize, Serialize};

pub use network_settings::*;
pub use static_ip_config::*;
pub use wifi_ap_settings::*;
pub use wifi_settings::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, defmt::Format)]
#[non_exhaustive]
pub struct Settings {
    pub network_settings: NetworkSettings,
    pub settings_version: u32,
}

impl Settings {
    pub const fn new() -> Self {
        Self {
            network_settings: NetworkSettings::new(),
            settings_version: 1,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            network_settings: NetworkSettings::default(),
            settings_version: 1,
        }
    }
}
