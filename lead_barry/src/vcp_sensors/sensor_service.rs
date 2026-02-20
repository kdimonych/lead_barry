use defmt_or_log as log;
use embassy_futures::*;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, SendFuture, Sender},
    priority_channel::{
        Max as MaxPriorityOrdering, PriorityChannel, ReceiveFuture, Receiver as PriorityReceiver,
        Sender as PrioritySender,
    },
};

use embassy_time::{Ticker, with_timeout};
use ina3221_async::*;

use crate::{
    units::TimeExt, vcp_sensors::config::*, vcp_sensors::data_model::*, vcp_sensors::error::*,
    vcp_sensors::events::*,
};

const POLL_TIMEOUT_MS: u64 = 40;
const HARDWARE_RESPONSE_TIMEOUT_MS: u64 = 100;

#[derive(Debug)]
#[defmt_or_log::derive_format_or_debug]
pub enum VcpCommand {
    EnableChannel(ChannelNum),
    DisableChannel(ChannelNum),
    EnableAllChannels,
    DisableAllChannels,
}

type VcpEventChannel<const EVENT_QUEUE_SIZE: usize> = PriorityChannel<
    CriticalSectionRawMutex,
    VcpSensorsEvents,
    MaxPriorityOrdering,
    EVENT_QUEUE_SIZE,
>;
pub type VcpEventReceiver<'a, const EVENT_QUEUE_SIZE: usize> = PriorityReceiver<
    'a,
    CriticalSectionRawMutex,
    VcpSensorsEvents,
    MaxPriorityOrdering,
    EVENT_QUEUE_SIZE,
>;
pub type VcpEventReceiveFuture<'a, const EVENT_QUEUE_SIZE: usize> = ReceiveFuture<
    'a,
    CriticalSectionRawMutex,
    VcpSensorsEvents,
    MaxPriorityOrdering,
    EVENT_QUEUE_SIZE,
>;

type VcpEventSender<'a, const EVENT_QUEUE_SIZE: usize> = PrioritySender<
    'a,
    CriticalSectionRawMutex,
    VcpSensorsEvents,
    MaxPriorityOrdering,
    EVENT_QUEUE_SIZE,
>;
type VcpCommandChannel = Channel<CriticalSectionRawMutex, VcpCommand, 1>;
type VcpCommandSendFuture<'a> = SendFuture<'a, CriticalSectionRawMutex, VcpCommand, 1>;

pub struct VcpSensorsState<const EVENT_QUEUE_SIZE: usize> {
    events: VcpEventChannel<EVENT_QUEUE_SIZE>,
    control: VcpCommandChannel,
}

impl<const EVENT_QUEUE_SIZE: usize> VcpSensorsState<EVENT_QUEUE_SIZE> {
    pub const fn new() -> Self {
        Self {
            events: VcpEventChannel::new(),
            control: VcpCommandChannel::new(),
        }
    }
}

pub struct VcpSensorsRunner<'a, SharedI2cDevice, const EVENT_QUEUE_SIZE: usize> {
    i2c_dev: Option<SharedI2cDevice>,
    event_sender: VcpEventSender<'a, EVENT_QUEUE_SIZE>,
    command_sender: Receiver<'a, CriticalSectionRawMutex, VcpCommand, 1>,
    config: VcpConfig,
}

pub struct VcpControl<'a, const EVENT_QUEUE_SIZE: usize> {
    event_receiver: VcpEventReceiver<'a, EVENT_QUEUE_SIZE>,
    command_receiver: Sender<'a, CriticalSectionRawMutex, VcpCommand, 1>,
}

#[allow(dead_code)]
impl<'a, const EVENT_QUEUE_SIZE: usize> VcpControl<'a, EVENT_QUEUE_SIZE> {
    pub fn receive_event(&self) -> VcpEventReceiveFuture<'_, EVENT_QUEUE_SIZE> {
        self.event_receiver.receive()
    }

    pub fn flush_events(&self) {
        while self.event_receiver.try_receive().is_ok() {}
    }

    pub fn enable_channel(&self, channel: ChannelNum) -> VcpCommandSendFuture<'_> {
        self.command_receiver
            .send(VcpCommand::EnableChannel(channel))
    }

    pub fn disable_channel(&self, channel: ChannelNum) -> VcpCommandSendFuture<'_> {
        self.command_receiver
            .send(VcpCommand::DisableChannel(channel))
    }
    pub fn enable_all_channels(&self) -> impl Future<Output = ()> + '_ {
        self.command_receiver.send(VcpCommand::EnableAllChannels)
    }
    pub fn disable_all_channels(&self) -> impl Future<Output = ()> + '_ {
        self.command_receiver.send(VcpCommand::DisableAllChannels)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VcpSensorsService(());

impl VcpSensorsService {
    /// Creates a new VCP sensors instance
    #[allow(clippy::new_ret_no_self)]
    pub fn new<'a, SharedI2cDevice, const EVENT_QUEUE_SIZE: usize>(
        i2c_dev: SharedI2cDevice,
        state: &'a mut VcpSensorsState<{ EVENT_QUEUE_SIZE }>,
        config: VcpConfig,
    ) -> (
        VcpSensorsRunner<'a, SharedI2cDevice, { EVENT_QUEUE_SIZE }>,
        VcpControl<'a, { EVENT_QUEUE_SIZE }>,
    ) {
        (
            VcpSensorsRunner {
                i2c_dev: Some(i2c_dev),
                event_sender: state.events.sender(),
                command_sender: state.control.receiver(),
                config,
            },
            VcpControl {
                event_receiver: state.events.receiver(),
                command_receiver: state.control.sender(),
            },
        )
    }
}

impl<'a, SharedI2cDevice, const EVENT_QUEUE_SIZE: usize>
    VcpSensorsRunner<'a, SharedI2cDevice, EVENT_QUEUE_SIZE>
where
    SharedI2cDevice: embedded_hal_async::i2c::I2c,
{
    async fn read_bus_voltage(
        &mut self,
        ina: &INA3221Async<SharedI2cDevice>,
        channel: u8,
    ) -> Result<VcpState, VcpError> {
        match ina.get_bus_voltage(channel).await {
            Err(e) => {
                log::error!(
                    "INA3221 bus voltage read error: {:?}",
                    defmt_or_log::Debug2Format(&e)
                );
                Err(VcpError::I2c)
            }
            Ok(voltage) => {
                if voltage.volts() < self.config.limits[channel as usize].min_voltage {
                    Ok(VcpState::Low(voltage.volts()))
                } else if voltage.volts() > self.config.limits[channel as usize].max_voltage {
                    Ok(VcpState::High(voltage.volts()))
                } else {
                    Ok(VcpState::Normal(voltage.volts()))
                }
            }
        }
    }

    async fn read_shunt_voltage(
        &mut self,
        ina: &INA3221Async<SharedI2cDevice>,
        channel: u8,
    ) -> Result<VcpState, VcpError> {
        match ina.get_shunt_voltage(channel).await {
            Err(e) => {
                log::error!(
                    "INA3221 shunt voltage read error: {:?}",
                    defmt_or_log::Debug2Format(&e)
                );
                Err(VcpError::I2c)
            }
            Ok(shunt_voltage) => {
                let shunt_resistance = self.config.shunt_resistance(channel);
                let shunt_current = shunt_voltage.volts() / shunt_resistance;
                if shunt_current < self.config.limits[channel as usize].min_current {
                    Ok(VcpState::Low(shunt_current))
                } else if shunt_current > self.config.limits[channel as usize].max_current {
                    Ok(VcpState::High(shunt_current))
                } else {
                    Ok(VcpState::Normal(shunt_current))
                }
            }
        }
    }

    async fn read_channel(
        &mut self,
        ina: &INA3221Async<SharedI2cDevice>,
        channel: u8,
    ) -> Result<VcpReading, VcpError> {
        let voltage = self.read_bus_voltage(ina, channel).await?;
        let current = self.read_shunt_voltage(ina, channel).await?;
        Ok(VcpReading {
            voltage,
            current,
            channel,
        })
    }

    async fn configure(&mut self, ina: &mut INA3221Async<SharedI2cDevice>) -> Result<(), VcpError> {
        // Set operating mode to continuous
        ina.set_mode(OperatingMode::Continuous).await.map_err(|e| {
            log::error!(
                "INA3221 set mode error: {:?}",
                defmt_or_log::Debug2Format(&e)
            );
            VcpError::I2c
        })?;

        // Enable selected channels
        for (i, enable) in self.config.enabled_channels.iter().enumerate() {
            ina.set_channel_enabled(i as u8, *enable)
                .await
                .map_err(|e| {
                    log::error!(
                        "INA3221 set channel {} enabled error: {:?}",
                        i,
                        defmt_or_log::Debug2Format(&e)
                    );
                    VcpError::I2c
                })?;
        }

        Ok(())
    }

    fn handle_command(&mut self, _: &mut INA3221Async<SharedI2cDevice>, command: VcpCommand) {
        match command {
            VcpCommand::EnableChannel(channel) => {
                if (channel as usize) < self.config.enabled_channels.len() {
                    self.config.enabled_channels[channel as usize] = true;
                    log::info!("Enabled channel {}", channel);
                } else {
                    log::warn!("Invalid channel number: {}", channel);
                }
            }
            VcpCommand::DisableChannel(channel) => {
                if (channel as usize) < self.config.enabled_channels.len() {
                    self.config.enabled_channels[channel as usize] = false;
                    log::info!("Disabled channel {}", channel);
                } else {
                    log::warn!("Invalid channel number: {}", channel);
                }
            }
            VcpCommand::EnableAllChannels => {
                for ch in 0u8..3u8 {
                    self.config.enabled_channels[ch as usize] = true;
                }
                log::info!("Enabled all channels");
            }
            VcpCommand::DisableAllChannels => {
                for ch in 0u8..3u8 {
                    self.config.enabled_channels[ch as usize] = false;
                }
                log::info!("Disabled all channels");
            }
        }
    }

    fn push_event(&mut self, event: VcpSensorsEvents) {
        if self.event_sender.is_full() {
            // If data queue is full, clear it to make space for new readings
            self.event_sender.clear();
        }
        if self.event_sender.try_send(event).is_err() {
            log::error!("Failed to send VCP event");
        }
    }

    pub async fn run(&mut self) -> ! {
        let i2c_dev = self.i2c_dev.take().expect("I2C device already taken");
        // Initialize the sensors here using self.i2c_dev
        let mut ina: INA3221Async<SharedI2cDevice> = INA3221Async::new(i2c_dev, 0x40);

        // Configure the INA3221
        if let Err(e) = self.configure(&mut ina).await {
            log::error!("Failed to configure INA3221: {:?}", e);
            self.event_sender.send(VcpSensorsEvents::Error(e)).await;
        }

        let mut ticker = Ticker::every(POLL_TIMEOUT_MS.ms());

        loop {
            match select::select(self.command_sender.receive(), ticker.next()).await {
                select::Either::First(command) => {
                    // Handle incoming command
                    log::debug!("Handled VCP command: {}", command);
                    self.handle_command(&mut ina, command);
                }
                select::Either::Second(_) => {}
            }
            // The sensor reading and processing logic here
            for ch in 0u8..3u8 {
                if !self.config.enabled_channels[ch as usize] {
                    continue;
                }
                let reading = with_timeout(
                    HARDWARE_RESPONSE_TIMEOUT_MS.ms(),
                    self.read_channel(&ina, ch),
                )
                .await;
                match reading {
                    Err(_) => {
                        log::error!("Timeout reading channel {}", ch);
                        self.push_event(VcpSensorsEvents::Error(VcpError::Timeout));
                        continue;
                    }
                    Ok(Err(e)) => {
                        log::error!("Error reading channel {}: {:?}", ch, e);
                        self.push_event(VcpSensorsEvents::Error(e));
                        continue;
                    }
                    Ok(Ok(reading)) => self.push_event(VcpSensorsEvents::Reading(reading)),
                };
            }
        }
    }
}
