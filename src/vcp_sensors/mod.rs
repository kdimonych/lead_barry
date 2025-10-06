mod vcp_data_model;
mod vcp_sensor_service;

pub use crate::vcp_sensors::vcp_data_model::{ChannelNum, VcpReading, VcpSensorsEvents};
pub use crate::vcp_sensors::vcp_sensor_service::{
    VcpConfig, VcpControl, VcpLimits, VcpMode, VcpSensorsRunner, VcpSensorsService, VcpSensorsState,
};
