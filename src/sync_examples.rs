//! Embassy synchronization primitives demonstration
//!
//! This module shows how to use various Embassy sync primitives
//! for coordinating between async tasks.

use defmt::*;
use embassy_executor;
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex, channel::Channel, mutex::Mutex, pipe::Pipe,
    signal::Signal, watch::Watch,
};
use embassy_time::{Duration, Timer};

/// Shared counter protected by mutex
static COUNTER: Mutex<ThreadModeRawMutex, u32> = Mutex::new(0);

/// Channel for sending sensor data between tasks
static SENSOR_CHANNEL: Channel<ThreadModeRawMutex, SensorData, 8> = Channel::new();

/// Signal for system events
static SYSTEM_EVENT: Signal<ThreadModeRawMutex, SystemEvent> = Signal::new();

/// Watch for broadcasting system state
static SYSTEM_STATE: Watch<ThreadModeRawMutex, SystemState, 3> = Watch::new();

/// Pipe for streaming data
static DATA_PIPE: Pipe<ThreadModeRawMutex, 256> = Pipe::new();

#[derive(Clone, Copy, Debug, defmt::Format)]
pub struct SensorData {
    pub temperature: f32,
    pub humidity: f32,
    pub timestamp: u64,
}

#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum SystemEvent {
    ButtonPressed,
    SensorError,
    NetworkConnected,
    LowBattery,
}

#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum SystemState {
    Initializing,
    Running,
    Error,
    Sleeping,
}

/// Demonstrates mutex usage for shared state
#[embassy_executor::task]
pub async fn mutex_example_task() {
    loop {
        // Lock and increment counter
        {
            let mut counter = COUNTER.lock().await;
            *counter += 1;
            if *counter % 10 == 0 {
                info!("Counter reached: {}", *counter);
            }
        } // Mutex automatically unlocked here

        Timer::after(Duration::from_millis(500)).await;
    }
}

/// Demonstrates channel producer
#[embassy_executor::task]
pub async fn sensor_producer_task() {
    let mut timestamp = 0u64;

    loop {
        let sensor_data = SensorData {
            temperature: 20.0 + (timestamp as f32 % 10.0),
            humidity: 50.0 + (timestamp as f32 % 20.0),
            timestamp,
        };

        // Send sensor data through channel
        SENSOR_CHANNEL.send(sensor_data).await;
        info!("Sent sensor data: {}", sensor_data);

        timestamp += 1;
        Timer::after(Duration::from_secs(2)).await;
    }
}

/// Demonstrates channel consumer
#[embassy_executor::task]
pub async fn sensor_consumer_task() {
    loop {
        // Receive sensor data from channel
        let data = SENSOR_CHANNEL.receive().await;
        info!(
            "Received sensor data: temp={}, humidity={}",
            data.temperature, data.humidity
        );

        // Process the data...
        if data.temperature > 25.0 {
            info!("High temperature detected!");
            SYSTEM_EVENT.signal(SystemEvent::SensorError);
        }
    }
}

/// Demonstrates signal usage for events
#[embassy_executor::task]
pub async fn event_handler_task() {
    loop {
        // Wait for system events
        let event = SYSTEM_EVENT.wait().await;
        info!("System event occurred: {}", event);

        match event {
            SystemEvent::ButtonPressed => {
                info!("Handling button press...");
                // Update system state
                SYSTEM_STATE.sender().send(SystemState::Running);
            }
            SystemEvent::SensorError => {
                info!("Handling sensor error...");
                SYSTEM_STATE.sender().send(SystemState::Error);
            }
            SystemEvent::NetworkConnected => {
                info!("Network is now available");
                SYSTEM_STATE.sender().send(SystemState::Running);
            }
            SystemEvent::LowBattery => {
                info!("Entering low power mode...");
                SYSTEM_STATE.sender().send(SystemState::Sleeping);
            }
        }
    }
}

/// Demonstrates watch for state monitoring
#[embassy_executor::task]
pub async fn state_monitor_task() {
    let mut receiver = SYSTEM_STATE.receiver().unwrap();

    loop {
        // Wait for state changes
        let state = receiver.changed().await;
        info!("System state changed to: {}", state);

        match state {
            SystemState::Initializing => {
                info!("System is starting up...");
            }
            SystemState::Running => {
                info!("System is operational");
            }
            SystemState::Error => {
                info!("System encountered an error");
                // Could trigger recovery procedures
            }
            SystemState::Sleeping => {
                info!("System entering sleep mode");
                // Reduce power consumption
            }
        }
    }
}

/// Demonstrates pipe for streaming data
#[embassy_executor::task]
pub async fn data_writer_task() {
    let mut data_counter = 0u8;

    loop {
        let data = [data_counter; 32]; // 32 bytes of test data

        // Write data to pipe
        DATA_PIPE.write(&data).await;
        info!("Wrote {} bytes to pipe", data.len());

        data_counter = data_counter.wrapping_add(1);
        Timer::after(Duration::from_millis(1000)).await;
    }
}

/// Demonstrates pipe for streaming data
#[embassy_executor::task]
pub async fn data_reader_task() {
    let mut buffer = [0u8; 16];

    loop {
        // Read data from pipe
        let bytes_read = DATA_PIPE.read(&mut buffer).await;
        info!(
            "Read {} bytes from pipe: {:?}",
            bytes_read,
            &buffer[..bytes_read]
        );
    }
}

/// Initialize system state
pub fn init_sync_system() {
    // Initialize system state
    SYSTEM_STATE.sender().send(SystemState::Initializing);
    info!("Sync system initialized");
}

/// Trigger a test event (for demonstration)
pub fn trigger_test_event() {
    SYSTEM_EVENT.signal(SystemEvent::ButtonPressed);
}

/// Advanced: Try operations (non-blocking)
pub fn try_operations_example() {
    // Try to receive without blocking
    match SENSOR_CHANNEL.try_receive() {
        Ok(data) => info!("Got immediate data: {}", data),
        Err(_) => info!("No data available right now"),
    }

    // Try to send without blocking
    let test_data = SensorData {
        temperature: 25.0,
        humidity: 60.0,
        timestamp: 12345,
    };

    match SENSOR_CHANNEL.try_send(test_data) {
        Ok(_) => info!("Sent data immediately"),
        Err(_) => info!("Channel is full, cannot send"),
    }
}
