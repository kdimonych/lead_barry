use embassy_executor::Spawner;

use crate::configuration::ConfigurationStorage;

pub struct HttpServerContext {
    spawner: Spawner,
    configuration_storage: &'static ConfigurationStorage<'static>,
}

impl HttpServerContext {
    pub fn new(
        spawner: Spawner,
        configuration_storage: &'static ConfigurationStorage<'static>,
    ) -> Self {
        Self {
            spawner,
            configuration_storage,
        }
    }

    #[inline(always)]
    pub const fn spawner(&self) -> Spawner {
        self.spawner
    }

    #[inline(always)]
    pub const fn configuration_storage(&self) -> &'static ConfigurationStorage<'static> {
        self.configuration_storage
    }
}
