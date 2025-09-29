//! Blinks the LED on a Pico W board
//!
//! This will blink the onboard LED on a Pico W, which is controlled via the CYW43 WiFi chip.
#![no_std]
#![no_main]

mod matrix_ops;

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::{
    gpio::{Level, Output},
    i2c::{self, I2c, InterruptHandler},
    peripherals::I2C0,
};
use embassy_time::{Duration, Timer};
use micromath::F32Ext;

// Display driver imports
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle, Triangle},
    text::{Baseline, Text, TextStyleBuilder},
};
use ssd1306::I2CDisplayInterface;
use ssd1306::Ssd1306Async;
use ssd1306::prelude::*;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

// Shared interfaces
type I2cBus = Mutex<ThreadModeRawMutex, I2c<'static, I2C0, i2c::Async>>;
bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
});

fn rotate(angle_rad: f32, pivot: Point, point: Point) -> Point {
    let cos_a = (angle_rad.cos() * 10000.0) as i32;
    let sin_a = (angle_rad.sin() * 10000.0) as i32;

    let translated_x = point.x - pivot.x;
    let translated_y = point.y - pivot.y;

    let rotated_x = (translated_x * cos_a - translated_y * sin_a) / 10000;
    let rotated_y = (translated_x * sin_a + translated_y * cos_a) / 10000;

    Point::new(rotated_x + pivot.x, rotated_y + pivot.y)
}

#[embassy_executor::task]
async fn display(i2c_bus: &'static I2cBus) {
    use matrix_ops::*;
    let i2c_dev = I2cDevice::new(i2c_bus);
    let interface = I2CDisplayInterface::new(i2c_dev);
    let mut disp = Ssd1306Async::new(
        interface,
        ssd1306::prelude::DisplaySize128x64,
        ssd1306::prelude::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    disp.init().await.unwrap();
    disp.flush().await.unwrap();

    info!("Display initialized");

    let mut angle = 0.0f32;
    loop {
        let yoffset = 8;

        disp.clear_buffer();

        let style = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .build();

        // screen outline
        // default display size is 128x64 if you don't pass a _DisplaySize_
        // enum to the _Builder_ struct
        Rectangle::new(Point::new(0, 0), Size::new(127, 31))
            .into_styled(style)
            .draw(&mut disp)
            .unwrap();

        // triangle
        let tr_pivot = Point::new(16 + 8, yoffset + 8);
        Triangle::new(
            rotate(angle, tr_pivot, Point::new(16, 16 + yoffset)),
            rotate(angle, tr_pivot, Point::new(16 + 16, 16 + yoffset)),
            rotate(angle, tr_pivot, Point::new(16 + 8, yoffset)),
        )
        .into_styled(style)
        .draw(&mut disp)
        .unwrap();

        // square
        Rectangle::new(Point::new(52, yoffset), Size::new_equal(16))
            .into_styled(style)
            .draw(&mut disp)
            .unwrap();

        // circle
        Circle::new(Point::new(88, yoffset), 16)
            .into_styled(style)
            .draw(&mut disp)
            .unwrap();
        disp.flush().await.unwrap();

        angle += 0.2;
        if angle > core::f32::consts::TAU {
            // 2π
            angle = 0.0;
        }
        Timer::after(Duration::from_millis(2)).await;
    }
    // You can add more display code here, e.g., drawing graphics or text
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

        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Program start");
    let p = embassy_rp::init(Default::default());

    // For regular GPIO LED (if you connect an external LED to a GPIO pin)
    let mut led_pin = Output::new(p.PIN_22, Level::Low);

    // Setup I2C
    let i2c = I2c::new_async(p.I2C0, p.PIN_5, p.PIN_4, Irqs, i2c::Config::default());
    static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));

    spawner.spawn(display(i2c_bus)).unwrap();
    spawner.spawn(matrix_operations_task()).unwrap();

    // Note: For the onboard LED on Pico W, you'd need to use the CYW43 driver
    // This example uses a regular GPIO pin. See the embassy examples for CYW43 usage.
    loop {
        info!("on!");
        led_pin.set_high();
        Timer::after(Duration::from_millis(1500)).await;
        info!("off!");
        led_pin.set_low();
        Timer::after(Duration::from_millis(1500)).await;
    }
}
