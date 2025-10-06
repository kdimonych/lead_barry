//! Blinks the LED on a Pico W board
//!
//! This will blink the onboard LED on a Pico W, which is controlled via the CYW43 WiFi chip.
#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

mod matrix_ops;
mod precise_timing;
mod sync_examples;
mod ui;
mod units;
mod vcp_sensors;
mod wifi;

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{Executor, Spawner};
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

use crate::units::{FrequencyExt, TimeExt};
use micromath::F32Ext;
use ui::*;
use vcp_sensors::*;
use wifi::*;

// Display driver imports
use {defmt_rtt as _, panic_probe as _};

// Interrupt handlers
bind_interrupts!(struct Irqs {
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
});

// Shared interfaces
type I2cBus = Mutex<CriticalSectionRawMutex, I2c<'static, I2C0, i2c::Async>>;
pub type UiControlHandle<'a, ScreenSet> = Mutex<CriticalSectionRawMutex, UiControl<'a, ScreenSet>>;

struct SharedResources {
    i2c_bus: &'static I2cBus,
    ui_control: &'static mut UiControlHandle<'static, ScreenCollection>,
}

// Static resources
static CORE1_STACK: StaticCell<Stack<4096>> = StaticCell::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static UI_SHARED_STATE: StaticCell<UiSharedState<ScreenCollection>> = StaticCell::new();
static UI_CONTROL: StaticCell<UiControlHandle<ScreenCollection>> = StaticCell::new();
static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
static LED_PIN: StaticCell<Output> = StaticCell::new();
static SHARED_RESOURCES: StaticCell<SharedResources> = StaticCell::new();

// Global data models
// Voltage reading model
type VoltageReading = DataModel<f32>;
static VOLTAGE_READING_MODEL: StaticCell<VoltageReading> = StaticCell::new();

struct ResourcesCore0 {
    // Owned resources
    voltage_reading: &'static VoltageReading,
    wifi_config: WiFiSubsystemConfig,
    led_pin: Peri<'static, PIN_22>,

    // Shared resources
    shared_resources: &'static SharedResources,
}

type UiRunnerType<'a> = UiRunner<
    'a,
    I2cDevice<'a, CriticalSectionRawMutex, I2c<'a, I2C0, i2c::Async>>,
    ssd1306::size::DisplaySize128x64,
    ScreenCollection,
>;
struct ResourcesCore1 {
    // Owned resources
    ui_runner: Option<UiRunnerType<'static>>,

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

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    log_system_frequencies();

    // Setup I2C with standard frequency for sensors
    let mut i2c_cfg = i2c::Config::default();
    i2c_cfg.frequency = 1.mhz(); // Fast I2C for better performance
    let i2c = I2c::new_async(p.I2C0, p.PIN_5, p.PIN_4, Irqs, i2c_cfg);

    let i2c_bus: &'static Mutex<CriticalSectionRawMutex, I2c<'static, I2C0, i2c::Async>> =
        I2C_BUS.init(Mutex::new(i2c));

    // Initialize the stack
    let stack = CORE1_STACK.init(Stack::new());

    // Initialize global data models
    let voltage_reading: &'static VoltageReading = VOLTAGE_READING_MODEL.init(DataModel::new(0.0));

    // Initialize the ui
    let ui_shared_state = UiSharedState::new();
    let state_ref = UI_SHARED_STATE.init(ui_shared_state);
    let (ui_control, ui_runner) = UiInterface::new(
        I2cDevice::new(i2c_bus),
        ssd1306::size::DisplaySize128x64,
        state_ref,
        None,
    );
    let ui_control: &'static mut Mutex<CriticalSectionRawMutex, UiControl<'_, ScreenCollection>> =
        UI_CONTROL.init(Mutex::new(ui_control));

    // WiFi configuration
    let mut wifi_config = WiFiSubsystemConfig {
        pwr_pin: p.PIN_23, // Power pin, pin 23
        cs_pin: p.PIN_25,  // Chip select pin, pin 25
        dio_pin: p.PIN_24, // Data In/Out pin, pin 24
        clk_pin: p.PIN_29, // Clock pin, pin 29
        pio: p.PIO0,       // PIO instance
        dma_ch: p.DMA_CH0, // DMA channel
        wifi_network: heapless::String::new(),
        wifi_password: heapless::String::new(),
    };

    wifi_config
        .wifi_network
        .push_str(env!("WIFI_SSID"))
        .unwrap();
    wifi_config
        .wifi_password
        .push_str(env!("WIFI_PASSWORD"))
        .unwrap();

    let shared_resources: &'static SharedResources = SHARED_RESOURCES.init(SharedResources {
        i2c_bus,
        ui_control,
    });

    // Spawn core threads
    embassy_rp::multicore::spawn_core1(p.CORE1, stack, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        debug!("Starting executor on core 1");
        executor1.run(|spawner| {
            spawner
                .spawn(core1_init(
                    spawner,
                    ResourcesCore1 {
                        ui_runner: Some(ui_runner),
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
                    led_pin: p.PIN_22,
                    voltage_reading,
                    wifi_config,
                    shared_resources,
                },
            ))
            .unwrap();
    });
}

use ina3221_async::INA3221Async;
#[embassy_executor::task]
async fn ina3221_voltage_read_task(
    i2c_bus: &'static I2cBus,
    voltage_reading: &'static VoltageReading,
) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let ina = INA3221Async::new(i2c_dev, 0x40);

    let mut ticker = Ticker::every(40.ms());

    loop {
        match ina.get_bus_voltage(0).await {
            Err(e) => {
                error!("INA3221 read error: {:?}", e);
            }
            Ok(voltage) => {
                let mut v = voltage_reading.lock().await;
                *v = voltage.volts();
            }
        };
        ticker.next().await;
    }
}

#[embassy_executor::task]
async fn screen_iterate_task(
    ui_control: &'static UiControlHandle<'static, ScreenCollection>,
    voltage_reading: &'static VoltageReading,
) -> ! {
    debug!("Starting screen iteration task...");
    let mut ticker = Ticker::every(10.s());

    ui_control
        .lock()
        .await
        .switch_screen(ScreenCollection::VIP(VIPScreen::new(
            voltage_reading,
            BaseUnits::Volts,
        )))
        .await;

    loop {
        ticker.next().await;
    }
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
async fn core0_init(spawner: Spawner, resources: ResourcesCore0) {
    // Spawn the LED blink task on Core 0
    debug!("Spawn LED task on core 0");
    // For regular GPIO LED (if you connect an external LED to a GPIO pin)
    let led = LED_PIN.init(Output::new(resources.led_pin, Level::Low));
    spawner.spawn(led_task(led)).unwrap();

    // Spawn the screen iteration task on Core 0
    debug!("Spawn screen iteration task on core 0");
    spawner
        .spawn(screen_iterate_task(
            resources.shared_resources.ui_control,
            resources.voltage_reading,
        ))
        .unwrap();

    // Spawn the voltage reading simulation task on Core 0
    spawner
        .spawn(ina3221_voltage_read_task(
            resources.shared_resources.i2c_bus,
            resources.voltage_reading,
        ))
        .unwrap();

    // //Spawn wifi task
    // spawner
    //     .spawn(wifi_task(resources.spawner, resources.wifi_config))
    //     .unwrap();
}

#[embassy_executor::task]
async fn core1_init(spawner: Spawner, resources: ResourcesCore1) {
    if let Some(ui_runner) = resources.ui_runner {
        // Spawn the display task on Core 1
        debug!("Spawn display task on core 1");
        spawner.spawn(display_task(ui_runner)).unwrap();
    }

    // Initialize and spawn synchronization example tasks
    sync_examples::init_sync_system();

    spawner.spawn(sync_examples::mutex_example_task()).unwrap();
    // resources
    //     .spawner
    //     .spawn(sync_examples::sensor_producer_task())
    //     .unwrap();
    // resources
    //     .spawner
    //     .spawn(sync_examples::sensor_consumer_task())
    //     .unwrap();
    // resources
    //     .spawner
    //     .spawn(sync_examples::event_handler_task())
    //     .unwrap();
    // resources
    //     .spawner
    //     .spawn(sync_examples::state_monitor_task())
    //     .unwrap();
    // resources
    //     .spawner
    //     .spawn(sync_examples::data_writer_task())
    //     .unwrap();
    // resources
    //     .spawner
    //     .spawn(sync_examples::data_reader_task())
    //     .unwrap();

    // sync_examples::trigger_test_event();
}

#[embassy_executor::task]
async fn display_task(
    mut ui_runner: UiRunner<
        'static,
        I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, I2C0, i2c::Async>>,
        ssd1306::size::DisplaySize128x64,
        ScreenCollection,
    >,
) -> ! {
    debug!("Starting display task...");
    ui_runner.run().await;
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
