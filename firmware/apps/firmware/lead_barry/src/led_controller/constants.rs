use embassy_time::Duration;

pub const MAX_MESSAGE_QUEUE_SIZE: usize = 3;
pub const MAGNITUDE: u16 = 255; // Max intensity for 8-bit PWM
pub const SAMPLE_RATE: u32 = 25; // 25 samples per period for smooth animation
pub const DELTA_T: Duration = Duration::from_millis(1000 / SAMPLE_RATE as u64); // Time between animation updates (in milliseconds)
