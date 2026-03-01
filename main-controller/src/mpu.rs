// Stack overflow guard via MPU (PMSAv8, Cortex-M33).
//
// Marks the bottom 256 bytes of RAM (0x20000000..0x200000FF) as read-only.
// Stack grows downward from the top of RAM, so if it overflows into this
// region, a write will trigger a HardFault instead of silently corrupting data.

const MPU_CTRL: *mut u32 = 0xE000_ED94 as *mut u32;
const MPU_RNR: *mut u32 = 0xE000_ED98 as *mut u32;
const MPU_RBAR: *mut u32 = 0xE000_ED9C as *mut u32;
const MPU_RLAR: *mut u32 = 0xE000_EDA0 as *mut u32;
const MPU_MAIR0: *mut u32 = 0xE000_EDC0 as *mut u32;

pub fn init_stack_guard() {
    unsafe {
        core::ptr::write_volatile(MPU_CTRL, 0);

        core::ptr::write_volatile(MPU_MAIR0, 0x00);

        core::ptr::write_volatile(MPU_RNR, 0);
        core::ptr::write_volatile(MPU_RBAR, 0x2000_0000 | (0b10 << 1) | (1 << 0));
        core::ptr::write_volatile(MPU_RLAR, 0x2000_00E0 | (0 << 1) | 1);

        core::ptr::write_volatile(MPU_CTRL, 0b101);

        cortex_m::asm::dsb();
        cortex_m::asm::isb();
    }
}
