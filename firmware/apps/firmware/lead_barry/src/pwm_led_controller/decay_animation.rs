use core::f32::consts::PI;
use libm::cosf;

pub struct DecayAnimation {
    angular_freq: f32,
    animation_period: u32,
    n: u32,
    magnitude: u16,
}

impl DecayAnimation {
    pub fn new(mut animation_period: u32, magnitude: u16) -> Self {
        animation_period = core::cmp::max(animation_period, 2); // Minimum 2 samples to avoid division by zero and ensure at least one update
        let angular_freq = PI / (animation_period - 1) as f32 / 2.0;
        Self {
            angular_freq,
            animation_period,
            magnitude,
            n: 0,
        }
    }
}

impl Iterator for DecayAnimation {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n == self.animation_period {
            return None;
        }

        let time = self.n as f32;
        let sample = (cosf(self.angular_freq * time) * self.magnitude as f32) as u16;
        self.n += 1;
        Some(sample)
    }
}
