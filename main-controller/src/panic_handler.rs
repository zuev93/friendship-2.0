use core::panic::PanicInfo;
use core::sync::atomic::Ordering;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let uptime = crate::runtime_stats::UPTIME_SECS.load(Ordering::Relaxed);

    if let Some(location) = info.location() {
        crate::crash_info::write_panic(location.file().as_bytes(), location.line(), uptime);
    } else {
        crate::crash_info::write_panic(b"<unknown>", 0, uptime);
    }

    cortex_m::peripheral::SCB::sys_reset();
}
