//! Blinks the LED on a Pico W board
//!
//! This will blink the onboard LED on a Pico W, which is controlled via the CYW43 WiFi chip.
#![no_std]
#![no_main]

mod ina3221_sensor;
mod matrix_ops;
mod optimization_demo;
mod precise_timing;
mod sync_examples;
mod ui;
mod units;
mod units_examples;

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{Executor, Spawner};
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    i2c::{self, I2c, InterruptHandler as I2cInterruptHandler},
    multicore::Stack,
    peripherals::I2C0,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex, watch::Watch};
use embassy_time::{Duration, Timer};

use static_cell::StaticCell;

use crate::units::{FrequencyExt, TimeExt};
use micromath::F32Ext;
use ui::*;

// Display driver imports
use {defmt_rtt as _, panic_probe as _};

// Interrupt handlers
bind_interrupts!(struct Irqs {
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
});

// Shared interfaces
type I2cBus = Mutex<ThreadModeRawMutex, I2c<'static, I2C0, i2c::Async>>;

// Static resources
static CORE1_STACK: StaticCell<Stack<4096>> = StaticCell::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
static LED_PIN: StaticCell<Output> = StaticCell::new();

struct ResourcesCore0 {
    spawner: Spawner,
    i2c_bus: &'static I2cBus,
    led: &'static mut Output<'static>,
}

struct ResourcesCore1 {
    spawner: Spawner,
    i2c_bus: &'static I2cBus,
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    // For regular GPIO LED (if you connect an external LED to a GPIO pin)
    let led = LED_PIN.init(Output::new(p.PIN_22, Level::Low));

    // Setup I2C with standard frequency for sensors
    let mut i2c_cfg = i2c::Config::default();
    i2c_cfg.frequency = 1.mhz(); // Fast I2C for better performance
    let i2c = I2c::new_async(p.I2C0, p.PIN_5, p.PIN_4, Irqs, i2c_cfg);

    let i2c_bus: &'static Mutex<ThreadModeRawMutex, I2c<'static, I2C0, i2c::Async>> =
        I2C_BUS.init(Mutex::new(i2c));

    // Initialize the stack
    let stack = CORE1_STACK.init(Stack::new());

    embassy_rp::multicore::spawn_core1(p.CORE1, stack, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| core1_init(ResourcesCore1 { spawner, i2c_bus }));
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        core0_init(ResourcesCore0 {
            spawner,
            i2c_bus,
            led,
        })
    });
}

/// Watch for broadcasting system state
static UI_STATE: Watch<ThreadModeRawMutex, ScreenSet, 1> = Watch::new();

#[embassy_executor::task]
async fn led_task(led: &'static mut Output<'static>) {
    loop {
        info!("on!");
        // Start with the welcome screen
        UI_STATE
            .sender()
            .send(ScreenSet::Welcome(WelcomeScreen::new()));

        led.set_high();
        Timer::after(5.s()).await;
        info!("off!");
        // Start with the animation screen
        UI_STATE
            .sender()
            .send(ScreenSet::Animation(AnimationScreen::new()));

        led.set_low();
        Timer::after(5.s()).await;
    }
}

fn core0_init(resources: ResourcesCore0) {
    // Spawn the LED blink task on Core 0
    resources.spawner.spawn(led_task(resources.led)).unwrap();

    resources.spawner.spawn(matrix_operations_task()).unwrap();
    resources.spawner.spawn(precise_sensor_task()).unwrap();
}

fn core1_init(resources: ResourcesCore1) {
    // Spawn the display task on Core 1
    resources.spawner.spawn(display(resources.i2c_bus)).unwrap();

    // Initialize and spawn synchronization example tasks
    sync_examples::init_sync_system();
    resources
        .spawner
        .spawn(sync_examples::mutex_example_task())
        .unwrap();
    resources
        .spawner
        .spawn(sync_examples::sensor_producer_task())
        .unwrap();
    resources
        .spawner
        .spawn(sync_examples::sensor_consumer_task())
        .unwrap();
    resources
        .spawner
        .spawn(sync_examples::event_handler_task())
        .unwrap();
    resources
        .spawner
        .spawn(sync_examples::state_monitor_task())
        .unwrap();
    resources
        .spawner
        .spawn(sync_examples::data_writer_task())
        .unwrap();
    resources
        .spawner
        .spawn(sync_examples::data_reader_task())
        .unwrap();

    sync_examples::trigger_test_event();
}

#[embassy_executor::task]
async fn display(i2c_bus: &'static I2cBus) {
    let i2c_dev = I2cDevice::new(i2c_bus);

    let state_receiver = UI_STATE.dyn_receiver().unwrap();
    let mut ui = UiInterface::new(i2c_dev, ssd1306::size::DisplaySize128x64, state_receiver);

    // Initialize the display
    ui.init().await;
    ui.draw_once().await;

    // Draw loop for animation screen. This will run indefinitely.
    ui.draw_loop().await;
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

        // Your precise sensor reading code here
        // Keep this fast - target <5ms to maintain 100Hz
        simulate_precise_sensor_work().await;

        let work_duration = Instant::now().duration_since(work_start);

        // Log performance every 1000 iterations (10 seconds)
        if counter % 1000 == 0 {
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

async fn simulate_precise_sensor_work() {
    // Simulate sensor I2C transaction + processing
    // In real code, this would be your INA3221 readings or other sensors
    embassy_time::Timer::after(Duration::from_micros(500)).await;
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
