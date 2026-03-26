use super::color::Color;
use core::f32::consts::PI;
use libm::sinf;

pub struct SineAnimation {
    color: Color,
    angular_freq: f32,
    animation_period: u32,
    n: u32,
    infinite: bool,
}

impl SineAnimation {
    pub fn new(color: Color, animation_period: u32, frequency_multiplier: u8, infinite: bool) -> Self {
        let angular_freq = 2.0 * PI * frequency_multiplier as f32 / animation_period as f32;

        Self {
            color,
            angular_freq,
            animation_period,
            n: 0,
            infinite,
        }
    }
}

impl Iterator for SineAnimation {
    type Item = Color;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n == self.animation_period {
            if self.infinite {
                self.n = 0;
            } else {
                return None;
            }
        }

        let time = self.n as f32;
        let sample = sinf(self.angular_freq * time - PI / 2.0) * 0.5 + 0.5;
        self.n += 1;
        Some(Color {
            r: (self.color.r as f32 * sample) as u8,
            g: (self.color.g as f32 * sample) as u8,
            b: (self.color.b as f32 * sample) as u8,
        })
    }
}
