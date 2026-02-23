use core::f32::consts::PI;
use libm::sinf;

pub struct SineAnimation {
    angular_freq: f32,
    animation_period: u32,
    n: u32,
    mid: u16,
    infinite: bool,
}

impl SineAnimation {
    pub fn new(animation_period: u32, magnitude: u16, frequency_multiplier: u8, infinite: bool) -> Self {
        let angular_freq = 2.0 * PI * frequency_multiplier as f32 / animation_period as f32;
        let mid = magnitude / 2;

        Self {
            angular_freq,
            animation_period,
            mid,
            n: 0,
            infinite,
        }
    }
}

impl Iterator for SineAnimation {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n == self.animation_period {
            if self.infinite {
                self.n = 0;
            } else {
                return None;
            }
        }

        let time = self.n as f32;
        let sample = (sinf(self.angular_freq * time - PI / 2.0) * self.mid as f32) as i32 + self.mid as i32;
        self.n += 1;
        Some(sample as u16)
    }
}
