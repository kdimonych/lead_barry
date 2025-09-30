/// Examples of using units in embedded systems
/// This shows practical usage patterns for the units library
use crate::units::{FrequencyExt, TimeExt};
use embassy_time::{Duration, Ticker};

/// Example timing configurations using the units library
pub mod timing_examples {
    use super::*;

    /// Standard timing configurations for embedded systems
    pub fn demo_timing_units() {
        // Frequency units - cleaner than raw numbers
        let i2c_freq = 100.khz(); // 100,000 Hz
        let spi_freq = 8.mhz(); // 8,000,000 Hz
        let uart_baud = 115200.hz(); // 115,200 Hz

        // Time units - more readable than raw Duration calls
        let short_delay = 500.us(); // 500 microseconds
        let medium_delay = 10.ms(); // 10 milliseconds
        let long_delay = 2.s(); // 2 seconds

        // Use in real configurations
        defmt::info!("I2C frequency: {} Hz", i2c_freq);
        defmt::info!("SPI frequency: {} Hz", spi_freq);
        defmt::info!("UART baud: {} Hz", uart_baud);
    }

    /// Example task with clean timing units
    #[embassy_executor::task]
    pub async fn sensor_sampling_task() {
        // Sample sensors at 1kHz (every 1ms)
        let mut ticker = Ticker::every(1.ms());

        loop {
            ticker.next().await;

            // Fast sensor reading - target under 500μs
            fast_sensor_read().await;
        }
    }

    /// High-frequency control loop
    #[embassy_executor::task]
    pub async fn control_loop_task() {
        // 10kHz control loop (100μs period)
        let mut ticker = Ticker::every(100.us());

        loop {
            ticker.next().await;

            // Critical control algorithm
            control_step();
        }
    }

    /// Communication setup with clean frequency specs
    pub fn setup_communications() {
        // Standard communication frequencies
        let i2c_standard = 100.khz(); // Standard I2C
        let i2c_fast = 400.khz(); // Fast I2C
        let i2c_fast_plus = 1.mhz(); // Fast+ I2C

        let spi_low = 1.mhz(); // Conservative SPI
        let spi_standard = 8.mhz(); // Standard SPI
        let spi_high = 25.mhz(); // High-speed SPI

        defmt::info!(
            "I2C frequencies: {} / {} / {} Hz",
            i2c_standard,
            i2c_fast,
            i2c_fast_plus
        );
        defmt::info!(
            "SPI frequencies: {} / {} / {} Hz",
            spi_low,
            spi_standard,
            spi_high
        );
    }

    /// Timing budget analysis
    pub fn analyze_timing_budget() {
        // System timing requirements
        let control_period = 100.us(); // 10kHz control
        let sensor_period = 1.ms(); // 1kHz sensors
        let ui_period = 33.ms(); // 30Hz UI

        // Work durations
        let control_work = 50.us(); // Control takes 50μs
        let sensor_work = 500.us(); // Sensor read takes 500μs
        let ui_work = 10.ms(); // UI update takes 10ms

        // Calculate utilization
        let control_util = (control_work.as_micros() * 100) / control_period.as_micros();
        let sensor_util = (sensor_work.as_micros() * 100) / sensor_period.as_micros();
        let ui_util = (ui_work.as_millis() * 100) / ui_period.as_millis();

        defmt::info!("Control loop utilization: {}%", control_util);
        defmt::info!("Sensor loop utilization: {}%", sensor_util);
        defmt::info!("UI loop utilization: {}%", ui_util);
    }

    // Helper functions
    async fn fast_sensor_read() {
        embassy_time::Timer::after(50.us()).await;
    }

    fn control_step() {
        // Fast control algorithm
        for _ in 0..10 {
            core::hint::black_box(42);
        }
    }
}

/// Real-world frequency constants for embedded systems
pub mod common_frequencies {
    use crate::units::freq;

    // Clock frequencies
    pub const CRYSTAL_16MHZ: u32 = freq::mhz(16);
    pub const CRYSTAL_8MHZ: u32 = freq::mhz(8);
    pub const USB_48MHZ: u32 = freq::mhz(48);

    // Communication frequencies
    pub const I2C_STANDARD: u32 = freq::khz(100);
    pub const I2C_FAST: u32 = freq::khz(400);
    pub const I2C_FAST_PLUS: u32 = freq::mhz(1);

    pub const SPI_1MHZ: u32 = freq::mhz(1);
    pub const SPI_8MHZ: u32 = freq::mhz(8);
    pub const SPI_25MHZ: u32 = freq::mhz(25);

    pub const UART_9600: u32 = freq::hz(9600);
    pub const UART_115200: u32 = freq::hz(115200);
    pub const UART_921600: u32 = freq::hz(921600);

    // PWM frequencies
    pub const PWM_1KHZ: u32 = freq::khz(1);
    pub const PWM_20KHZ: u32 = freq::khz(20); // Above human hearing
    pub const PWM_100KHZ: u32 = freq::khz(100); // High efficiency switching

    // Sensor sampling rates
    pub const IMU_1KHZ: u32 = freq::khz(1); // High-rate IMU
    pub const TEMP_1HZ: u32 = freq::hz(1); // Slow temperature
    pub const ADC_10KHZ: u32 = freq::khz(10); // Fast ADC sampling
}

/// Time constants for embedded systems
pub mod common_delays {
    use crate::units::time;
    use embassy_time::Duration;

    // Startup delays
    pub const POWER_ON_DELAY: Duration = time::ms(100);
    pub const SENSOR_INIT_DELAY: Duration = time::ms(50);
    pub const OSCILLATOR_STARTUP: Duration = time::ms(10);

    // Communication timeouts
    pub const I2C_TIMEOUT: Duration = time::ms(10);
    pub const SPI_TIMEOUT: Duration = time::ms(1);
    pub const UART_TIMEOUT: Duration = time::ms(100);

    // Debounce delays
    pub const BUTTON_DEBOUNCE: Duration = time::ms(20);
    pub const SWITCH_DEBOUNCE: Duration = time::ms(50);

    // Watchdog intervals
    pub const WATCHDOG_FAST: Duration = time::ms(100);
    pub const WATCHDOG_NORMAL: Duration = time::s(1);
    pub const WATCHDOG_SLOW: Duration = time::s(10);
}
