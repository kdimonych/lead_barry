#[derive(Debug, Copy, Clone)]
#[defmt_or_log::derive_format_or_debug]
pub enum VcpError {
    I2c,
    Timeout,
}
