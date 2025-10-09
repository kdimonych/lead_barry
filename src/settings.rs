use crate::flash_storage::*;
use defmt::*;
use serde::{Deserialize, Serialize};

const CRC_SUM_SIZE: usize = 4; // Size of CRC32 checksum in bytes§

#[derive(defmt::Format)]
pub enum Error {
    StorageRead(embassy_rp::flash::Error),
    StorageErase(embassy_rp::flash::Error),
    StorageWrite(embassy_rp::flash::Error),
    StorageCrcWrite(embassy_rp::flash::Error),
    Serialization,
    Deserialization,
    Crc,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Settings {
    pub wifi_ssid: heapless::String<32>,
    pub wifi_password: heapless::String<64>,
    pub settings_version: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wifi_ssid: heapless::String::new(),
            wifi_password: heapless::String::new(),
            settings_version: 1,
        }
    }
}

impl Settings {
    const fn new() -> Self {
        Self {
            wifi_ssid: heapless::String::new(),
            wifi_password: heapless::String::new(),
            settings_version: 1,
        }
    }

    fn deserialize(data: &[u8]) -> Result<(Self, &[u8]), postcard::Error> {
        postcard::take_from_bytes(data)
    }

    fn serialize_to_slice(&self, buffer: &mut [u8]) -> Result<usize, postcard::Error> {
        let used = postcard::to_slice(self, buffer)?;
        Ok(used.len())
    }

    pub fn save(&self, storage: &mut Storage) -> Result<(), Error> {
        let mut buffer = [0u8; Storage::storage_size() - 4]; // Reserve 4 bytes for checksum
        let used = self
            .serialize_to_slice(&mut buffer)
            .map_err(|_| Error::Serialization)?;

        storage.blocking_erase().map_err(Error::StorageErase)?;
        storage
            .blocking_write(0, &buffer[..used])
            .map_err(Error::StorageWrite)?;

        // Write checksum
        let checksum = crc32_checksum(&buffer[..used]);
        let checksum_bytes = checksum.to_le_bytes();
        storage
            .blocking_write(used, &checksum_bytes)
            .map_err(Error::StorageCrcWrite)?;
        Ok(())
    }

    pub fn load(storage: &mut Storage) -> Result<Self, Error> {
        let mut buffer = [0u8; Storage::storage_size()];
        // Load entire storage into buffer
        storage
            .blocking_read(0, &mut buffer)
            .map_err(Error::StorageRead)?;

        let data_len = buffer.len() - CRC_SUM_SIZE;

        let (settings, crc_block) =
            Self::deserialize(&buffer[..data_len]).map_err(|_| Error::Deserialization)?;
        let parsed_block_size = buffer.len() - crc_block.len();

        if crc_block.len() < CRC_SUM_SIZE {
            error!("CRC block is too small");
            return Err(Error::Deserialization);
        }

        let stored_checksum =
            u32::from_le_bytes([crc_block[0], crc_block[1], crc_block[3], crc_block[4]]);

        // Verify checksum
        let calculated_checksum = crc32_checksum(&buffer[..parsed_block_size]);
        if stored_checksum != calculated_checksum {
            return Err(Error::Crc);
        }

        Ok(settings)
    }

    pub async fn load_async<'a>(storage: &mut Storage<'a>) -> Result<Self, Error> {
        let mut buffer = [0u8; Storage::storage_size()];
        // Load entire storage into buffer
        storage
            .background_read(0, &mut buffer)
            .await
            .map_err(Error::StorageRead)?;

        let data_len = buffer.len() - CRC_SUM_SIZE;

        let (settings, crc_block) =
            Self::deserialize(&buffer[..data_len]).map_err(|_| Error::Deserialization)?;
        let parsed_block_size = buffer.len() - crc_block.len();

        if crc_block.len() < CRC_SUM_SIZE {
            return Err(Error::Deserialization);
        }

        let stored_checksum =
            u32::from_le_bytes([crc_block[0], crc_block[1], crc_block[3], crc_block[4]]);

        // Verify checksum
        let calculated_checksum = crc32_checksum(&buffer[..parsed_block_size]);
        if stored_checksum != calculated_checksum {
            return Err(Error::Crc);
        }

        Ok(settings)
    }
}

// TODO: Implement with hardware CRC32 if available
// Simple CRC32 implementation
fn crc32_checksum(data: &[u8]) -> u32 {
    const CRC32_POLYNOMIAL: u32 = 0xEDB88320;
    let mut crc = 0xFFFFFFFF;

    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC32_POLYNOMIAL;
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}
