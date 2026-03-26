use super::color::Color;
use embassy_rp::pio_programs::ws2812::PioWs2812;

use embassy_rp::pio::Instance;

pub type Led = usize;

pub struct Ws2812LedDriver<'d, P: Instance, const S: usize, const N: usize> {
    pub leds: [Color; N],
    ws2812: PioWs2812<'d, P, S, N>,
}

#[allow(dead_code)]
impl<'d, P: Instance, const S: usize, const N: usize> Ws2812LedDriver<'d, P, S, N> {
    pub fn new(ws2812: PioWs2812<'d, P, S, N>) -> Self {
        Self {
            ws2812,
            leds: [Color::default(); N],
        }
    }

    #[inline]
    pub fn set_color(&mut self, led: Led, color: Color) {
        self.leds[led as usize] = color;
    }

    #[inline]
    pub fn led_off(&mut self, led: Led) {
        self.leds[led as usize] = Color::default();
    }

    #[inline]
    pub fn led_color(&self, led: Led) -> Color {
        self.leds[led as usize]
    }

    #[inline]
    pub fn led_on(&mut self, led: Led, color: Color) {
        self.leds[led as usize] = color;
    }

    pub fn all_on(&mut self, color: Color) {
        for led in self.leds.iter_mut() {
            *led = color;
        }
    }

    pub fn all_off(&mut self) {
        for led in self.leds.iter_mut() {
            *led = Color::default();
        }
    }

    pub async fn flush(&mut self) {
        self.ws2812.write(&self.leds).await;
    }
}
