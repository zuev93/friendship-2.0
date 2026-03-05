#![no_std]

#[cfg(target_os = "none")]
pub type PlatformMutex = embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

#[cfg(not(target_os = "none"))]
pub type PlatformMutex = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

pub mod drivers;
pub mod error;
pub mod protocol_types;
pub mod spi_protocol;
