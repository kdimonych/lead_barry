use core::marker::PhantomData;

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
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Level, Output},
    interrupt::typelevel::Binding,
    pio::{InterruptHandler, Pio},
};

use super::config::*;

pub trait WiFiState {}

pub struct IdleState;
impl WiFiState for IdleState {}

pub struct JoinedState;
impl WiFiState for JoinedState {}

pub struct ApState;
impl WiFiState for ApState {}

pub struct WiFiController<'a, State>
where
    State: WiFiState,
{
    control: Control<'a>,
    _marker: core::marker::PhantomData<State>,
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

pub enum WiFiCtrlState<'a> {
    Uninitialized,
    Idle(WiFiController<'a, IdleState>),
    Joined(WiFiController<'a, JoinedState>),
    Ap(WiFiController<'a, ApState>),
}

impl<'a> WiFiCtrlState<'a> {
    pub fn is_idle(&self) -> bool {
        matches!(self, WiFiCtrlState::Idle(_))
    }

    pub fn is_joined(&self) -> bool {
        matches!(self, WiFiCtrlState::Joined(_))
    }

    pub fn is_ap(&self) -> bool {
        matches!(self, WiFiCtrlState::Ap(_))
    }
    pub fn is_uninitialized(&self) -> bool {
        matches!(self, WiFiCtrlState::Uninitialized)
    }

    pub fn change<Modifier>(&mut self, modifier: Modifier)
    where
        Modifier: FnOnce(WiFiCtrlState<'a>) -> WiFiCtrlState<'a>,
    {
        let old_state = core::mem::replace(self, WiFiCtrlState::Uninitialized);
        let new_state = modifier(old_state);
        *self = new_state;
    }

    pub async fn change_async<Modifier>(&mut self, modifier: Modifier)
    where
        Modifier: AsyncFnOnce(WiFiCtrlState<'a>) -> WiFiCtrlState<'a>,
    {
        let old_state = core::mem::replace(self, WiFiCtrlState::Uninitialized);
        let new_state = modifier(old_state).await;
        *self = new_state;
    }
}

pub struct NoWiFiBuilderCreated;
pub struct WiFiBuilderCreated<PIO, DMA>
where
    DMA: embassy_rp::dma::Channel + 'static,
    PIO: embassy_rp::pio::Instance + 'static,
{
    pio_spi: PioSpi<'static, PIO, 0, DMA>,
    pwr: Output<'static>,
}

pub struct WiFiDriverBuilder<Step = NoWiFiBuilderCreated> {
    step: Step,
}

impl WiFiDriverBuilder<NoWiFiBuilderCreated> {
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
    pub fn new<PIO, DMA>(
        wifi_cfg: WiFiConfig<PIO, DMA>,
        irq: impl Binding<PIO::Interrupt, InterruptHandler<PIO>>,
    ) -> WiFiDriverBuilder<WiFiBuilderCreated<PIO, DMA>>
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
        let pwr: Output<'_> = Output::new(wifi_cfg.pwr_pin, Level::Low);

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

        WiFiDriverBuilder {
            step: WiFiBuilderCreated { pio_spi: spi, pwr },
        }
    }
}

impl<PIO, DMA> WiFiDriverBuilder<WiFiBuilderCreated<PIO, DMA>>
where
    // Bounds from impl:
    DMA: embassy_rp::dma::Channel + 'static,
    PIO: embassy_rp::pio::Instance + 'static,
{
    /// Initialize the WiFi hardware and transition to Idle state
    pub async fn build<SpawnTokenBuilder, S>(
        self,
        wifi_static_state: &'static mut WiFiStaticData,
        spawner: Spawner,
        wifi_runner_task: SpawnTokenBuilder,
    ) -> (WiFiController<'static, IdleState>, NetDriver<'static>)
    where
        SpawnTokenBuilder: Fn(
            cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO, 0, DMA>>,
        ) -> ::embassy_executor::SpawnToken<S>,
    {
        let fw = CYW43_43439A0; // Firmware binary included in the cyw43_firmware crate;

        let state = &mut wifi_static_state.cyw43_state;
        debug!("Creating WiFi driver...");
        let (net_device, mut control, cyw43_runner) =
            cyw43::new(state, self.step.pwr, self.step.pio_spi, fw).await;
        debug!("WiFi driver created.");

        // Spawn the CYW43 runner task. Spawning this task here guarantees the WiFi driver operates correctly.
        spawner.spawn(wifi_runner_task(cyw43_runner)).unwrap();

        // Initialize the WiFi hardware with CLM data
        debug!("Initializing WiFi driver...");
        let clm = CYW43_43439A0_CLM; // CLM binary included in the cyw43_firmware crate;
        control.init(clm).await;
        control
            .set_power_management(cyw43::PowerManagementMode::Performance)
            .await;
        debug!("WiFi driver initialized.");

        (
            WiFiController {
                control,
                _marker: PhantomData,
            },
            net_device,
        )
    }
}

impl<'a> WiFiController<'a, IdleState> {
    /// Initialize the WiFi hardware and transition to Joined state
    pub async fn join(
        mut self,
        ssid: &str,
        join_options: JoinOptions<'_>,
    ) -> Result<WiFiController<'a, JoinedState>, (Self, Error)> {
        if let Err(error) = self.control.join(ssid, join_options).await {
            Err((self, error))
        } else {
            Ok(WiFiController {
                control: self.control,
                _marker: PhantomData,
            })
        }
    }

    /// Initialize the WiFi hardware and transition to AP state
    pub async fn start_ap_open(mut self, ssid: &str, channel: u8) -> WiFiController<'a, ApState> {
        self.control.start_ap_open(ssid, channel).await;
        WiFiController {
            control: self.control,
            _marker: PhantomData,
        }
    }

    /// Initialize the WiFi hardware and transition to AP state with WPA2
    pub async fn start_ap_wpa2(
        mut self,
        ssid: &str,
        password: &str,
        channel: u8,
    ) -> WiFiController<'a, ApState> {
        self.control.start_ap_wpa2(ssid, password, channel).await;
        WiFiController {
            control: self.control,
            _marker: PhantomData,
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

impl<'a> From<WiFiController<'a, IdleState>> for WiFiCtrlState<'a> {
    fn from(controller: WiFiController<'a, IdleState>) -> Self {
        Self::Idle(controller)
    }
}

impl<'a> WiFiController<'a, JoinedState> {
    /// Disconnect from the current WiFi network and transition to Idle state
    pub async fn leave(mut self) -> WiFiController<'a, IdleState> {
        self.control.leave().await;
        WiFiController {
            control: self.control,
            _marker: PhantomData,
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

impl<'a> From<WiFiController<'a, JoinedState>> for WiFiCtrlState<'a> {
    fn from(controller: WiFiController<'a, JoinedState>) -> Self {
        Self::Joined(controller)
    }
}

impl<'a> WiFiController<'a, ApState> {
    /// Close the access point and transition to Idle state
    pub async fn close_ap(mut self) -> WiFiController<'a, IdleState> {
        self.control.close_ap().await;
        WiFiController {
            control: self.control,
            _marker: PhantomData,
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

impl<'a> From<WiFiController<'a, ApState>> for WiFiCtrlState<'a> {
    fn from(controller: WiFiController<'a, ApState>) -> Self {
        Self::Ap(controller)
    }
}
