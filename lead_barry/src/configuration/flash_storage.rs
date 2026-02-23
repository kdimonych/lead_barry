#![allow(dead_code)]

use defmt_or_log as log;
use embassy_rp::Peri;
use embassy_rp::dma::Channel;
use embassy_rp::flash::{ASYNC_READ_SIZE, Async, ERASE_SIZE, Flash};
use embassy_rp::peripherals::FLASH;

unsafe extern "C" {
    static _user_flash_start: u32;
    static _user_flash_size: u32;
}

const FLASH_SIZE: usize = (2 * 1024 * 1024) as usize; // 2MB for flash (see memory.x for details)
const FLASH_BOOT_SIZE: usize = 0x100; // 256B for bootloader (see memory.x for details)
const FLASH_STORAGE_SIZE: usize = 0x1000; // 4KB for storage (see memory.x for details)
const FLASH_PG_SIZE: usize = FLASH_SIZE - FLASH_BOOT_SIZE - FLASH_STORAGE_SIZE; // Remain for program FLASH size

const FLASH_STORAGE_START_OFFSET: usize = FLASH_BOOT_SIZE + FLASH_PG_SIZE; // Start of storage area
const FLASH_STORAGE_END_OFFSET: usize = FLASH_STORAGE_START_OFFSET + FLASH_STORAGE_SIZE; // End of storage area

// Compile-time assertions to ensure flash layout is valid
const _: () = {
    // Ensure storage size is multiple of erase size
    assert!(
        FLASH_STORAGE_SIZE.is_multiple_of(ERASE_SIZE),
        "Storage size must be multiple of erase size"
    );

    // Ensure storage start is aligned to erase boundaries
    assert!(
        FLASH_STORAGE_START_OFFSET.is_multiple_of(ERASE_SIZE),
        "Storage start must be erase-aligned"
    );
    // Ensure storage start is aligned to async read boundaries
    assert!(
        FLASH_STORAGE_START_OFFSET.is_multiple_of(ASYNC_READ_SIZE),
        "Storage start must be async read-aligned"
    );

    // Ensure total flash layout doesn't exceed available flash
    assert!(
        FLASH_STORAGE_END_OFFSET <= FLASH_SIZE,
        "Flash layout exceeds available flash memory"
    );

    // Ensure storage size is reasonable (at least one erase block)
    assert!(
        FLASH_STORAGE_SIZE >= ERASE_SIZE,
        "Storage must be at least one erase block"
    );
};

type FlashType<'a> = Flash<'a, FLASH, Async, FLASH_SIZE>;

pub struct Storage<'a> {
    flash: FlashType<'a>,
}

pub fn get_user_flash_start() -> u32 {
    unsafe { &_user_flash_start as *const u32 as u32 }
}

pub fn get_user_flash_size() -> u32 {
    unsafe { &_user_flash_size as *const u32 as u32 }
}

impl<'a> Storage<'a> {
    pub fn new(flash_peripheral: Peri<'static, FLASH>, dma: Peri<'static, impl Channel>) -> Self {
        let flash = FlashType::new(flash_peripheral, dma);
        log::info!("Flash storage capacity:  size={:#X}", flash.capacity());
        Self { flash }
    }

    pub fn blocking_erase(&mut self) -> Result<(), embassy_rp::flash::Error> {
        // Erase the entire storage area
        for offset in (FLASH_STORAGE_START_OFFSET..FLASH_STORAGE_END_OFFSET).step_by(ERASE_SIZE) {
            self.flash.blocking_erase(offset as u32, (offset + ERASE_SIZE) as u32)?;
        }
        Ok(())
    }

    pub fn blocking_write(&mut self, offset: usize, data: &[u8]) -> Result<(), embassy_rp::flash::Error> {
        // Ensure offset and data length are within bounds
        if offset + data.len() > FLASH_STORAGE_SIZE {
            return Err(embassy_rp::flash::Error::OutOfBounds);
        }

        self.flash
            .blocking_write((FLASH_STORAGE_START_OFFSET + offset) as u32, data)?;

        Ok(())
    }

    pub async fn background_read(&mut self, offset: usize, buffer: &mut [u8]) -> Result<(), embassy_rp::flash::Error> {
        // Ensure offset and buffer length are within bounds
        if offset + buffer.len() > FLASH_STORAGE_SIZE {
            return Err(embassy_rp::flash::Error::OutOfBounds);
        }
        // Check alignment
        if !offset.is_multiple_of(ASYNC_READ_SIZE) || !(buffer.as_ptr() as usize).is_multiple_of(ASYNC_READ_SIZE) {
            return Err(embassy_rp::flash::Error::Unaligned);
        }

        let u32_buffer = bytemuck::cast_slice_mut::<u8, u32>(buffer);

        self.flash
            .background_read((FLASH_STORAGE_START_OFFSET + offset) as u32, u32_buffer)?
            .await;

        Ok(())
    }

    pub fn blocking_read(&mut self, offset: usize, buffer: &mut [u8]) -> Result<(), embassy_rp::flash::Error> {
        // Ensure offset and buffer length are within bounds
        if offset + buffer.len() > FLASH_STORAGE_SIZE {
            return Err(embassy_rp::flash::Error::OutOfBounds);
        }

        self.flash
            .blocking_read((FLASH_STORAGE_START_OFFSET + offset) as u32, buffer)?;

        Ok(())
    }

    pub const fn storage_size() -> usize {
        FLASH_STORAGE_SIZE
    }
}
