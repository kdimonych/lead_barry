use super::settings::*;
#[cfg(feature_use_static_ip_config)]
use crate::configuration::settings;
use crate::flash_storage::*;
use crc::{CRC_32_ISCSI, Crc};
use defmt::*;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use static_cell::StaticCell;

static SHARED_STORAGE: StaticCell<ConfigurationStorage<'static>> = StaticCell::new();

#[derive(defmt::Format, Debug)]
pub enum Error {
    StorageRead(embassy_rp::flash::Error),
    StorageErase(embassy_rp::flash::Error),
    StorageWrite(embassy_rp::flash::Error),
    Serialization,
    Deserialization,
}

pub struct ConfigurationStorageBuilder {
    flash_storage: Storage<'static>,
}

impl ConfigurationStorageBuilder {
    pub const fn new(flash_storage: Storage<'static>) -> Self {
        Self { flash_storage }
    }

    pub fn build(mut self) -> &'static ConfigurationStorage<'static> {
        let debug_settings_opt = debug_settings();

        if debug_settings_opt.is_some() {
            info!("Using debug settings from build configuration");
        }

        let initial_settings = match sync_load(&mut self.flash_storage) {
            Ok(settings) => settings,

            Err(error) => {
                error!(
                    "Can't load settings from storage: {}. Using default settings.",
                    error
                );
                let default_settings = debug_settings_opt.unwrap_or_default();
                if let Err(error) = sync_save(&mut self.flash_storage, &default_settings) {
                    error!("Can't save default settings to storage: {}", error);
                }
                default_settings
            }
        };

        #[cfg(all(feature_use_static_ip_config, feature_use_debug_settings))]
        let initial_settings = {
            defmt::info!("Overriding loaded settings with debug static IP config");
            debug_settings().unwrap_or(initial_settings)
        };

        let storage = SHARED_STORAGE.init(ConfigurationStorage::new(
            self.flash_storage,
            initial_settings,
        ));
        storage
    }
}

pub struct ConfigurationStorage<'a> {
    storage: Mutex<CriticalSectionRawMutex, StorageImpl<'a>>,
}

impl<'a> ConfigurationStorage<'a> {
    const fn new(flash_storage: Storage<'a>, initial_settings: Settings) -> Self {
        Self {
            storage: Mutex::new(StorageImpl::new(flash_storage, initial_settings)),
        }
    }

    /// Modify the current settings in cache asynchronously.
    /// - The modify_fn closure is called with a mutable reference to the settings which allow guarded modification of cached settings.
    pub async fn modify_settings<F>(&self, modify_fn: F)
    where
        F: FnOnce(&mut Settings),
    {
        let mut s = self.storage.lock().await;
        modify_fn(&mut s.settings_cache);
    }

    /// Set the current settings in cache asynchronously.
    pub async fn set_settings(&self, settings: Settings) {
        let mut s = self.storage.lock().await;
        s.settings_cache = settings;
    }

    /// Get a clone of the current settings from cache asynchronously.
    pub async fn get_settings(&self) -> Settings {
        let s = self.storage.lock().await;
        s.settings_cache.clone()
    }

    /// Load settings from flash storage asynchronously to the cache and return checked settings.
    pub async fn load(&self) -> Result<Settings, Error> {
        let mut storage = self.storage.lock().await;
        let mut buffer = [0u8; Storage::storage_size()];
        // Load entire storage into buffer

        storage
            .flash_storage
            .background_read(0, &mut buffer)
            .await
            .map_err(Error::StorageRead)?;

        let crc = Crc::<u32>::new(&CRC_32_ISCSI);
        storage.settings_cache = postcard::from_bytes_crc32::<Settings>(&buffer, crc.digest())
            .map_err(|_| Error::Deserialization)?;

        Ok(storage.settings_cache.clone())
    }

    /// Save settings from cache to flash storage asynchronously.
    pub async fn save(&self) -> Result<(), Error> {
        let mut storage = self.storage.lock().await;
        let mut buffer = [0u8; Storage::storage_size()]; // Reserve 4 bytes for checksum

        let crc = Crc::<u32>::new(&CRC_32_ISCSI);
        let used = postcard::to_slice_crc32(&storage.settings_cache, &mut buffer, crc.digest())
            .map_err(|_| Error::Serialization)?;

        debug!(
            "Used during save size: {} , \n\tdata: {:?}",
            used.len(),
            &used
        );

        storage
            .flash_storage
            .blocking_erase()
            .map_err(Error::StorageErase)?;
        storage
            .flash_storage
            .blocking_write(0, used)
            .map_err(Error::StorageWrite)?;

        Ok(())
    }
}

struct StorageImpl<'a> {
    settings_cache: Settings,
    flash_storage: Storage<'a>,
}

impl<'a> StorageImpl<'a> {
    pub const fn new(flash_storage: Storage<'a>, initial_settings: Settings) -> Self {
        Self {
            settings_cache: initial_settings,
            flash_storage,
        }
    }
}

fn sync_load(flash_storage: &mut Storage<'_>) -> Result<Settings, Error> {
    let mut buffer = [0u8; Storage::storage_size()];
    // Load entire storage into buffer
    flash_storage
        .blocking_read(0, &mut buffer)
        .map_err(Error::StorageRead)?;

    let crc = Crc::<u32>::new(&CRC_32_ISCSI);
    let settings = postcard::from_bytes_crc32::<Settings>(&buffer, crc.digest())
        .map_err(|_| Error::Deserialization)?;

    Ok(settings)
}

fn sync_save(flash_storage: &mut Storage<'_>, settings: &Settings) -> Result<(), Error> {
    let mut buffer = [0u8; Storage::storage_size()]; // Reserve 4 bytes for checksum

    let crc = Crc::<u32>::new(&CRC_32_ISCSI);
    let used = postcard::to_slice_crc32(settings, &mut buffer, crc.digest())
        .map_err(|_| Error::Serialization)?;

    debug!(
        "Used during sync save size: {} , \n\tdata: {:?}",
        used.len(),
        &used
    );

    flash_storage
        .blocking_erase()
        .map_err(Error::StorageErase)?;
    flash_storage
        .blocking_write(0, used)
        .map_err(Error::StorageWrite)?;

    Ok(())
}
