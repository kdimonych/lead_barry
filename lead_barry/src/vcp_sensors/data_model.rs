#![allow(dead_code)]

#[derive(Debug, Copy, Clone)]
#[defmt_or_log::derive_format_or_debug]
pub enum VcpState {
    Normal(f32),
    Low(f32),
    High(f32),
}

pub type ChannelNum = u8;

#[derive(Debug, Copy, Clone)]
#[defmt_or_log::derive_format_or_debug]
pub struct VcpReading {
    pub voltage: VcpState,
    pub current: VcpState,
    pub channel: ChannelNum,
}

impl Into<f32> for VcpState {
    fn into(self) -> f32 {
        match self {
            VcpState::Normal(v) | VcpState::Low(v) | VcpState::High(v) => v,
        }
    }
}

impl VcpState {
    pub fn value(&self) -> f32 {
        match self {
            VcpState::Normal(v) | VcpState::Low(v) | VcpState::High(v) => *v,
        }
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, VcpState::Normal(_))
    }
    pub fn is_low(&self) -> bool {
        matches!(self, VcpState::Low(_))
    }
    pub fn is_high(&self) -> bool {
        matches!(self, VcpState::High(_))
    }
}
