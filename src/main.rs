//! Blinks the LED on a Pico W board
//!
//! This will blink the onboard LED on a Pico W, which is controlled via the CYW43 WiFi chip.
#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

mod configuration;
mod flash_storage;
mod input;
mod main_logic_controller;
mod matrix_ops;
mod precise_timing;
mod reset;
mod ui;
mod units;
mod vcp_sensors;
mod web_server;
mod wifi;

use cyw43_pio::PioSpi;
use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{Executor, Spawner};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::InterruptHandler as PioInterruptHandler;
use embassy_rp::{
    Peri, bind_interrupts,
    gpio::{Level, Output},
    i2c::{self, I2c, InterruptHandler as I2cInterruptHandler},
    multicore::Stack,
    peripherals::{I2C0, PIN_22},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Ticker, Timer};

use static_cell::StaticCell;

use crate::configuration::{ConfigurationStorage, ConfigurationStorageBuilder};
use crate::units::{FrequencyExt, TimeExt};
use flash_storage::*;
use input::*;
use main_logic_controller::*;
use micromath::F32Ext;
use ui::*;
use vcp_sensors::*;
use wifi::*;

// Display driver imports
use {defmt_rtt as _, panic_probe as _};

// Constants
const CORE1_STACK_SIZE: usize = 4096 * 4;
const VCP_SENSORS_EVENT_QUEUE_SIZE: usize = 8;

// Global types
type I2cBus = Mutex<CriticalSectionRawMutex, I2c<'static, I2C0, i2c::Async>>;
type I2cDeviceType<'a> = I2cDevice<'a, CriticalSectionRawMutex, I2c<'a, I2C0, i2c::Async>>;
type UiRunnerType<'a> =
    UiRunner<'a, I2cDeviceType<'a>, ssd1306::size::DisplaySize128x64, ScCollection>;
type UiControlType<'a> = UiControl<'a, ScCollection>;
type VcpSensorsRunnerType<'a> =
    VcpSensorsRunner<'a, I2cDeviceType<'a>, VCP_SENSORS_EVENT_QUEUE_SIZE>;

// Interrupt handlers
bind_interrupts!(struct I2c0Irqs {
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
});

bind_interrupts!(struct Pio0Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

// Shared interfaces

struct SharedResources {
    i2c_bus: &'static I2cBus,
    ui_control: &'static UiControlType<'static>,
    vcp_control: &'static VcpControlType<'static>,
    configuration_storage: &'static ConfigurationStorage<'static>,
}

// Static resources
static BUTTON_CONTROLLER: StaticCell<ButtonControllerState> = StaticCell::new();
static CORE1_STACK: StaticCell<Stack<CORE1_STACK_SIZE>> = StaticCell::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static UI_SHARED_STATE: StaticCell<UiSharedState<ScCollection>> = StaticCell::new();
static UI_CONTROL: StaticCell<UiControlType> = StaticCell::new();
static VCP_SENSORS_STATE: StaticCell<VcpSensorsState<VCP_SENSORS_EVENT_QUEUE_SIZE>> =
    StaticCell::new();
static VCP_SENSORS_CONTROL: StaticCell<VcpControlType> = StaticCell::new();
static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
static LED_PIN: StaticCell<Output> = StaticCell::new();
static SHARED_RESOURCES: StaticCell<SharedResources> = StaticCell::new();
static WIFI_STATIC_DATA: StaticCell<WiFiStaticData> = StaticCell::new();

// Global data models
// Voltage reading model
type VoltageReading = DataModel<f32>;
static VOLTAGE_READING_MODEL: StaticCell<VoltageReading> = StaticCell::new();

struct ResourcesCore0 {
    // Owned resources
    button_controller_builder: ButtonControllerBuilder,
    voltage_reading: &'static VoltageReading,
    led_pin: Peri<'static, PIN_22>,

    vcp_runner: Option<VcpSensorsRunnerType<'static>>,
    wifi_builder: WiFiDriverBuilder<WiFiBuilderCreated<PIO0, DMA_CH0>>,

    // Shared resources
    shared_resources: &'static SharedResources,
}

struct ResourcesCore1 {
    // Owned resources
    ui_runner: Option<UiRunnerType<'static>>,
    core1_stack_base: usize,
    core1_stack_end: usize,

    // Shared resources
    shared_resources: &'static SharedResources,
}

fn log_system_frequencies() {
    let sys_freq = embassy_rp::clocks::clk_sys_freq();
    let peri_freq = embassy_rp::clocks::clk_peri_freq();
    let usb_freq = embassy_rp::clocks::clk_usb_freq();
    let adc_freq = embassy_rp::clocks::clk_adc_freq();
    let rtc_freq = embassy_rp::clocks::clk_rtc_freq();
    let xosc_freq = embassy_rp::clocks::xosc_freq();

    info!("=== System Clock Frequencies ===");
    info!("System Clock:     {} MHz", sys_freq / 1_000_000);
    info!("Peripheral Clock: {} MHz", peri_freq / 1_000_000);
    info!("USB Clock:        {} MHz", usb_freq / 1_000_000);
    info!("ADC Clock:        {} MHz", adc_freq / 1_000_000);
    info!("RTC Clock:        {} Hz", rtc_freq);
    info!("XOSC Clock:       {} MHz", xosc_freq / 1_000_000);
    info!("================================");
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

    info!("=== Memory Layout ===");
    info!("RAM Start:    0x{:08x}", ram_start);
    info!("RAM End:      0x{:08x}", ram_end);
    info!("Stack Start:  0x{:08x}", stack_start);
    info!("Stack End:    0x{:08x}", stack_end);
    info!("Current SP:   0x{:08x}", current_sp);
    info!("Stack Size:   {} bytes", stack_size);
}

#[cortex_m_rt::entry]
fn main() -> ! {
    debug_memory_layout();

    let p = embassy_rp::init(Default::default());

    log_system_frequencies();

    // Bind button pins
    let mut button_controller_builder = ButtonControllerBuilder::new();
    button_controller_builder.bind_pin(Buttons::Yellow, p.PIN_2, embassy_rp::gpio::Pull::Up);
    button_controller_builder.bind_pin(Buttons::Blue, p.PIN_3, embassy_rp::gpio::Pull::Up);

    //User FLASH storage
    let storage = Storage::new(p.FLASH, p.DMA_CH1);
    let configuration_storage_builder = ConfigurationStorageBuilder::new(storage);
    let configuration_storage = configuration_storage_builder.build();

    // Setup I2C with standard frequency for sensors
    let mut i2c_cfg = i2c::Config::default();
    i2c_cfg.frequency = 1.mhz(); // Fast I2C for better performance
    let i2c = I2c::new_async(p.I2C0, p.PIN_5, p.PIN_4, I2c0Irqs, i2c_cfg);

    let i2c_bus: &'static Mutex<CriticalSectionRawMutex, I2c<'static, I2C0, i2c::Async>> =
        I2C_BUS.init(Mutex::new(i2c));

    // Initialize the stack
    let core1_stack = CORE1_STACK.init_with(Stack::new);
    let core1_stack_base = core1_stack.mem.as_ptr() as usize;
    let core1_stack_end = core1_stack_base + core1_stack.mem.len();

    // Initialize global data models
    let voltage_reading: &'static VoltageReading = VOLTAGE_READING_MODEL.init(DataModel::new(0.0));

    // Initialize the ui
    let ui_shared_state = UiSharedState::new();
    let state_ref = UI_SHARED_STATE.init(ui_shared_state);
    let (ui_control, ui_runner) = UiInterface::new(
        I2cDevice::new(i2c_bus),
        ssd1306::size::DisplaySize128x64,
        state_ref,
        Some(ScWelcome::new().into()),
    );
    let ui_control: &'static UiControlType = UI_CONTROL.init(ui_control);

    // Initialize the VCP sensors
    let vcp_state_ref = VCP_SENSORS_STATE.init_with(VcpSensorsState::new);
    let (vcp_runner, vcp_control) =
        VcpSensorsService::new(I2cDevice::new(i2c_bus), vcp_state_ref, VcpConfig::default());
    let vcp_control: &'static VcpControlType = VCP_SENSORS_CONTROL.init(vcp_control);

    let wifi_cfg = WiFiConfig::<PIO0, DMA_CH0> {
        pwr_pin: p.PIN_23, // Power pin, pin 23
        cs_pin: p.PIN_25,  // Chip select pin, pin 25
        dio_pin: p.PIN_24, // Data In/Out pin, pin 24
        clk_pin: p.PIN_29, // Clock pin, pin 29
        pio: p.PIO0,       // PIO instance
        dma_ch: p.DMA_CH0, // DMA channel
    };

    let wifi_builder = WiFiDriverBuilder::new(wifi_cfg, Pio0Irqs);

    // wifi_config
    //     .wifi_network
    //     .push_str(env!("WIFI_SSID"))
    //     .unwrap();
    // wifi_config
    //     .wifi_password
    //     .push_str(env!("WIFI_PASSWORD"))
    //     .unwrap();

    let shared_resources: &'static SharedResources = SHARED_RESOURCES.init(SharedResources {
        i2c_bus,
        ui_control,
        vcp_control,
        configuration_storage,
    });

    // Spawn core threads
    embassy_rp::multicore::spawn_core1(p.CORE1, core1_stack, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        debug!("Starting executor on core 1");
        executor1.run(|spawner| {
            spawner
                .spawn(core1_init(
                    spawner,
                    ResourcesCore1 {
                        ui_runner: Some(ui_runner),
                        core1_stack_base,
                        core1_stack_end,
                        shared_resources,
                    },
                ))
                .unwrap();
        });
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(move |spawner| {
        debug!("Starting executor on core 0");
        spawner
            .spawn(core0_init(
                spawner,
                ResourcesCore0 {
                    button_controller_builder,
                    vcp_runner: Some(vcp_runner),
                    led_pin: p.PIN_22,
                    voltage_reading,
                    wifi_builder,
                    shared_resources,
                },
            ))
            .unwrap();
    });
}

#[embassy_executor::task]
async fn screen_iterate_task(
    ui_control: &'static UiControlType<'static>,
    vcp_control: &'static VcpControlType<'static>,
    voltage_reading: &'static VoltageReading,
) -> ! {
    debug!("Starting screen iteration task...");
    //let mut ticker = Ticker::every(100.ms());

    vcp_control.disable_channel(1).await;
    vcp_control.disable_channel(2).await;

    ui_control
        .switch(ScCollection::Vcp(ScVcp::new(
            voltage_reading,
            ScvBaseUnits::Volts,
        )))
        .await;

    loop {
        let event: VcpSensorsEvents = vcp_control.receive_event().await;
        match event {
            VcpSensorsEvents::Reading(reading) => {
                trace!("Reading: {}", reading);
                if reading.channel == 0 {
                    let mut voltage = voltage_reading.lock().await;
                    *voltage = reading.voltage.value();
                }
            }
            VcpSensorsEvents::Error(description) => {
                error!("VCP Event: Error: {}", description);
            }
        }
        //ticker.next().await;
    }
}

#[embassy_executor::task]
async fn core0_init(spawner: Spawner, resources: ResourcesCore0) -> ! {
    // Spawn stack monitor task
    spawner.spawn(core_0_stack_monitor_task()).unwrap();

    // Spawn the LED blink task on Core 0
    debug!("Spawn LED task on core 0");
    // For regular GPIO LED (if you connect an external LED to a GPIO pin)
    let led = LED_PIN.init(Output::new(resources.led_pin, Level::Low));
    spawner.spawn(led_task(led)).unwrap();

    // Spawn the VCP sensors task on Core 0
    if let Some(vcp_runner) = resources.vcp_runner {
        // Spawn the VCP sensors task on core 0
        debug!("Spawn vcp sensors task on core 0");
        spawner.spawn(vcp_sensors_runner_task(vcp_runner)).unwrap();
    }

    //Initialize wifi controller
    info!("Create wifi controller");
    let wifi_static_data = WIFI_STATIC_DATA.init(WiFiStaticData::new());
    let (wifi_controller, wifi_network_driver) = resources
        .wifi_builder
        .build(wifi_static_data, spawner, cyw43_task)
        .await;

    // Initialize button controller
    let button_controller_state = BUTTON_CONTROLLER.init(ButtonControllerState::new());
    let (button_controller, button_controller_runner) = resources
        .button_controller_builder
        .build(button_controller_state);
    debug!("Spawn buttons controller task on core 0");
    spawner
        .spawn(buttons_controller_task(button_controller_runner))
        .unwrap();

    //Call main logic controller
    main_logic_controller(
        spawner,
        resources.shared_resources.vcp_control,
        resources.shared_resources.ui_control,
        wifi_controller,
        wifi_network_driver,
        button_controller,
        resources.shared_resources.configuration_storage,
    )
    .await;
}

#[embassy_executor::task]
async fn buttons_controller_task(button_controller_runner: ButtonControllerRunner<'static>) -> ! {
    debug!("Starting buttons controller task...");
    button_controller_runner.run().await;
}

#[embassy_executor::task]
async fn led_task(led: &'static mut Output<'static>) -> ! {
    let mut ticker = Ticker::every(1500.ms());

    let mut led_state = false;

    loop {
        if led_state {
            led.set_low();
        } else {
            led.set_high();
        }
        led_state = !led_state;

        ticker.next().await;
    }
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
        debug!("Spawn display task on core 1");
        spawner.spawn(display_runner_task(ui_runner)).unwrap();
    }
}

#[embassy_executor::task]
async fn display_runner_task(mut ui_runner: UiRunnerType<'static>) -> ! {
    debug!("Starting display task...");
    ui_runner.run().await;
}

#[embassy_executor::task]
async fn vcp_sensors_runner_task(mut vcp_sensors_runner: VcpSensorsRunnerType<'static>) -> ! {
    debug!("Starting VCP sensors task...");
    vcp_sensors_runner.run().await;
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    debug!("Starting CYW43 driver task...");
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
            defmt::warn!(
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
    defmt::debug_assert!(core1_stack_end > core1_stack_base);

    (
        core1_stack_end - current_sp,
        core1_stack_end - core1_stack_base,
    )
}

#[embassy_executor::task]
async fn core_1_stack_monitor_task(core1_stack_base: usize, core1_stack_end: usize) -> ! {
    loop {
        let (stack_used, stack_size) = get_core_1_stack_usage(core1_stack_base, core1_stack_end);
        if stack_used > (stack_size as f32 * 0.8) as usize {
            defmt::warn!(
                "❗ [ATTENTION!] High stack usage at core 1: {} bytes of {} bytes",
                stack_used,
                stack_size
            );
        }

        // Your task work here
        embassy_time::Timer::after(Duration::from_millis(3000)).await;
    }
}

#[embassy_executor::task]
async fn precise_sensor_task() {
    use embassy_time::{Duration, Instant, Ticker};

    // Create a precise 100Hz ticker for sensor readings
    let mut sensor_ticker = Ticker::every(Duration::from_millis(10));
    let mut counter = 0u32;
    let mut max_jitter = Duration::from_micros(0);
    let mut last_time = Instant::now();

    info!("Starting precise sensor task at 100Hz");

    loop {
        sensor_ticker.next().await;

        let now = Instant::now();
        let elapsed = now.duration_since(last_time);
        let expected = Duration::from_millis(10);

        // Measure timing jitter
        let jitter = if elapsed > expected {
            elapsed - expected
        } else {
            expected - elapsed
        };

        if jitter > max_jitter {
            max_jitter = jitter;
        }

        counter += 1;

        // Simulate precise sensor work
        let work_start = Instant::now();
        let work_duration = Instant::now().duration_since(work_start);

        // Log performance every 1000 iterations (10 seconds)
        if counter.is_multiple_of(1000) {
            info!(
                "Sensor task: {} cycles, max jitter: {}μs, last work: {}μs",
                counter,
                max_jitter.as_micros(),
                work_duration.as_micros()
            );
            max_jitter = Duration::from_micros(0); // Reset max jitter
        }

        last_time = now;
    }
}

#[embassy_executor::task]
async fn matrix_operations_task() {
    use matrix_ops::*;

    info!("Starting matrix operations demonstration...");

    // Run the matrix operations demo
    demo_matrix_operations();

    // Continuous matrix operations for real-time applications
    let mut angle = 0.0f32;
    let mut angle_deg = 0i32;
    let mut filter = KalmanFilter::new(0.0, 1.0, 0.01, 0.1);

    loop {
        // Rotate a point around origin
        let rotation_matrix = MatrixOps::rotation_2d(angle);
        let point = Point2D::new(1.0, 0.0);
        let rotated = MatrixOps::transform_point_2d(&rotation_matrix, point);

        // Simulate sensor data with noise
        let simulated_sensor = (angle * 2.0).sin() + 0.1 * (angle * 10.0).sin();

        // Apply Kalman filtering
        filter.predict();
        filter.update(simulated_sensor);

        {
            let rotated_x = (rotated.x * 100.0) as i32; // Convert to fixed point for display
            let rotated_y = (rotated.y * 100.0) as i32;
            let sensor_raw = (simulated_sensor * 1000.0) as i32;
            let sensor_filtered = (filter.estimate() * 1000.0) as i32;

            info!(
                "Angle: {}°, Rotated point: ({}.{:02}, {}.{:02})",
                angle_deg,
                rotated_x / 100,
                rotated_x.abs() % 100,
                rotated_y / 100,
                rotated_y.abs() % 100
            );
            info!(
                "Raw sensor: {}.{:03}, Filtered: {}.{:03}",
                sensor_raw / 1000,
                sensor_raw.abs() % 1000,
                sensor_filtered / 1000,
                sensor_filtered.abs() % 1000
            );
        }

        angle += 0.1;
        if angle > core::f32::consts::TAU {
            // 2π
            angle = 0.0;
        }
        angle_deg = (angle * 180.0 / core::f32::consts::PI) as i32;

        Timer::after(Duration::from_millis(500)).await;
    }
}
