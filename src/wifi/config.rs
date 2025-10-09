use embassy_rp::Peri;
use embassy_rp::dma::Channel;
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::Instance;

pub struct WiFiConfig<PIO, DMA>
where
    // Bounds from impl:
    DMA: Channel + 'static,
    PIO: Instance + 'static,
{
    pub pwr_pin: Peri<'static, PIN_23>, // Power pin, pin 23
    pub cs_pin: Peri<'static, PIN_25>,  // Chip select pin, pin 25
    pub dio_pin: Peri<'static, PIN_24>, // Data In/Out pin, pin 24
    pub clk_pin: Peri<'static, PIN_29>, // Clock pin, pin 29
    pub pio: Peri<'static, PIO>,        // PIO instance
    pub dma_ch: Peri<'static, DMA>,     // DMA channel
}
