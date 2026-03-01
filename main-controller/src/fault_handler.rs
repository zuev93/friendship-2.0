use core::sync::atomic::Ordering;
use cortex_m_rt::ExceptionFrame;

#[cortex_m_rt::exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    let cfsr = core::ptr::read_volatile(0xE000_ED28 as *const u32);
    let hfsr = core::ptr::read_volatile(0xE000_ED2C as *const u32);
    let mmfar = core::ptr::read_volatile(0xE000_ED34 as *const u32);
    let bfar = core::ptr::read_volatile(0xE000_ED38 as *const u32);
    let uptime = crate::runtime_stats::UPTIME_SECS.load(Ordering::Relaxed);

    crate::crash_info::write_fault(ef.pc(), ef.lr(), cfsr, hfsr, mmfar, bfar, uptime);

    cortex_m::peripheral::SCB::sys_reset();
}
