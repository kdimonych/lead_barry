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
use embassy_time::{Delay, Duration, Ticker};
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
        tx_boost: true,
        rx_boost: false,
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

    info!("Setting LoRa packet parameters...");
    set_led(RGB8::new(255, 192, 203)).await; // pink = starting
    let mut tx_pkt_params = {
        match lora.create_tx_packet_params(4, false, true, false, &mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                error!("Radio error = {}", err);
                set_led(RGB8::new(255, 0, 0)).await; // red = sleep
                return;
            }
        }
    };

    let buffer = [0x01u8, 0x02u8, 0x03u8];
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        info!("Prepare for transmitting LoRa P2P packet...");
        set_led(RGB8::new(0, 255, 255)).await; // cyan = starting
        match lora.prepare_for_tx(&mdltn_params, &mut tx_pkt_params, 2, &buffer).await {
            Ok(()) => {}
            Err(err) => {
                error!("Radio error = {}", err);
                set_led(RGB8::new(255, 0, 0)).await; // red = sleep
                return;
            }
        };

        info!("Transmitting LoRa P2P packet...");
        set_led(RGB8::new(255, 255, 255)).await; // white = transmitting
        match lora.tx().await {
            Ok(()) => {
                info!("TX DONE");
                set_led(RGB8::new(0, 128, 0)).await; // green = transmitting
            }
            Err(err) => {
                error!("Radio error = {}", err);
                set_led(RGB8::new(255, 0, 0)).await; // red = sleep
                return;
            }
        };

        info!("Sleeping radio...");
        match lora.sleep(false).await {
            Ok(()) => {
                info!("Sleep successful");
            }
            Err(err) => {
                error!("Sleep unsuccessful = {}", err);
                set_led(RGB8::new(255, 0, 0)).await; // red = sleep
            }
        }
        // Sleep for 1 seconds before the next transmission
        ticker.next().await;
    }
}
