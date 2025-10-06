use crate::vcp_sensors::data_model::ChannelNum;

#[derive(Debug, Copy, Clone, defmt::Format)]
pub struct VcpLimits {
    pub min_voltage: f32,
    pub max_voltage: f32,
    pub min_current: f32,
    pub max_current: f32,
}

#[derive(Debug)]
pub struct VcpConfig {
    pub limits: [VcpLimits; 3],
    shunt_resistance: [f32; 3],
    pub enabled_channels: [bool; 3],
}

impl Default for VcpLimits {
    fn default() -> Self {
        Self {
            min_voltage: 0.0,
            max_voltage: 5.0,
            min_current: 0.0,
            max_current: 2.0,
        }
    }
}

macro_rules! positive_f32 {
    ($value:expr) => {{
        const _: () = core::assert!($value > 0.0, "Value must be positive");
        $value
    }};
}

impl VcpConfig {
    pub fn shunt_resistance(&self, channel: ChannelNum) -> f32 {
        self.shunt_resistance[channel as usize]
    }
    pub fn set_shunt_resistance(
        &mut self,
        channel: ChannelNum,
        resistance: f32,
    ) -> Result<(), &'static str> {
        if resistance <= 0.0 {
            return Err("Shunt resistance must be positive");
        }
        self.shunt_resistance[channel as usize] = resistance;
        Ok(())
    }
}

impl Default for VcpConfig {
    fn default() -> Self {
        Self {
            limits: [VcpLimits::default(); 3],
            shunt_resistance: [positive_f32!(0.1); 3],
            enabled_channels: [true; 3],
        }
    }
}
