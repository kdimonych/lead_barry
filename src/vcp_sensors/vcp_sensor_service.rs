use core::error::Error;

use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
    watch::Watch,
};
use embassy_time::Ticker;
use ina3221_async::INA3221Async;

use crate::{
    units::TimeExt,
    vcp_sensors::vcp_data_model::{ChannelNum, VcpReading, VcpSensorsEvents, VcpState},
};

pub type VcpMode = ina3221_async::OperatingMode;
//pub use ina3221_async::OperatingMode::*;

enum VcpCommand {
    EnableChannel(ChannelNum),
    DisableChannel(ChannelNum),
    SetMode(VcpMode),
}

type VcpEventChannel<const EVENT_QUEUE_SIZE: usize> =
    Channel<CriticalSectionRawMutex, VcpSensorsEvents, EVENT_QUEUE_SIZE>;
type VcpCommandChannel = Channel<CriticalSectionRawMutex, VcpCommand, 1>;
type VcpCurrentReading<const READING_CONSUMERS: usize> =
    Watch<CriticalSectionRawMutex, [VcpReading; 3], READING_CONSUMERS>;
type VcpControlSignal<const READING_CONSUMERS: usize> =
    Watch<CriticalSectionRawMutex, [VcpReading; 3], READING_CONSUMERS>;
pub struct VcpSensorsState<const READING_CONSUMERS: usize, const EVENT_QUEUE_SIZE: usize> {
    events: VcpEventChannel<EVENT_QUEUE_SIZE>,
    readings: VcpCurrentReading<READING_CONSUMERS>,
    control: VcpCommandChannel,
}

impl<const READING_CONSUMERS: usize, const EVENT_QUEUE_SIZE: usize>
    VcpSensorsState<READING_CONSUMERS, EVENT_QUEUE_SIZE>
{
    pub const fn new() -> Self {
        Self {
            events: Channel::new(),
            readings: Watch::new(),
            control: Channel::new(),
        }
    }
}
#[derive(Debug, Copy, Clone, defmt::Format)]
pub struct VcpLimits {
    pub min_voltage: f32,
    pub max_voltage: f32,
    pub min_current: f32,
    pub max_current: f32,
}

impl Default for VcpLimits {
    fn default() -> Self {
        Self {
            min_voltage: 0.0,
            max_voltage: 5.0,
            min_current: 0.0,
            max_current: 2.0,
        }
    }
}

macro_rules! positive_f32 {
    ($value:expr) => {{
        const _: () = core::assert!($value > 0.0, "Value must be positive");
        $value
    }};
}

#[derive(Debug)]
pub struct VcpConfig {
    pub limits: [VcpLimits; 3],
    shunt_resistance: [f32; 3],
    pub initial_mode: VcpMode,
}

impl VcpConfig {
    pub fn shunt_resistance(&self, channel: ChannelNum) -> f32 {
        self.shunt_resistance[channel as usize]
    }
    pub fn set_shunt_resistance(
        &mut self,
        channel: ChannelNum,
        resistance: f32,
    ) -> Result<(), &'static str> {
        if resistance <= 0.0 {
            return Err("Shunt resistance must be positive");
        }
        self.shunt_resistance[channel as usize] = resistance;
        Ok(())
    }
}

impl Default for VcpConfig {
    fn default() -> Self {
        Self {
            limits: [VcpLimits::default(); 3],
            shunt_resistance: [positive_f32!(0.1); 3],
            initial_mode: VcpMode::Continuous,
        }
    }
}

pub struct VcpSensorsRunner<
    'a,
    SharedI2cDevice,
    const READING_CONSUMERS: usize,
    const EVENT_QUEUE_SIZE: usize,
> {
    i2c_dev: Option<SharedI2cDevice>,
    readings: &'a VcpCurrentReading<READING_CONSUMERS>,
    event_sender: Sender<'a, CriticalSectionRawMutex, VcpSensorsEvents, EVENT_QUEUE_SIZE>,
    command_sender: Receiver<'a, CriticalSectionRawMutex, VcpCommand, 1>,
    config: VcpConfig,
}

pub struct VcpControl<'a, const READING_CONSUMERS: usize, const EVENT_QUEUE_SIZE: usize> {
    readings: &'a VcpCurrentReading<READING_CONSUMERS>,
    event_receiver: Receiver<'a, CriticalSectionRawMutex, VcpSensorsEvents, EVENT_QUEUE_SIZE>,
    command_receiver: Sender<'a, CriticalSectionRawMutex, VcpCommand, 1>,
}

#[derive(Debug, Copy, Clone)]
pub struct VcpSensorsService(());

impl VcpSensorsService {
    /// Creates a new VCP sensors instance
    #[allow(clippy::new_ret_no_self)]
    pub fn new<
        'a,
        SharedI2cDevice,
        const READING_CONSUMERS: usize,
        const EVENT_QUEUE_SIZE: usize,
    >(
        i2c_dev: SharedI2cDevice,
        state: &'a mut VcpSensorsState<{ READING_CONSUMERS }, { EVENT_QUEUE_SIZE }>,
        config: VcpConfig,
    ) -> (
        VcpSensorsRunner<'a, SharedI2cDevice, { READING_CONSUMERS }, { EVENT_QUEUE_SIZE }>,
        VcpControl<'a, { READING_CONSUMERS }, { EVENT_QUEUE_SIZE }>,
    ) {
        (
            VcpSensorsRunner {
                i2c_dev: Some(i2c_dev),
                readings: &state.readings,
                event_sender: state.events.sender(),
                command_sender: state.control.receiver(),
                config,
            },
            VcpControl {
                readings: &state.readings,
                event_receiver: state.events.receiver(),
                command_receiver: state.control.sender(),
            },
        )
    }
}

impl<'a, SharedI2cDevice, const READING_CONSUMERS: usize, const EVENT_QUEUE_SIZE: usize>
    VcpSensorsRunner<'a, SharedI2cDevice, READING_CONSUMERS, EVENT_QUEUE_SIZE>
where
    SharedI2cDevice: embedded_hal_async::i2c::I2c,
    <SharedI2cDevice as embedded_hal_async::i2c::ErrorType>::Error: defmt::Format,
{
    async fn read_bus_voltage(
        &mut self,
        ina: &INA3221Async<SharedI2cDevice>,
        channel: u8,
    ) -> VcpState {
        match ina.get_bus_voltage(channel).await {
            Err(e) => {
                error!("INA3221 bus voltage read error: {:?}", e);
                VcpState::Error
            }
            Ok(voltage) => {
                if voltage.volts() < self.config.limits[channel as usize].min_voltage {
                    VcpState::Low(voltage.volts())
                } else if voltage.volts() > self.config.limits[channel as usize].max_voltage {
                    VcpState::High(voltage.volts())
                } else {
                    VcpState::Normal(voltage.volts())
                }
            }
        }
    }

    async fn read_shunt_voltage(
        &mut self,
        ina: &INA3221Async<SharedI2cDevice>,
        channel: u8,
    ) -> VcpState {
        match ina.get_shunt_voltage(channel).await {
            Err(e) => {
                error!("INA3221 shunt voltage read error: {:?}", e);
                VcpState::Error
            }
            Ok(shunt_voltage) => {
                let shunt_resistance = self.config.shunt_resistance(channel);
                let shunt_current = shunt_voltage.volts() / shunt_resistance;
                if shunt_current < self.config.limits[channel as usize].min_current {
                    VcpState::Low(shunt_current)
                } else if shunt_current > self.config.limits[channel as usize].max_current {
                    VcpState::High(shunt_current)
                } else {
                    VcpState::Normal(shunt_current)
                }
            }
        }
    }

    async fn read_channel(
        &mut self,
        ina: &INA3221Async<SharedI2cDevice>,
        channel: u8,
    ) -> VcpReading {
        // Placeholder for actual channel reading logic
        VcpReading {
            voltage: self.read_bus_voltage(ina, channel).await,
            current: self.read_shunt_voltage(ina, channel).await,
            channel,
        }
    }

    async fn read_all(&mut self, ina: &INA3221Async<SharedI2cDevice>) -> [VcpReading; 3] {
        let ch0 = self.read_channel(ina, 0).await;
        let ch1 = self.read_channel(ina, 1).await;
        let ch2 = self.read_channel(ina, 2).await;
        [ch0, ch1, ch2]
    }

    pub async fn run(&mut self) -> ! {
        let i2c_dev = self.i2c_dev.take().expect("I2C device already taken");
        // Initialize the sensors here using self.i2c_dev
        let mut ina: INA3221Async<SharedI2cDevice> = INA3221Async::new(i2c_dev, 0x40);

        let mut ticker = Ticker::every(40.ms());

        loop {
            // The sensor reading and processing logic here
            let readings = self.read_all(&ina).await;
            self.readings.sender().send(readings);
            ticker.next().await;
        }
    }
}
