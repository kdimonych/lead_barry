use ds323x::Ds323xAsync;
use ds323x::ic::DS3231;
use ds323x::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

pub use ds323x::{DateTimeAccess, Datelike, NaiveDateTime, Rtcc};

pub type RtcDs3231<I2C> = Ds323xAsync<interface::I2cInterfaceAsync<I2C>, DS3231>;
pub type RtcDs3231Ref<I2C> = Mutex<CriticalSectionRawMutex, RtcDs3231<I2C>>;

pub fn create_rtc_ds3231<I2C, E>(i2c_device: I2C) -> RtcDs3231Ref<I2C>
where
    I2C: embedded_hal_async::i2c::I2c<Error = E>,
{
    Mutex::new(Ds323xAsync::new_ds3231(i2c_device))
}
