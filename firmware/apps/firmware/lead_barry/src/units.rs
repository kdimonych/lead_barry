/// Simple units extension for frequency and timing
/// Provides Hz, kHz, MHz methods on numeric types

pub trait FrequencyExt {
    #[allow(dead_code)] // To avoid warnings if not used
    fn hz(self) -> u32;
    #[allow(dead_code)] // To avoid warnings if not used
    fn khz(self) -> u32;
    #[allow(dead_code)] // To avoid warnings if not used
    fn mhz(self) -> u32;
}

impl FrequencyExt for u32 {
    #[allow(dead_code)] // To avoid warnings if not used
    fn hz(self) -> u32 {
        self
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn khz(self) -> u32 {
        self * 1_000
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn mhz(self) -> u32 {
        self * 1_000_000
    }
}

impl FrequencyExt for i32 {
    #[allow(dead_code)] // To avoid warnings if not used
    fn hz(self) -> u32 {
        self as u32
    }
    #[allow(dead_code)] // To avoid warnings if not used
    fn khz(self) -> u32 {
        (self as u32) * 1_000
    }
    #[allow(dead_code)] // To avoid warnings if not used
    fn mhz(self) -> u32 {
        (self as u32) * 1_000_000
    }
}

impl FrequencyExt for f32 {
    #[allow(dead_code)] // To avoid warnings if not used
    fn hz(self) -> u32 {
        self as u32
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn khz(self) -> u32 {
        (self * 1_000.0) as u32
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn mhz(self) -> u32 {
        (self * 1_000_000.0) as u32
    }
}

/// Time/Duration extensions
pub trait TimeExt {
    #[allow(dead_code)] // To avoid warnings if not used
    fn us(self) -> embassy_time::Duration;
    #[allow(dead_code)] // To avoid warnings if not used
    fn ms(self) -> embassy_time::Duration;
    #[allow(dead_code)] // To avoid warnings if not used
    fn s(self) -> embassy_time::Duration;
}

impl TimeExt for u64 {
    #[allow(dead_code)] // To avoid warnings if not used
    fn us(self) -> embassy_time::Duration {
        embassy_time::Duration::from_micros(self)
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn ms(self) -> embassy_time::Duration {
        embassy_time::Duration::from_millis(self)
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn s(self) -> embassy_time::Duration {
        embassy_time::Duration::from_secs(self)
    }
}

impl TimeExt for u32 {
    #[allow(dead_code)] // To avoid warnings if not used
    fn us(self) -> embassy_time::Duration {
        embassy_time::Duration::from_micros(self as u64)
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn ms(self) -> embassy_time::Duration {
        embassy_time::Duration::from_millis(self as u64)
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn s(self) -> embassy_time::Duration {
        embassy_time::Duration::from_secs(self as u64)
    }
}

impl TimeExt for i32 {
    #[allow(dead_code)] // To avoid warnings if not used
    fn us(self) -> embassy_time::Duration {
        embassy_time::Duration::from_micros(self as u64)
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn ms(self) -> embassy_time::Duration {
        embassy_time::Duration::from_millis(self as u64)
    }

    #[allow(dead_code)] // To avoid warnings if not used
    fn s(self) -> embassy_time::Duration {
        embassy_time::Duration::from_secs(self as u64)
    }
}

/// Const-friendly frequency constants (no trait methods)
pub mod freq {
    #[allow(dead_code)] // To avoid warnings if not used
    pub const fn hz(val: u32) -> u32 {
        val
    }
    #[allow(dead_code)] // To avoid warnings if not used
    pub const fn khz(val: u32) -> u32 {
        val * 1_000
    }
    #[allow(dead_code)] // To avoid warnings if not used
    pub const fn mhz(val: u32) -> u32 {
        val * 1_000_000
    }
}

/// Const-friendly time constants
pub mod time {
    use embassy_time::Duration;

    #[allow(dead_code)] // To avoid warnings if not used
    pub const fn us(val: u64) -> Duration {
        Duration::from_micros(val)
    }
    #[allow(dead_code)] // To avoid warnings if not used
    pub const fn ms(val: u64) -> Duration {
        Duration::from_millis(val)
    }
    #[allow(dead_code)] // To avoid warnings if not used
    pub const fn s(val: u64) -> Duration {
        Duration::from_secs(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_ext() {
        assert_eq!(1.khz(), 1_000);
        assert_eq!(2.mhz(), 2_000_000);
        assert_eq!(100.hz(), 100);
    }

    #[test]
    fn test_time_ext() {
        assert_eq!(500.us(), embassy_time::Duration::from_micros(500));
        assert_eq!(10.ms(), embassy_time::Duration::from_millis(10));
        assert_eq!(1.s(), embassy_time::Duration::from_secs(1));
    }
}
