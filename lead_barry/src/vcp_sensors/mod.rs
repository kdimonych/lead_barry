mod config;
mod data_model;
mod error;
mod events;
mod sensor_service;

pub use crate::global_types::I2c0Device;

pub use self::config::*;
pub use self::data_model::{ChannelNum, VcpReading};
pub use self::error::VcpError;
pub use self::events::VcpSensorsEvents;
pub use self::sensor_service::{VcpSensorsService, VcpSensorsState};
pub const VCP_SENSORS_EVENT_QUEUE_SIZE: usize = 8;

pub type VcpSensorsRunner<'a> =
    self::sensor_service::VcpSensorsRunner<'a, I2c0Device<'a>, VCP_SENSORS_EVENT_QUEUE_SIZE>;
pub type VcpControl<'a> = self::sensor_service::VcpControl<'a, VCP_SENSORS_EVENT_QUEUE_SIZE>;
