#[derive(Debug, Copy, Clone, defmt::Format)]
pub enum VcpState {
    Normal(f32),
    Low(f32),
    High(f32),
}

pub type ChannelNum = u8;

#[derive(Debug, Copy, Clone, defmt::Format)]
pub struct VcpReading {
    pub voltage: VcpState,
    pub current: VcpState,
    pub channel: ChannelNum,
}

impl VcpState {
    pub fn value(&self) -> f32 {
        match self {
            VcpState::Normal(v) | VcpState::Low(v) | VcpState::High(v) => *v,
        }
    }
}
