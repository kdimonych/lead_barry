use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_rp::{
    i2c::{self, I2c},
    peripherals::{I2C0, I2C1},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

// Global types
pub type I2c0Bus = Mutex<CriticalSectionRawMutex, I2c<'static, I2C0, i2c::Async>>;
pub type I2c1Bus = Mutex<CriticalSectionRawMutex, I2c<'static, I2C1, i2c::Async>>;
pub type I2c0DeviceType<'a> = I2cDevice<'a, CriticalSectionRawMutex, I2c<'a, I2C0, i2c::Async>>;
pub type I2c1DeviceType<'a> = I2cDevice<'a, CriticalSectionRawMutex, I2c<'a, I2C1, i2c::Async>>;
