use crate::flash_storage::*;
use crc::{CRC_32_ISCSI, Crc};
use defmt::*;
use serde::{Deserialize, Serialize};

const CRC_SUM_SIZE: usize = 4; // Size of CRC32 checksum in bytes§

#[derive(defmt::Format)]
pub enum Error {
    StorageRead(embassy_rp::flash::Error),
    StorageErase(embassy_rp::flash::Error),
    StorageWrite(embassy_rp::flash::Error),
    Serialization,
    Deserialization,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StaticIpConfig {
    pub ip: [u8; 4],
    pub gateway: [u8; 4],
    pub subnet: [u8; 4],
    pub dns_servers: Option<[u8; 4]>, // Optional DNS server
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Settings {
    pub wifi_ssid: heapless::String<32>,
    pub wifi_password: heapless::String<64>,
    pub use_static_ip_config: bool,
    pub static_ip_config: Option<StaticIpConfig>,
    pub settings_version: u32,
}

impl Settings {
    const fn new() -> Self {
        Self {
            wifi_ssid: heapless::String::new(),
            wifi_password: heapless::String::new(),
            settings_version: 1,
            use_static_ip_config: false,
            static_ip_config: None,
        }
    }

    pub fn save(&self, storage: &mut Storage) -> Result<(), Error> {
        let mut buffer = [0u8; Storage::storage_size()]; // Reserve 4 bytes for checksum

        let crc = Crc::<u32>::new(&CRC_32_ISCSI);
        let used = postcard::to_slice_crc32(self, &mut buffer, crc.digest())
            .map_err(|_| Error::Serialization)?;

        debug!(
            "Used during save size: {} , \n\tdata: {:?}",
            used.len(),
            &used
        );

        storage.blocking_erase().map_err(Error::StorageErase)?;
        storage
            .blocking_write(0, used)
            .map_err(Error::StorageWrite)?;

        Ok(())
    }

    pub fn load(storage: &mut Storage) -> Result<Self, Error> {
        let mut buffer = [0u8; Storage::storage_size()];
        // Load entire storage into buffer
        storage
            .blocking_read(0, &mut buffer)
            .map_err(Error::StorageRead)?;

        let crc = Crc::<u32>::new(&CRC_32_ISCSI);
        postcard::from_bytes_crc32::<Self>(&buffer, crc.digest())
            .map_err(|_| Error::Deserialization)
    }

    pub async fn load_async<'a>(storage: &mut Storage<'a>) -> Result<Self, Error> {
        let mut buffer = [0u8; Storage::storage_size()];
        // Load entire storage into buffer
        storage
            .background_read(0, &mut buffer)
            .await
            .map_err(Error::StorageRead)?;

        let crc = Crc::<u32>::new(&CRC_32_ISCSI);
        postcard::from_bytes_crc32::<Self>(&buffer, crc.digest())
            .map_err(|_| Error::Deserialization)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl StaticIpConfig {
    pub const fn new() -> Self {
        Self {
            ip: [0, 0, 0, 0],
            gateway: [0, 0, 0, 0],
            subnet: [0, 0, 0, 0],
            dns_servers: Some([0, 0, 0, 0]),
        }
    }
}

impl Default for StaticIpConfig {
    fn default() -> Self {
        Self::new()
    }
}
