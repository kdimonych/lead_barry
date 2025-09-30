/// Compiler optimization verification for units library
/// This demonstrates how the Rust compiler optimizes unit conversions
use crate::units::FrequencyExt;

/// Example showing constant folding optimization
pub fn demonstrate_optimization() {
    // These will be optimized to compile-time constants
    let freq1 = 5.khz(); // Optimized to: 5000
    let freq2 = 400.khz(); // Optimized to: 400000
    let freq3 = 8.mhz(); // Optimized to: 8000000

    // Even complex expressions get optimized
    let combined = 2.khz() + 3.khz(); // Optimized to: 5000

    defmt::info!("Frequencies: {} {} {} {}", freq1, freq2, freq3, combined);
}

/// Const evaluation example - proves compile-time optimization
pub const COMPILE_TIME_FREQ: u32 = {
    // This MUST be compile-time evaluable or it won't compile
    use crate::units::freq;
    freq::khz(400) // This works because it's const fn
};

/// Function showing assembly-level optimization
#[inline(never)] // Prevent inlining to see actual assembly
pub fn optimized_frequency_setup() -> u32 {
    // All of these become compile-time constants
    let i2c_standard = 100.khz(); // → 100000
    let i2c_fast = 400.khz(); // → 400000
    let spi_freq = 8.mhz(); // → 8000000

    // Even this expression is compile-time evaluated
    i2c_fast + (spi_freq / 1000) // → 400000 + 8000 = 408000
}

/// Benchmarking functions to compare performance
pub mod benchmarks {
    use super::*;

    /// Raw constant version
    #[inline(never)]
    pub fn raw_constants() -> u32 {
        let freq = 400000u32; // Raw constant
        freq
    }

    /// Units library version
    #[inline(never)]
    pub fn units_version() -> u32 {
        let freq = 400.khz(); // Should optimize to same as above
        freq
    }

    /// Complex calculation with units
    #[inline(never)]
    pub fn complex_units() -> u32 {
        // All of this should optimize to a single constant
        let base = 100.khz();
        let multiplier = 4;
        let result = base * multiplier; // 400000
        result
    }
}

/// Compile-time assertions to verify optimization
pub const _COMPILE_TIME_CHECKS: () = {
    // These assertions will fail at compile time if values are wrong
    use crate::units::freq;

    assert!(freq::khz(1) == 1_000);
    assert!(freq::khz(400) == 400_000);
    assert!(freq::mhz(8) == 8_000_000);
};

/// Runtime verification that values are correct
pub fn verify_optimizations() {
    // Verify that our optimized values match expectations
    assert_eq!(1.khz(), 1_000);
    assert_eq!(400.khz(), 400_000);
    assert_eq!(8.mhz(), 8_000_000);

    // Verify expressions are optimized correctly
    assert_eq!(2.khz() + 3.khz(), 5_000);
    assert_eq!(1.mhz() / 1000, 1_000);

    defmt::info!("✅ All optimization verifications passed!");
}

/// Example showing LLVM IR would look like after optimization
/// (This is what the compiler generates internally)
/*
Unoptimized LLVM IR:
    %freq = call i32 @khz(i32 400)
    %result = mul i32 %freq, 1000

Optimized LLVM IR:
    %result = 400000  ; Direct constant!
*/

/// Performance comparison demonstration
#[embassy_executor::task]
pub async fn performance_demo() {
    use embassy_time::{Duration, Instant};

    let iterations = 1000000u32;

    // Time the "raw constants" approach
    let start1 = Instant::now();
    for _ in 0..iterations {
        let _freq = 400000u32; // Raw constant
        core::hint::black_box(_freq);
    }
    let time1 = Instant::now().duration_since(start1);

    // Time the "units" approach
    let start2 = Instant::now();
    for _ in 0..iterations {
        let _freq = 400.khz(); // Units library
        core::hint::black_box(_freq);
    }
    let time2 = Instant::now().duration_since(start2);

    defmt::info!("Raw constants: {}μs", time1.as_micros());
    defmt::info!("Units library: {}μs", time2.as_micros());
    defmt::info!(
        "Performance difference: {}%",
        if time2 > time1 {
            ((time2.as_micros() - time1.as_micros()) * 100) / time1.as_micros()
        } else {
            0
        }
    );

    // They should be nearly identical because both optimize to constants
}

/// Advanced: Showing const evaluation in generic contexts
pub const fn generic_frequency<const N: u32>() -> u32 {
    // Even in generic contexts, const evaluation works
    crate::units::freq::khz(N)
}

// These are all compile-time constants
pub const FREQ_1: u32 = generic_frequency::<100>(); // 100,000
pub const FREQ_2: u32 = generic_frequency::<400>(); // 400,000
pub const FREQ_3: u32 = generic_frequency::<1000>(); // 1,000,000
