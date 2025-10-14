use cyw43::{Control, NetDriver};
use cyw43_firmware::{CYW43_43439A0, CYW43_43439A0_CLM};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};

// Re-export cyw43 types for convenience
pub use cyw43::AddMulticastAddressError;
pub use cyw43::ControlError as Error;
pub use cyw43::JoinAuth;
pub use cyw43::JoinOptions;
pub use cyw43::PowerManagementMode;
pub use cyw43::ScanOptions;
pub use cyw43::Scanner;

use defmt::debug;
use embassy_rp::{
    Peri,
    gpio::{Level, Output},
    interrupt::typelevel::Binding,
    peripherals::PIN_23,
    pio::{InterruptHandler, Pio},
};

use crate::wifi::config::*;

pub enum WiFiState {
    Uninitialized,
    Idle,
    Joined,
    Ap,
}

// WiFi service states
pub struct WiFiDriverCreatedState<PIO, DMA>
where
    DMA: embassy_rp::dma::Channel + 'static,
    PIO: embassy_rp::pio::Instance + 'static,
{
    pio_spi: PioSpi<'static, PIO, 0, DMA>,
    pwr_pin: Peri<'static, PIN_23>, // Power pin, pin 23 (will be used in following states)
}

pub struct InitializedState<'a> {
    control: Control<'a>,
}

pub struct IdleState<'a> {
    control: Control<'a>,
}

pub struct JoinedState<'a> {
    control: Control<'a>,
}

pub struct ApState<'a> {
    control: Control<'a>,
    // You can add fields if needed
}

pub struct WiFiStaticData {
    cyw43_state: cyw43::State,
}

impl WiFiStaticData {
    pub const fn new() -> Self {
        Self {
            cyw43_state: cyw43::State::new(),
        }
    }
}

impl Default for WiFiStaticData {
    fn default() -> Self {
        Self::new()
    }
}

pub enum WiFiController<'a> {
    Idle(IdleState<'a>),
    Joined(JoinedState<'a>),
    Ap(ApState<'a>),
}

/// Create a new WiFi service instance
/// 'static lifetime is required for the peripherals and state
/// PIO and DMA types are generic to allow for different instances
/// The irq parameter is used to bind the PIO interrupt
/// You must bind appropriate PIO interrupts in your main.rs, for example for PIO0:
/// ```rust,ignore
/// bind_interrupts!(struct Irqs {
///     PIO0_IRQ_0 => InterruptHandler<PIO0>;
/// });
/// ```
pub fn new_wifi_service<PIO, DMA>(
    wifi_cfg: WiFiConfig<PIO, DMA>,
    irq: impl Binding<PIO::Interrupt, InterruptHandler<PIO>>,
) -> WiFiDriverCreatedState<PIO, DMA>
where
    // Bounds from impl:
    DMA: embassy_rp::dma::Channel + 'static,
    PIO: embassy_rp::pio::Instance + 'static,
{
    // let fw = CYW43_43439A0; // Firmware binary included in the cyw43_firmware crate;
    // let clm = CYW43_43439A0_CLM; // CLM binary included in the cyw43_firmware crate;

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    // let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    // let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    //let pwr = Output::new(wifi_cfg.pwr_pin, Level::Low);
    let cs = Output::new(wifi_cfg.cs_pin, Level::High);
    let mut pio = Pio::new(wifi_cfg.pio, irq);

    let spi: PioSpi<'_, PIO, 0, DMA> = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        wifi_cfg.dio_pin,
        wifi_cfg.clk_pin,
        wifi_cfg.dma_ch,
    );

    WiFiDriverCreatedState::<PIO, DMA> {
        pio_spi: spi,
        pwr_pin: wifi_cfg.pwr_pin,
    }
}

impl<PIO, DMA> WiFiDriverCreatedState<PIO, DMA>
where
    DMA: embassy_rp::dma::Channel + 'static,
    PIO: embassy_rp::pio::Instance + 'static,
{
    /// Initialize the WiFi hardware and transition to Idle state
    pub async fn initialize(
        self,
        wifi_static_state: &'_ mut WiFiStaticData,
    ) -> (
        InitializedState<'_>,
        cyw43::Runner<'_, Output<'_>, PioSpi<'_, PIO, 0, DMA>>,
        NetDriver<'_>,
    ) {
        let fw = CYW43_43439A0; // Firmware binary included in the cyw43_firmware crate;

        let pwr = Output::new(self.pwr_pin, Level::Low);

        let state = &mut wifi_static_state.cyw43_state;
        debug!("Creating WiFi driver...");
        let (net_device, control, cyw43_runner) = cyw43::new(state, pwr, self.pio_spi, fw).await;

        debug!("WiFi driver created.");
        (InitializedState { control }, cyw43_runner, net_device)
    }
}

impl<'a> InitializedState<'a> {
    /// Initialize the WiFi hardware and transition to Idle state
    pub async fn initialize_controller(mut self) -> IdleState<'a> {
        let clm = CYW43_43439A0_CLM; // CLM binary included in the cyw43_firmware crate;
        self.control.init(clm).await;
        self.control
            .set_power_management(cyw43::PowerManagementMode::Performance)
            .await;
        IdleState {
            control: self.control,
        }
    }
}

impl<'a> IdleState<'a> {
    /// Initialize the WiFi hardware and transition to Joined state
    pub async fn join(
        mut self,
        ssid: &str,
        join_options: JoinOptions<'_>,
    ) -> Result<JoinedState<'a>, (Self, Error)> {
        if let Err(error) = self.control.join(ssid, join_options).await {
            Err((self, error))
        } else {
            Ok(JoinedState {
                control: self.control,
            })
        }
    }

    /// Initialize the WiFi hardware and transition to AP state
    pub async fn start_ap_open(mut self, ssid: &str, channel: u8) -> ApState<'a> {
        self.control.start_ap_open(ssid, channel).await;
        ApState {
            control: self.control,
        }
    }

    /// Initialize the WiFi hardware and transition to AP state with WPA2
    pub async fn start_ap_wpa2(mut self, ssid: &str, password: &str, channel: u8) -> ApState<'a> {
        self.control.start_ap_wpa2(ssid, password, channel).await;
        ApState {
            control: self.control,
        }
    }

    pub async fn led(&mut self, gpio_en: bool) {
        self.control.gpio_set(0, gpio_en).await;
    }

    pub async fn address(&mut self) -> [u8; 6] {
        self.control.address().await
    }

    pub async fn set_power_management(&mut self, mode: PowerManagementMode) {
        self.control.set_power_management(mode).await;
    }

    pub async fn add_multicast_address(
        &mut self,
        address: [u8; 6],
    ) -> Result<usize, AddMulticastAddressError> {
        self.control.add_multicast_address(address).await
    }

    pub async fn list_multicast_addresses(&mut self, result: &mut [[u8; 6]; 10]) -> usize {
        self.control.list_multicast_addresses(result).await
    }
    pub async fn scan(&mut self, scan_opts: ScanOptions) -> Scanner<'_> {
        self.control.scan(scan_opts).await
    }
}

impl<'a> JoinedState<'a> {
    /// Disconnect from the current WiFi network and transition to Idle state
    pub async fn leave(mut self) -> IdleState<'a> {
        self.control.leave().await;
        IdleState {
            control: self.control,
        }
    }

    pub async fn led(&mut self, gpio_en: bool) {
        self.control.gpio_set(0, gpio_en).await;
    }

    pub async fn address(&mut self) -> [u8; 6] {
        self.control.address().await
    }

    pub async fn set_power_management(&mut self, mode: PowerManagementMode) {
        self.control.set_power_management(mode).await;
    }

    pub async fn add_multicast_address(
        &mut self,
        address: [u8; 6],
    ) -> Result<usize, AddMulticastAddressError> {
        self.control.add_multicast_address(address).await
    }

    pub async fn list_multicast_addresses(&mut self, result: &mut [[u8; 6]; 10]) -> usize {
        self.control.list_multicast_addresses(result).await
    }
    pub async fn scan(&mut self, scan_opts: ScanOptions) -> Scanner<'_> {
        self.control.scan(scan_opts).await
    }
}

impl<'a> ApState<'a> {
    /// Close the access point and transition to Idle state
    pub async fn close_ap(mut self) -> IdleState<'a> {
        self.control.close_ap().await;
        IdleState {
            control: self.control,
        }
    }

    pub async fn led(&mut self, gpio_en: bool) {
        self.control.gpio_set(0, gpio_en).await;
    }

    pub async fn address(&mut self) -> [u8; 6] {
        self.control.address().await
    }

    pub async fn set_power_management(&mut self, mode: PowerManagementMode) {
        self.control.set_power_management(mode).await;
    }

    pub async fn add_multicast_address(
        &mut self,
        address: [u8; 6],
    ) -> Result<usize, AddMulticastAddressError> {
        self.control.add_multicast_address(address).await
    }

    pub async fn list_multicast_addresses(&mut self, result: &mut [[u8; 6]; 10]) -> usize {
        self.control.list_multicast_addresses(result).await
    }

    pub async fn scan(&mut self, scan_opts: ScanOptions) -> Scanner<'_> {
        self.control.scan(scan_opts).await
    }
}
