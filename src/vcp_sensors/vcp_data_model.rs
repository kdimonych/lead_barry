#[derive(Debug, Copy, Clone, defmt::Format)]
pub enum VcpState {
    Normal(f32),
    Low(f32),
    High(f32),
    Critical(f32),
    Error,
}

pub type ChannelNum = u8;

#[derive(Debug, Copy, Clone, defmt::Format)]
pub struct VcpReading {
    pub voltage: VcpState,
    pub current: VcpState,
    pub channel: ChannelNum,
}

#[derive(Debug, Copy, Clone, defmt::Format)]
pub enum VcpSensorsEvents {
    Reading(VcpReading),
    Alert,
    Error,
}
