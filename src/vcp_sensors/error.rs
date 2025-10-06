use defmt::*;

#[derive(Debug, Copy, Clone, defmt::Format)]
pub enum VcpError {
    I2cError(&'static str),
    InvalidChannel,
    SensorReadError,
}

impl VcpError {
    pub fn error_description(&self) -> Option<&'static str> {
        match self {
            VcpError::I2cError(msg) => Some(msg),
            VcpError::InvalidChannel => Some("Invalid channel number"),
            VcpError::SensorReadError => Some("Sensor read error"),
        }
    }
}
