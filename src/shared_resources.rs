use crate::global_types::{I2c0Bus, I2c1Bus, I2c1Device};

use crate::configuration::ConfigurationStorage;
use crate::rtc::RtcDs3231Ref;
use crate::ui::UiControl;
use crate::vcp_sensors::VcpControl;

// Shared interfaces
pub struct SharedResources {
    pub i2c0_bus: &'static I2c0Bus,
    pub i2c1_bus: &'static I2c1Bus,
    pub ui_control: &'static UiControl<'static>,
    pub vcp_control: &'static VcpControl<'static>,
    pub rtc: &'static RtcDs3231Ref<I2c1Device<'static>>,
    pub configuration_storage: &'static ConfigurationStorage<'static>,
}
