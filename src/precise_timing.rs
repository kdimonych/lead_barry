use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Ticker, Timer};

/// High-precision timing examples for Embassy framework
///
/// Embassy provides several timing primitives for different use cases:
/// 1. Ticker - for periodic tasks with precise intervals
/// 2. Timer - for one-shot delays
/// 3. Instant - for measuring elapsed time
/// 4. Duration - for time periods

/// Example 1: Precise periodic task using Ticker
/// This is the most accurate for regular intervals
#[embassy_executor::task]
pub async fn precise_periodic_task() {
    // Create a ticker that fires every 10ms with high precision
    let mut ticker = Ticker::every(Duration::from_millis(10));
    let mut counter = 0u32;

    loop {
        // Wait for the next tick - this maintains precise timing
        ticker.next().await;

        counter += 1;

        // Your precise work here
        // This will execute exactly every 10ms
        defmt::info!("Precise tick: {}", counter);

        // Even if this work takes time, the next tick will be precisely timed
        do_precise_work().await;
    }
}

/// Example 2: High-frequency ticker (1kHz)
#[embassy_executor::task]
pub async fn high_frequency_task() {
    // 1kHz ticker (1000 times per second)
    let mut ticker = Ticker::every(Duration::from_micros(1000));

    loop {
        ticker.next().await;

        // Critical timing work - keep this fast!
        // At 1kHz, you have only 1ms per iteration
        fast_critical_work();
    }
}

/// Example 3: Precise timing with jitter measurement
#[embassy_executor::task]
pub async fn timing_measurement_task() {
    let mut ticker = Ticker::every(Duration::from_millis(5));
    let mut last_time = Instant::now();
    let mut max_jitter = Duration::from_micros(0);

    loop {
        ticker.next().await;

        let now = Instant::now();
        let elapsed = now.duration_since(last_time);
        let expected = Duration::from_millis(5);

        // Calculate jitter (deviation from expected timing)
        let jitter = if elapsed > expected {
            elapsed - expected
        } else {
            expected - elapsed
        };

        if jitter > max_jitter {
            max_jitter = jitter;
            defmt::warn!("New max jitter: {} μs", jitter.as_micros());
        }

        last_time = now;

        // Log timing stats every 1000 iterations
        static mut COUNTER: u32 = 0;
        unsafe {
            COUNTER += 1;
            if COUNTER % 1000 == 0 {
                defmt::info!(
                    "Completed {} cycles, max jitter: {} μs",
                    COUNTER,
                    max_jitter.as_micros()
                );
            }
        }
    }
}

/// Example 4: Adaptive timing task
/// Adjusts timing based on work completion time
#[embassy_executor::task]
pub async fn adaptive_timing_task() {
    let target_interval = Duration::from_millis(20);
    let mut next_deadline = Instant::now() + target_interval;

    loop {
        let work_start = Instant::now();

        // Do your work
        variable_duration_work().await;

        let work_duration = Instant::now().duration_since(work_start);

        // Sleep until the next deadline
        Timer::at(next_deadline).await;

        // Calculate next deadline
        next_deadline += target_interval;

        // If work took too long, skip ahead to avoid backlog
        let now = Instant::now();
        if next_deadline < now {
            defmt::warn!(
                "Work overran by {} μs",
                now.duration_since(next_deadline).as_micros()
            );
            next_deadline = now + target_interval;
        }

        defmt::debug!("Work took {} μs", work_duration.as_micros());
    }
}

/// Example 5: Multi-rate timing system
/// Different tasks running at different precise rates
pub async fn spawn_multi_rate_tasks(spawner: Spawner) {
    // Fast control loop - 1kHz
    spawner.spawn(fast_control_loop()).unwrap();

    // Medium rate sensor reading - 100Hz
    spawner.spawn(sensor_reading_loop()).unwrap();

    // Slow logging/housekeeping - 1Hz
    spawner.spawn(slow_housekeeping_loop()).unwrap();

    // UI updates - 30Hz
    spawner.spawn(ui_update_loop()).unwrap();
}

#[embassy_executor::task]
async fn fast_control_loop() {
    let mut ticker = Ticker::every(Duration::from_micros(1000)); // 1kHz

    loop {
        ticker.next().await;

        // Critical control logic here
        // Keep this under 500μs to maintain real-time performance
        fast_control_step();
    }
}

#[embassy_executor::task]
async fn sensor_reading_loop() {
    let mut ticker = Ticker::every(Duration::from_millis(10)); // 100Hz

    loop {
        ticker.next().await;

        // Read sensors, process data
        read_and_process_sensors().await;
    }
}

#[embassy_executor::task]
async fn slow_housekeeping_loop() {
    let mut ticker = Ticker::every(Duration::from_secs(1)); // 1Hz

    loop {
        ticker.next().await;

        // Logging, diagnostics, cleanup
        housekeeping_tasks().await;
    }
}

#[embassy_executor::task]
async fn ui_update_loop() {
    let mut ticker = Ticker::every(Duration::from_millis(33)); // ~30Hz

    loop {
        ticker.next().await;

        // Update display, handle user input
        update_ui().await;
    }
}

/// Example 6: Precise timing with error handling
#[embassy_executor::task]
pub async fn robust_timing_task() {
    let mut ticker = Ticker::every(Duration::from_millis(10));
    let mut consecutive_overruns = 0u32;
    const MAX_OVERRUNS: u32 = 5;

    loop {
        let _deadline = Instant::now() + Duration::from_millis(10);
        ticker.next().await;

        let work_start = Instant::now();

        // Do work that might occasionally take too long
        if let Err(_) = timeout_work(Duration::from_millis(8)).await {
            consecutive_overruns += 1;
            defmt::warn!("Work timeout, overruns: {}", consecutive_overruns);

            if consecutive_overruns > MAX_OVERRUNS {
                defmt::error!("Too many consecutive overruns, entering safe mode");
                // Enter degraded performance mode
                safe_mode_operation().await;
                consecutive_overruns = 0;
            }
        } else {
            consecutive_overruns = 0;
        }

        let work_duration = Instant::now().duration_since(work_start);
        if work_duration > Duration::from_millis(8) {
            defmt::warn!("Work took {} ms (target: 8ms)", work_duration.as_millis());
        }
    }
}

/// Helper function with timeout
async fn timeout_work(timeout: Duration) -> Result<(), ()> {
    // Use Timer::after instead of embassy_futures for simplicity
    Timer::after(timeout).await;
    Ok(())
}

// Placeholder implementations for the examples
async fn do_precise_work() {
    // Simulate some work
    Timer::after(Duration::from_micros(100)).await;
}

fn fast_critical_work() {
    // Very fast operation - no async calls!
    for _ in 0..100 {
        core::hint::black_box(42);
    }
}

async fn variable_duration_work() {
    // Work that varies in duration
    static mut COUNTER: u32 = 0;
    unsafe {
        COUNTER += 1;
        let delay = Duration::from_millis((1 + (COUNTER % 10)) as u64);
        Timer::after(delay).await;
    }
}

fn fast_control_step() {
    // Critical control algorithm - keep under 500μs
}

async fn read_and_process_sensors() {
    // Sensor I/O and processing
    Timer::after(Duration::from_millis(1)).await;
}

async fn housekeeping_tasks() {
    // Logging, diagnostics, etc.
    Timer::after(Duration::from_millis(10)).await;
}

async fn update_ui() {
    // UI rendering and input handling
    Timer::after(Duration::from_millis(5)).await;
}

async fn safe_mode_operation() {
    // Reduced functionality mode
    Timer::after(Duration::from_millis(100)).await;
}

async fn actual_work() {
    // The actual work being done
    Timer::after(Duration::from_millis(5)).await;
}

/// Timing best practices:
///
/// 1. Use Ticker for regular intervals - it compensates for drift
/// 2. Keep high-frequency tasks fast and non-blocking
/// 3. Use embassy_futures::select for timeouts
/// 4. Monitor timing performance in debug builds
/// 5. Have fallback strategies for timing overruns
/// 6. Consider task priorities for critical timing
/// 7. Avoid async operations in very fast loops
/// 8. Use interrupt-driven approaches for sub-millisecond timing
/// Timing accuracy on RP2040:
/// - Timer resolution: 1μs
/// - Typical jitter: 1-10μs for Ticker
/// - Maximum practical frequency: ~10kHz for async tasks
/// - For >10kHz, consider interrupt handlers instead
pub mod timing_config {
    use embassy_time::Duration;

    /// Standard control loop frequency (1kHz)
    pub const CONTROL_LOOP_INTERVAL: Duration = Duration::from_micros(1000);

    /// Standard sensor reading frequency (100Hz)
    pub const SENSOR_INTERVAL: Duration = Duration::from_millis(10);

    /// UI update frequency (30Hz)
    pub const UI_INTERVAL: Duration = Duration::from_millis(33);

    /// Housekeeping frequency (1Hz)
    pub const HOUSEKEEPING_INTERVAL: Duration = Duration::from_secs(1);
}
