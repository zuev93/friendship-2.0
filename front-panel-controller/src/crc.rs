use core::cell::RefCell;
use druzhba_common::spi_protocol::Crc16;
use embassy_stm32::crc::{self, Crc, InputReverseConfig, PolySize};
use embassy_stm32::Peri;
use druzhba_common::PlatformMutex;
use embassy_sync::blocking_mutex::Mutex;

pub struct HardwareCrc16Modbus {
    crc: Mutex<PlatformMutex, RefCell<Crc<'static>>>,
}

impl HardwareCrc16Modbus {
    pub fn new(peripheral: Peri<'static, embassy_stm32::peripherals::CRC>) -> Self {
        let config = match crc::Config::new(
            InputReverseConfig::Byte,
            true,
            PolySize::Width16,
            0xFFFF,
            0x8005,
        ) {
            Ok(c) => c,
            Err(_) => panic!(),
        };

        Self {
            crc: Mutex::new(RefCell::new(Crc::new(peripheral, config))),
        }
    }
}

impl Crc16 for HardwareCrc16Modbus {
    fn calculate(&self, data: &[u8]) -> u16 {
        self.crc.lock(|cell| {
            let mut crc = cell.borrow_mut();
            crc.reset();
            crc.feed_bytes(data) as u16
        })
    }
}
