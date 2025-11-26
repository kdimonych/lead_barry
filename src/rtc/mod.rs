use ds323x::ic::DS3231;
use ds323x::*;
use ds323x::{DateTimeAccess, Ds323xAsync, NaiveDate, Rtcc};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

static RTC_DS3231: StaticCell<
    Mutex<
        CriticalSectionRawMutex,
        Ds323xAsync<
            I2cDevice<'static, CriticalSectionRawMutex, embassy_rp::peripherals::I2C1>,
            DS3231,
        >,
    >,
> = StaticCell::new();

pub async fn init_rtc<I2C, E>(
    i2c_device: I2C,
) -> Ds323xAsync<interface::I2cInterfaceAsync<I2C>, DS3231>
where
    I2C: embedded_hal_async::i2c::I2c<Error = E>,
{
    let mut rtc = Ds323xAsync::new_ds3231(i2c_device);
    let datetime = NaiveDate::from_ymd_opt(2020, 5, 1)
        .unwrap()
        .and_hms_opt(19, 59, 58)
        .unwrap();
    rtc.set_datetime(&datetime).await.ok();
    let time = rtc.time().await.ok();
    //defmt::info!("RTC initialized with time: {:?}", time);

    rtc // Return the RTC instead of destroying it
}
