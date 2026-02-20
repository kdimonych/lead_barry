use crate::vcp_sensors::data_model::VcpReading;
use crate::vcp_sensors::error::VcpError;

#[derive(Debug, Copy, Clone)]
#[defmt_or_log::derive_format_or_debug]
pub enum VcpSensorsEvents {
    Reading(VcpReading),
    Error(VcpError),
}

impl VcpSensorsEvents {
    /// Returns the priority of the event. Higher values indicate higher priority.
    const fn priority(&self) -> u8 {
        match self {
            VcpSensorsEvents::Reading(_) => 0,
            VcpSensorsEvents::Error(_) => 1,
        }
    }
}

impl core::cmp::PartialEq for VcpSensorsEvents {
    fn eq(&self, other: &Self) -> bool {
        self.priority().eq(&other.priority())
    }
}

impl core::cmp::Eq for VcpSensorsEvents {}

impl core::cmp::PartialOrd for VcpSensorsEvents {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.priority().partial_cmp(&other.priority())
    }
}

impl core::cmp::Ord for VcpSensorsEvents {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.priority().cmp(&other.priority())
    }
}
