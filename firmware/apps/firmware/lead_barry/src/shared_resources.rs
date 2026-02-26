use crate::global_types::I2c1Device;

use crate::configuration::ConfigurationStorage;
pub use crate::led_controller::LedController;

use crate::rtc::RtcDs3231Ref;
use crate::ui::UiControl;
use crate::vcp_sensors::VcpControl;

// Shared interfaces
pub struct SharedResources {
    pub ui_control: &'static UiControl<'static>,
    pub vcp_control: &'static VcpControl<'static>,
    pub rtc: &'static RtcDs3231Ref<I2c1Device<'static>>,
    pub configuration_storage: &'static ConfigurationStorage<'static>,

    pub led_controller: LedController,
}
