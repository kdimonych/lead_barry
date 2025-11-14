use cortex_m::peripheral::SCB;
use embassy_executor::{Executor, Spawner};
use embassy_time::{Duration, Timer};

pub fn trigger_system_reset() -> ! {
    cortex_m::interrupt::disable();
    SCB::sys_reset();
}

#[embassy_executor::task]
async fn deferred_system_reset_task(delay: Duration) {
    Timer::after(delay).await;
    trigger_system_reset();
}

pub fn deferred_system_reset(spawner: Spawner, delay: Duration) {
    // Implement a deferred reset mechanism if needed
    // For example, setting a flag to reset later
    spawner.spawn(deferred_system_reset_task(delay)).unwrap();
}
