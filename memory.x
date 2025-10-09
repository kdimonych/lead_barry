MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    /* Program flash - 1.75MB */
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100 - 4K

    /* User data storage area - last 4KB of flash */
    USER_FLASH : ORIGIN = 0x101FF000, LENGTH = 4K

    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}

_user_flash_start = ORIGIN(USER_FLASH);
_user_flash_size = LENGTH(USER_FLASH);

EXTERN(BOOT2_FIRMWARE)

SECTIONS {
    /* ### Boot loader */
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2

        /* ### User flash storage - zero initialized */
    .user_data _user_flash_start : AT(_user_flash_start)
    {
        . = ALIGN(4);
        _user_data_start = .;
        . = . + LENGTH(USER_FLASH);
        _user_data_end = .;
    } > USER_FLASH = 0x00
} INSERT BEFORE .text;
