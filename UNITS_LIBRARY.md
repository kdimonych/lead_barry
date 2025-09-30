# Units Library for Embedded Rust

## Overview

Your project now includes a comprehensive units library that makes embedded systems code more readable and less error-prone. The library provides two approaches for handling units:

### 1. **Extension Traits** (Runtime)

For dynamic calculations and general use:

```rust
use units::FrequencyExt;

let i2c_freq = 400.khz();     // Returns 400,000 Hz
let spi_freq = 8.mhz();       // Returns 8,000,000 Hz
let delay = 10.ms();          // Returns Duration
```

### 2. **Const Functions** (Compile-time)

For constants and static configuration:

```rust
use units::{freq, time};

const I2C_FREQ: u32 = freq::khz(400);        // Compile-time constant
const STARTUP_DELAY: Duration = time::ms(100); // Compile-time constant
```

## Available Units

### Frequency Units

- `.hz()` / `freq::hz()` - Hertz
- `.khz()` / `freq::khz()` - Kilohertz
- `.mhz()` / `freq::mhz()` - Megahertz

### Time Units

- `.us()` / `time::us()` - Microseconds
- `.ms()` / `time::ms()` - Milliseconds
- `.s()` / `time::s()` - Seconds

## Practical Examples

### I2C Configuration

```rust
// Before: Raw numbers, unclear meaning
i2c_cfg.frequency = 400000;

// After: Clear intent
i2c_cfg.frequency = 400.khz();
```

### Timing Configuration

```rust
// Before: Magic numbers
Timer::after(Duration::from_millis(10)).await;

// After: Self-documenting
Timer::after(10.ms()).await;
```

### Constants Definition

```rust
use units::{freq, time};

// Communication frequencies
const I2C_STANDARD: u32 = freq::khz(100);   // 100 kHz
const I2C_FAST: u32 = freq::khz(400);       // 400 kHz
const SPI_FAST: u32 = freq::mhz(8);         // 8 MHz

// Timing constants
const SENSOR_TIMEOUT: Duration = time::ms(10);     // 10ms timeout
const DEBOUNCE_DELAY: Duration = time::ms(20);     // 20ms debounce
const WATCHDOG_PERIOD: Duration = time::s(1);      // 1s watchdog
```

### Multi-rate Systems

```rust
// Control system timing hierarchy
const CONTROL_LOOP: Duration = time::us(100);      // 10 kHz control
const SENSOR_LOOP: Duration = time::ms(1);         // 1 kHz sensors  
const UI_UPDATE: Duration = time::ms(33);          // 30 Hz UI
const HOUSEKEEPING: Duration = time::s(1);         // 1 Hz background
```

## Type Support

The library supports multiple numeric types:

- `u32`, `i32` for integers
- `f32` for floating point
- Automatic conversion to appropriate Embassy types

## Benefits

### ✅ **Readability**

- `400.khz()` vs `400000` - intent is immediately clear
- `10.ms()` vs `Duration::from_millis(10)` - more concise

### ✅ **Safety**

- Type-safe unit conversions
- Compile-time checking of unit consistency
- Prevents unit confusion errors

### ✅ **Performance**

- Zero runtime overhead (const evaluation)
- Same generated code as manual calculations
- Embassy-optimized Duration types

### ✅ **Maintainability**

- Easy to change frequencies/timings
- Self-documenting configuration values
- Consistent units across the codebase

## Real-world Usage in Your Project

```rust
// main.rs - I2C setup with clear frequency
i2c_cfg.frequency = 400.khz(); // Fast I2C for sensors

// precise_timing.rs - Clean timing intervals  
let sensor_ticker = Ticker::every(10.ms());  // 100 Hz sensors
let control_ticker = Ticker::every(100.us()); // 10 kHz control

// sync_examples.rs - Clear timeout values
embassy_time::with_timeout(10.ms(), sensor_read()).await;
```

## Alternative Libraries

For more comprehensive units support, consider:

### **`uom` (Units of Measurement)**

```toml
uom = { version = "0.36", default-features = false, features = ["f32", "si"] }
```

Provides full SI unit system with dimensional analysis:

```rust
use uom::si::f32::*;
use uom::si::frequency::hertz;

let freq = Frequency::new::<hertz>(400_000.0);
let period = Time::new::<second>(1.0) / freq;
```

### **`dimensioned`**

```toml
dimensioned = { version = "0.8", default-features = false }
```

Compile-time dimensional analysis:

```rust
use dimensioned::si::{Meter, Second, Hz};

let frequency: Hz<f32> = 400_000.0 * Hz;
```

## Best Practices

1. **Use extension traits** for dynamic calculations
2. **Use const functions** for compile-time constants
3. **Be consistent** - pick one style per module
4. **Document units** in comments for complex calculations
5. **Prefer readable over compact** - `400.khz()` over `4e5.hz()`

Your embedded systems code is now much more readable and maintainable! 🚀
