use crate::vcp_sensors::data_model::ChannelNum;

#[derive(Debug, Copy, Clone, defmt::Format)]
pub struct VcpLimits {
    pub min_voltage: f32,
    pub max_voltage: f32,
    pub min_current: f32,
    pub max_current: f32,
}

const DEFAULT_SHUNT_RESISTANCE: [f32; 3] = [0.1; 3]; // Ohms
const DEFAULT_MIN_VOLTAGE: f32 = 0.0; // Volts
const DEFAULT_MAX_VOLTAGE: f32 = 5.0; // Volts
const DEFAULT_MIN_CURRENT: f32 = 0.0; // Amps
const DEFAULT_MAX_CURRENT: f32 = 2.0; // Amps

#[derive(Debug)]
pub struct VcpConfig {
    pub limits: [VcpLimits; 3],
    shunt_resistance: &'static [f32; 3],
    pub enabled_channels: [bool; 3],
}

impl VcpLimits {
    pub const fn new(
        min_voltage: f32,
        max_voltage: f32,
        min_current: f32,
        max_current: f32,
    ) -> Self {
        Self {
            min_voltage,
            max_voltage,
            min_current,
            max_current,
        }
    }

    pub fn with_min_voltage(mut self, min_voltage: f32) -> Self {
        self.min_voltage = min_voltage;
        self
    }

    pub fn with_max_voltage(mut self, max_voltage: f32) -> Self {
        self.max_voltage = max_voltage;
        self
    }

    pub fn with_min_current(mut self, min_current: f32) -> Self {
        self.min_current = min_current;
        self
    }

    pub fn with_max_current(mut self, max_current: f32) -> Self {
        self.max_current = max_current;
        self
    }

    pub const fn const_default() -> Self {
        Self::new(
            DEFAULT_MIN_VOLTAGE,
            DEFAULT_MAX_VOLTAGE,
            DEFAULT_MIN_CURRENT,
            DEFAULT_MAX_CURRENT,
        )
    }
}

impl VcpConfig {
    pub const fn new(
        limits: [VcpLimits; 3],
        enabled_channels: [bool; 3],
        shunt_resistance: &'static [f32; 3],
    ) -> Self {
        if shunt_resistance[0] <= 0.0 {
            panic!("Shunt 0 resistance values must be positive and non-zero");
        }
        if shunt_resistance[1] <= 0.0 {
            panic!("Shunt 1 resistance values must be positive and non-zero");
        }
        if shunt_resistance[2] <= 0.0 {
            panic!("Shunt 2 resistance values must be positive and non-zero");
        }

        Self {
            limits,
            shunt_resistance,
            enabled_channels,
        }
    }

    pub fn with_limits(mut self, channel: ChannelNum, limits: VcpLimits) -> Self {
        self.limits[channel as usize] = limits;
        self
    }
    pub fn with_enabled(mut self, channel: ChannelNum, enabled: bool) -> Self {
        self.enabled_channels[channel as usize] = enabled;
        self
    }

    pub fn with_shunt_resistance(mut self, shunt_resistance: &'static [f32; 3]) -> Self {
        if shunt_resistance[0] <= 0.0 {
            panic!("Shunt 0 resistance values must be positive and non-zero");
        }
        if shunt_resistance[1] <= 0.0 {
            panic!("Shunt 1 resistance values must be positive and non-zero");
        }
        if shunt_resistance[2] <= 0.0 {
            panic!("Shunt 2 resistance values must be positive and non-zero");
        }

        self.shunt_resistance = shunt_resistance;
        self
    }

    /// Const version of default
    pub const fn const_default() -> Self {
        Self::new(
            [VcpLimits::const_default(); 3],
            [true; 3],
            &DEFAULT_SHUNT_RESISTANCE,
        )
    }

    pub fn shunt_resistance(&self, channel: ChannelNum) -> f32 {
        self.shunt_resistance[channel as usize]
    }

    pub fn shunt_resistances(&self) -> &'_ [f32; 3] {
        self.shunt_resistance
    }
}

impl Default for VcpConfig {
    fn default() -> Self {
        Self::const_default()
    }
}

impl Default for VcpLimits {
    fn default() -> Self {
        Self::const_default()
    }
}
