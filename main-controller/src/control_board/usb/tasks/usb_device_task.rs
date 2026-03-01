use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::usb::Driver;
use embassy_usb::UsbDevice;

type UsbDriver = Driver<'static, stm_peripherals::USB>;

#[embassy_executor::task]
pub async fn usb_device_task(mut usb: UsbDevice<'static, UsbDriver>) -> ! {
    usb.run().await
}
