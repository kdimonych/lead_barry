//! This example runs on the Raspberry Pi Pico with a Waveshare board containing a Semtech Sx1262 radio.
//! It demonstrates LORA P2P send functionality.

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::{
    interrupt::typelevel::Binding,
    spi::{Config, Spi},
};
use embassy_time::{Delay, Duration, Ticker, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;

use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};

use lora_phy::LoRa;
use lora_phy::iv::GenericSx127xInterfaceVariant;
use lora_phy::sx127x::{Sx127x, Sx1272};
use lora_phy::{mod_params::*, sx127x};
use {defmt_rtt as _, panic_probe as _};

use smart_leds::{RGB8, SmartLedsWrite};

const LORA_FREQUENCY_IN_HZ: u32 = 433_900_000; // warning: set this appropriately for the region

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    const NUM_LEDS: usize = 1;
    let mut data = [RGB8::default(); NUM_LEDS];
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let program = PioWs2812Program::new(&mut common);
    let mut ws2812 = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &program);

    let mut set_led = async move |color: RGB8| {
        data[0] = color;
        ws2812.write(&data).await;
    };

    set_led(RGB8::new(255, 0, 0)).await; // red = starting

    info!("Starting LoRa P2P send example");

    let nss = Output::new(p.PIN_3, Level::High);
    let reset = Output::new(p.PIN_15, Level::High);
    let dio1 = Input::new(p.PIN_4, Pull::None);

    let mut spi_config = Config::default();
    spi_config.frequency = 100_000;

    let spi = Spi::new(p.SPI1, p.PIN_10, p.PIN_11, p.PIN_12, p.DMA_CH1, p.DMA_CH2, spi_config);
    let spi = ExclusiveDevice::new(spi, nss, Delay).unwrap();

    // IMPORTANT: The TCXO configuration must match your board's hardware.
    // If your board does not have a TCXO (Temperature-Compensated Crystal Oscillator),
    // set `tcxo_ctrl` to `None`. An incorrect setting will cause transmission
    // functions (e.g., `lora.tx()`) to hang indefinitely.
    let config = sx127x::Config {
        chip: Sx1272,
        tcxo_used: true,
        tx_boost: false,
        rx_boost: true,
    };
    info!("Initializing LoRa radio...");
    set_led(RGB8::new(255, 165, 0)).await; // orange = starting
    let iv = GenericSx127xInterfaceVariant::new(reset, dio1, None, None).unwrap();
    let mut lora = LoRa::new(Sx127x::new(spi, iv, config), true, Delay).await.unwrap();

    info!("Configuring LoRa modulation parameters...");
    set_led(RGB8::new(255, 255, 0)).await; // yellow = starting
    let mdltn_params = {
        match lora.create_modulation_params(
            SpreadingFactor::_11,
            Bandwidth::_250KHz,
            CodingRate::_4_8,
            LORA_FREQUENCY_IN_HZ,
        ) {
            Ok(mp) => mp,
            Err(err) => {
                error!("Radio error = {}", err);
                set_led(RGB8::new(255, 0, 0)).await; // red = sleep
                return;
            }
        }
    };

    let mut receiving_buffer = [00u8; 100];

    info!("Setting LoRa packet parameters...");
    set_led(RGB8::new(255, 192, 203)).await; // pink = starting
    let mut rx_pkt_params = {
        match lora.create_rx_packet_params(4, false, receiving_buffer.len() as u8, true, false, &mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                error!("Radio error = {}", err);
                set_led(RGB8::new(255, 0, 0)).await; // red = sleep
                return;
            }
        }
    };

    set_led(RGB8::new(200, 192, 203)).await; // magenta = starting

    loop {
        match lora
            .prepare_for_rx(RxMode::Continuous, &mdltn_params, &rx_pkt_params)
            .await
        {
            Ok(()) => {}
            Err(err) => {
                info!("Radio error = {}", err);
                set_led(RGB8::new(255, 0, 0)).await; // red = sleep
                Timer::after_millis(1000).await;
                return;
            }
        };

        receiving_buffer = [00u8; 100];

        match lora.rx(&rx_pkt_params, &mut receiving_buffer).await {
            Ok((received_len, _rx_pkt_status)) => {
                if (received_len == 3)
                    && (receiving_buffer[0] == 0x01u8)
                    && (receiving_buffer[1] == 0x02u8)
                    && (receiving_buffer[2] == 0x03u8)
                {
                    info!("rx successful");
                    set_led(RGB8::new(0, 255, 0)).await; // green = successful packet
                    Timer::after_millis(200).await;
                } else {
                    info!("rx unknown packet");
                    set_led(RGB8::new(255, 0, 0)).await; // red = unknown packet
                    Timer::after_millis(1000).await;
                }
            }
            Err(err) => {
                info!("rx unsuccessful = {}", err);
                for _ in 0..3 {
                    set_led(RGB8::new(255, 165, 0)).await; // orange
                    Timer::after_millis(100).await;
                    set_led(RGB8::new(0, 0, 0)).await; // off
                    Timer::after_millis(100).await;
                }
                Timer::after_millis(800).await;
            }
        }
        set_led(RGB8::new(0, 0, 0)).await; // off
    }
}
