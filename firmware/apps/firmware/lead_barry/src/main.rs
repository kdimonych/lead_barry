//! Blinks the LED on a Pico W board
//!
//! This will blink the onboard LED on a Pico W, which is controlled via the CYW43 WiFi chip.
#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

mod async_infinite_stream;
mod async_stream;
mod configuration;
mod global_state;
mod global_types;
mod input;
mod led_controller;
mod main_logic_controller;
mod reset;
mod rtc;
mod shared_resources;
mod ui;
mod units;
mod vcp_sensors;
mod web_server;
mod wifi;

use defmt_or_log as log;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{Executor, Spawner};
use embassy_rp::peripherals::{DMA_CH0, I2C1, PIO0};
use embassy_rp::pio::InterruptHandler as PioInterruptHandler;
use embassy_rp::{
    bind_interrupts,
    i2c::{self, I2c, InterruptHandler as I2cInterruptHandler},
    multicore::Stack,
    peripherals::I2C0,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Duration;
use static_cell::StaticCell;

use crate::configuration::{ConfigurationStorageBuilder, Storage};

use crate::led_controller::{
    Led, LedAnimation, LedControllerBuilder, LedControllerRunner, PwmHardwareConfig, Repetitions,
};
use crate::rtc::RtcDs3231Ref;
use crate::units::FrequencyExt;
use global_types::*;
use input::*;
use main_logic_controller::*;
use shared_resources::*;
use ui::*;
use vcp_sensors::*;
use wifi::*;

// Configure panic behavior based on features
#[cfg(not(any(feature = "defmt", feature = "log")))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};
#[cfg(all(feature = "log", not(feature = "defmt")))]
use {panic_rtt_target as _, rtt_target as _};

// Constants
const CORE1_STACK_SIZE: usize = 4096 * 4;

// Interrupt handlers
bind_interrupts!(struct I2c0Irqs {
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
});

bind_interrupts!(struct I2c1Irqs {
    I2C1_IRQ => I2cInterruptHandler<I2C1>;
});

bind_interrupts!(struct Pio0Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

// Static resources
static BUTTON_CONTROLLER: StaticCell<ButtonControllerState> = StaticCell::new();
static CORE1_STACK: StaticCell<Stack<CORE1_STACK_SIZE>> = StaticCell::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static UI_SHARED_STATE: StaticCell<UiSharedState> = StaticCell::new();
static UI_CONTROL: StaticCell<UiControl> = StaticCell::new();
static VCP_SENSORS_STATE: StaticCell<VcpSensorsState<VCP_SENSORS_EVENT_QUEUE_SIZE>> = StaticCell::new();
static VCP_SENSORS_CONTROL: StaticCell<VcpControl> = StaticCell::new();
static I2C0_BUS: StaticCell<I2c0Bus> = StaticCell::new();
static I2C1_BUS: StaticCell<I2c1Bus> = StaticCell::new();
static SHARED_RESOURCES: StaticCell<SharedResources> = StaticCell::new();
static RTC_DS3231: StaticCell<RtcDs3231Ref<I2c1Device<'static>>> = StaticCell::new();

struct ResourcesCore0 {
    // Owned resources
    button_controller_builder: ButtonControllerBuilder,

    vcp_runner: Option<VcpSensorsRunner<'static>>,
    wifi_service_builder: WiFiServiceBuilder<PIO0, DMA_CH0>,
    led_controller_runner: LedControllerRunner,

    // Shared resources
    shared_resources: &'static SharedResources,
}

struct ResourcesCore1 {
    // Owned resources
    ui_runner: Option<UiRunner<'static>>,
    core1_stack_base: usize,
    core1_stack_end: usize,
}

fn log_system_frequencies() {
    let sys_freq = embassy_rp::clocks::clk_sys_freq();
    let peri_freq = embassy_rp::clocks::clk_peri_freq();
    let usb_freq = embassy_rp::clocks::clk_usb_freq();
    let adc_freq = embassy_rp::clocks::clk_adc_freq();
    let rtc_freq = embassy_rp::clocks::clk_rtc_freq();
    let xosc_freq = embassy_rp::clocks::xosc_freq();

    log::info!("=== System Clock Frequencies ===");
    log::info!("System Clock:     {} MHz", sys_freq / 1_000_000);
    log::info!("Peripheral Clock: {} MHz", peri_freq / 1_000_000);
    log::info!("USB Clock:        {} MHz", usb_freq / 1_000_000);
    log::info!("ADC Clock:        {} MHz", adc_freq / 1_000_000);
    log::info!("RTC Clock:        {} Hz", rtc_freq);
    log::info!("XOSC Clock:       {} MHz", xosc_freq / 1_000_000);
    log::info!("================================");
}

fn debug_memory_layout() {
    unsafe extern "C" {
        static _ram_start: u32;
        unsafe static _ram_end: u32;
        static _stack_start: u32;
        static _stack_end: u32;
        static _stack_size: u32;
    }

    let ram_start = { &raw const _ram_start as usize };
    let ram_end = { &raw const _ram_end as usize };
    let stack_start = { &raw const _stack_start as usize };
    let stack_end = { &raw const _stack_end as usize };
    let current_sp = cortex_m::register::msp::read() as usize;
    let stack_size = { &raw const _stack_size as usize };

    log::info!("=== Memory Layout ===");
    log::info!("RAM Start:    0x{:08x}", ram_start);
    log::info!("RAM End:      0x{:08x}", ram_end);
    log::info!("Stack Start:  0x{:08x}", stack_start);
    log::info!("Stack End:    0x{:08x}", stack_end);
    log::info!("Current SP:   0x{:08x}", current_sp);
    log::info!("Stack Size:   {} bytes", stack_size);
}

#[cortex_m_rt::entry]
fn main() -> ! {
    debug_memory_layout();
    let p: embassy_rp::Peripherals = embassy_rp::init(Default::default());

    // Bind led pins
    let config = PwmHardwareConfig {
        slice1: p.PWM_SLICE1,
        slice3: p.PWM_SLICE3,
        led_red: p.PIN_18,
        led_yellow: p.PIN_19,
        led_blue: p.PIN_22,
    };

    // Initialize the LED controller builder and build the controller and runner
    let led_controller_builder = LedControllerBuilder::new(config);
    let (led_controller, led_controller_runner) = led_controller_builder.build_once();

    // Set heartbeat animation on blue LED to verify it's working
    led_controller
        .try_set_animation(Led::Blue, LedAnimation::Sine(5000, Repetitions::Infinite))
        .unwrap();

    log_system_frequencies();

    // Bind button pins
    let mut button_controller_builder = ButtonControllerBuilder::new();
    button_controller_builder.bind_pin(Buttons::Yellow, p.PIN_2, embassy_rp::gpio::Pull::Up);
    button_controller_builder.bind_pin(Buttons::Blue, p.PIN_3, embassy_rp::gpio::Pull::Up);

    //User FLASH storage
    let storage = Storage::new(p.FLASH, p.DMA_CH1);
    let configuration_storage_builder = ConfigurationStorageBuilder::new(storage);
    let configuration_storage = configuration_storage_builder.build();

    // Setup I2C0 with standard frequency for sensors
    let mut i2c0_cfg = i2c::Config::default();
    i2c0_cfg.frequency = 1.mhz(); // Fast I2C clk for better performance
    let i2c0 = I2c::new_async(p.I2C0, p.PIN_5, p.PIN_4, I2c0Irqs, i2c0_cfg);
    let i2c0_bus: &'static Mutex<CriticalSectionRawMutex, I2c<'static, I2C0, i2c::Async>> =
        I2C0_BUS.init(Mutex::new(i2c0));

    // Setup I2C1 with standard frequency for sensors
    let mut i2c1_cfg = i2c::Config::default();
    i2c1_cfg.frequency = 400.khz(); // Fast I2C clk for better performance
    let i2c1 = I2c::new_async(p.I2C1, p.PIN_15, p.PIN_14, I2c1Irqs, i2c1_cfg);
    let i2c1_bus: &'static Mutex<CriticalSectionRawMutex, I2c<'static, I2C1, i2c::Async>> =
        I2C1_BUS.init(Mutex::new(i2c1));

    // Initialize the stack
    let core1_stack = CORE1_STACK.init_with(Stack::new);
    let core1_stack_base = core1_stack.mem.as_ptr() as usize;
    let core1_stack_end = core1_stack_base + core1_stack.mem.len();

    // Initialize the ui
    let ui_shared_state = UiSharedState::new();
    let state_ref = UI_SHARED_STATE.init(ui_shared_state);
    let (ui_control, ui_runner) = UiInterface::new(
        I2cDevice::new(i2c0_bus),
        ssd1306::size::DisplaySize128x64,
        state_ref,
        Some(ScWelcome::new().into()),
    );
    let ui_control: &'static UiControl = UI_CONTROL.init(ui_control);

    // Initialize the VCP sensors
    let vcp_state_ref = VCP_SENSORS_STATE.init_with(VcpSensorsState::new);
    let (vcp_runner, vcp_control) =
        VcpSensorsService::new(I2cDevice::new(i2c0_bus), vcp_state_ref, VcpConfig::default());
    let vcp_control: &'static VcpControl = VCP_SENSORS_CONTROL.init(vcp_control);

    let wifi_cfg: WiFiConfig<PIO0, DMA_CH0> = WiFiConfig::<PIO0, DMA_CH0> {
        pwr_pin: p.PIN_23, // Power pin, pin 23
        cs_pin: p.PIN_25,  // Chip select pin, pin 25
        dio_pin: p.PIN_24, // Data In/Out pin, pin 24
        clk_pin: p.PIN_29, // Clock pin, pin 29
        pio: p.PIO0,       // PIO instance
        dma_ch: p.DMA_CH0, // DMA channel
    };

    let wifi_service_builder = WiFiServiceBuilder::new(wifi_cfg, Pio0Irqs);

    // Initialize the RTC DS3231
    let rtc_ds3231 = rtc::create_rtc_ds3231(I2cDevice::new(i2c1_bus));
    let rtc_ds3231_ref: &'static RtcDs3231Ref<I2c1Device<'static>> = RTC_DS3231.init(rtc_ds3231);

    let shared_resources: &'static SharedResources = SHARED_RESOURCES.init(SharedResources {
        rtc: rtc_ds3231_ref,
        ui_control,
        vcp_control,
        configuration_storage,
        led_controller,
    });

    // Spawn core threads
    embassy_rp::multicore::spawn_core1(p.CORE1, core1_stack, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        log::debug!("Starting executor on core 1");
        executor1.run(|spawner| {
            spawner
                .spawn(core1_init(
                    spawner,
                    ResourcesCore1 {
                        ui_runner: Some(ui_runner),
                        core1_stack_base,
                        core1_stack_end,
                    },
                ))
                .unwrap();
        });
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(move |spawner| {
        log::debug!("Starting executor on core 0");
        spawner
            .spawn(core0_init(
                spawner,
                ResourcesCore0 {
                    button_controller_builder,
                    vcp_runner: Some(vcp_runner),
                    wifi_service_builder,
                    shared_resources,
                    led_controller_runner,
                },
            ))
            .unwrap();
    });
}

#[embassy_executor::task]
async fn core0_init(spawner: Spawner, resources: ResourcesCore0) -> ! {
    // Spawn stack monitor task
    spawner.spawn(core_0_stack_monitor_task()).unwrap();

    // Spawn the LED blink task on Core 0
    log::info!("Spawn LED task on core 0");
    // For regular GPIO LED (if you connect an external LED to a GPIO pin)

    spawner
        .spawn(led_controller_task(resources.led_controller_runner))
        .unwrap();

    // Spawn the VCP sensors task on Core 0
    if let Some(vcp_runner) = resources.vcp_runner {
        // Spawn the VCP sensors task on core 0
        log::info!("Spawn vcp sensors task on core 0");
        spawner.spawn(vcp_sensors_runner_task(vcp_runner)).unwrap();
    }

    // Initialize button controller
    let button_controller_state = BUTTON_CONTROLLER.init(ButtonControllerState::new());
    let (button_controller, button_controller_runner) =
        resources.button_controller_builder.build(button_controller_state);
    log::info!("Spawn buttons controller task on core 0");
    spawner
        .spawn(buttons_controller_task(button_controller_runner))
        .unwrap();

    log::info!("Create wifi service");
    let wifi_service = resources.wifi_service_builder.build(spawner, cyw43_task).await;

    //Call main logic controller
    main_logic_controller(spawner, resources.shared_resources, wifi_service, button_controller).await;
}

#[embassy_executor::task]
async fn buttons_controller_task(button_controller_runner: ButtonControllerRunner<'static>) -> ! {
    log::debug!("Starting buttons controller task...");
    button_controller_runner.run().await
}

#[embassy_executor::task]
async fn led_controller_task(led_controller_runner: LedControllerRunner) -> ! {
    log::debug!("Starting led controller task...");
    led_controller_runner.run().await;
}

#[embassy_executor::task]
async fn core1_init(spawner: Spawner, resources: ResourcesCore1) {
    // Spawn stack monitor task
    spawner
        .spawn(core_1_stack_monitor_task(
            resources.core1_stack_base,
            resources.core1_stack_end,
        ))
        .unwrap();

    // Spawn the UI task on Core 1
    if let Some(ui_runner) = resources.ui_runner {
        // Spawn the display task on Core 1
        log::debug!("Spawn display task on core 1");
        spawner.spawn(display_runner_task(ui_runner)).unwrap();
    }
}

#[embassy_executor::task]
async fn display_runner_task(mut ui_runner: UiRunner<'static>) -> ! {
    log::debug!("Starting display task...");
    ui_runner.run().await;
}

#[embassy_executor::task]
async fn vcp_sensors_runner_task(mut vcp_sensors_runner: VcpSensorsRunner<'static>) -> ! {
    log::debug!("Starting VCP sensors task...");
    vcp_sensors_runner.run().await
}

#[embassy_executor::task]
async fn cyw43_task(runner: WiFiDriverRunner<PIO0, DMA_CH0>) -> ! {
    log::debug!("Starting CYW43 driver task...");
    runner.run().await
}

fn get_core_0_stack_usage() -> (usize, usize) {
    unsafe extern "C" {
        static mut _stack_start: u32;
        static mut _stack_end: u32;
        static _stack_size: u32;
    }

    let stack_start = { &raw const _stack_start as *const _ as usize };

    let current_sp = cortex_m::register::msp::read() as usize;

    (stack_start - current_sp, &raw const _stack_size as usize)
}

#[embassy_executor::task]
async fn core_0_stack_monitor_task() -> ! {
    loop {
        let (stack_used, stack_size) = get_core_0_stack_usage();
        if stack_used > (stack_size as f32 * 0.8) as usize {
            log::warn!(
                "❗ [ATTENTION!] High stack usage at core 0: {} bytes of {} bytes",
                stack_used,
                stack_size
            );
        }

        // Your task work here
        embassy_time::Timer::after(Duration::from_millis(3000)).await;
    }
}

fn get_core_1_stack_usage(core1_stack_base: usize, core1_stack_end: usize) -> (usize, usize) {
    let current_sp = cortex_m::register::msp::read() as usize;
    log::debug_assert!(core1_stack_end > core1_stack_base);

    (core1_stack_end - current_sp, core1_stack_end - core1_stack_base)
}

#[embassy_executor::task]
async fn core_1_stack_monitor_task(core1_stack_base: usize, core1_stack_end: usize) -> ! {
    loop {
        let (stack_used, stack_size) = get_core_1_stack_usage(core1_stack_base, core1_stack_end);
        if stack_used > (stack_size as f32 * 0.8) as usize {
            log::warn!(
                "❗ [ATTENTION!] High stack usage at core 1: {} bytes of {} bytes",
                stack_used,
                stack_size
            );
        }

        // Your task work here
        embassy_time::Timer::after(Duration::from_millis(3000)).await;
    }
}
