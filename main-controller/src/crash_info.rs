use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::Watch;

const CRASH_MAGIC: u32 = 0xDEAD_BEEF;
const BKPSRAM_BASE: usize = 0x40036400;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum ResetReason {
    Unknown = 0,
    HardFault = 1,
    Panic = 2,
    Watchdog = 3,
}

impl ResetReason {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::HardFault,
            2 => Self::Panic,
            3 => Self::Watchdog,
            _ => Self::Unknown,
        }
    }
}

#[repr(C)]
struct CrashInfo {
    magic: u32,
    reset_reason: u8,
    _pad: [u8; 3],
    pc: u32,
    lr: u32,
    cfsr: u32,
    hfsr: u32,
    mmfar: u32,
    bfar: u32,
    panic_line: u32,
    panic_file: [u8; 64],
    uptime_secs: u32,
}

#[derive(Clone, Copy)]
pub struct CrashInfoSnapshot {
    pub reset_reason: u8,
    pub pc: u32,
    pub lr: u32,
    pub cfsr: u32,
    pub hfsr: u32,
    pub mmfar: u32,
    pub bfar: u32,
    pub panic_line: u32,
    pub panic_file: [u8; 64],
    pub uptime_secs: u32,
}

pub static LAST_CRASH: Watch<ThreadModeRawMutex, CrashInfoSnapshot, 2> = Watch::new();

fn crash_info_ptr() -> *mut CrashInfo {
    BKPSRAM_BASE as *mut CrashInfo
}

pub fn check_and_take() -> Option<CrashInfoSnapshot> {
    unsafe {
        let info = crash_info_ptr();
        let magic = core::ptr::read_volatile(&raw const (*info).magic);
        if magic != CRASH_MAGIC {
            return None;
        }

        let snapshot = CrashInfoSnapshot {
            reset_reason: core::ptr::read_volatile(&raw const (*info).reset_reason),
            pc: core::ptr::read_volatile(&raw const (*info).pc),
            lr: core::ptr::read_volatile(&raw const (*info).lr),
            cfsr: core::ptr::read_volatile(&raw const (*info).cfsr),
            hfsr: core::ptr::read_volatile(&raw const (*info).hfsr),
            mmfar: core::ptr::read_volatile(&raw const (*info).mmfar),
            bfar: core::ptr::read_volatile(&raw const (*info).bfar),
            panic_line: core::ptr::read_volatile(&raw const (*info).panic_line),
            panic_file: core::ptr::read_volatile(&raw const (*info).panic_file),
            uptime_secs: core::ptr::read_volatile(&raw const (*info).uptime_secs),
        };

        core::ptr::write_volatile(&raw mut (*info).magic, 0);

        Some(snapshot)
    }
}

pub fn write_fault(pc: u32, lr: u32, cfsr: u32, hfsr: u32, mmfar: u32, bfar: u32, uptime: u32) {
    unsafe {
        let info = crash_info_ptr();
        core::ptr::write_volatile(&raw mut (*info).reset_reason, ResetReason::HardFault as u8);
        core::ptr::write_volatile(&raw mut (*info).pc, pc);
        core::ptr::write_volatile(&raw mut (*info).lr, lr);
        core::ptr::write_volatile(&raw mut (*info).cfsr, cfsr);
        core::ptr::write_volatile(&raw mut (*info).hfsr, hfsr);
        core::ptr::write_volatile(&raw mut (*info).mmfar, mmfar);
        core::ptr::write_volatile(&raw mut (*info).bfar, bfar);
        core::ptr::write_volatile(&raw mut (*info).uptime_secs, uptime);
        core::ptr::write_volatile(&raw mut (*info).magic, CRASH_MAGIC);
    }
}

pub fn write_panic(file: &[u8], line: u32, uptime: u32) {
    unsafe {
        let info = crash_info_ptr();
        core::ptr::write_volatile(&raw mut (*info).reset_reason, ResetReason::Panic as u8);
        core::ptr::write_volatile(&raw mut (*info).panic_line, line);

        let mut buf = [0u8; 64];
        let len = file.len().min(64);
        buf[..len].copy_from_slice(&file[..len]);
        core::ptr::write_volatile(&raw mut (*info).panic_file, buf);

        core::ptr::write_volatile(&raw mut (*info).uptime_secs, uptime);
        core::ptr::write_volatile(&raw mut (*info).magic, CRASH_MAGIC);
    }
}

pub fn write_watchdog(uptime: u32) {
    unsafe {
        let info = crash_info_ptr();
        core::ptr::write_volatile(&raw mut (*info).reset_reason, ResetReason::Watchdog as u8);
        core::ptr::write_volatile(&raw mut (*info).uptime_secs, uptime);
        core::ptr::write_volatile(&raw mut (*info).magic, CRASH_MAGIC);
    }
}
