use embassy_executor::Spawner;

use crate::{
    configuration::ConfigurationStorage, global_types::I2c0Device, rtc::RtcDs3231Ref, shared_resources::SharedResources,
};

pub struct HttpServerContext {
    //TODO: Get rid of spawner
    spawner: Spawner,
    shared: &'static SharedResources,
}

impl HttpServerContext {
    pub fn new(spawner: Spawner, shared: &'static SharedResources) -> Self {
        Self { spawner, shared }
    }

    #[inline(always)]
    pub const fn spawner(&self) -> Spawner {
        self.spawner
    }

    #[inline(always)]
    pub const fn configuration_storage(&self) -> &'static ConfigurationStorage<'static> {
        self.shared.configuration_storage
    }

    pub const fn shared_resources(&self) -> &'static SharedResources {
        self.shared
    }

    pub const fn rtc(&self) -> &'static RtcDs3231Ref<I2c0Device<'static>> {
        self.shared.rtc
    }
}
