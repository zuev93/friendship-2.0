use druzhba_common::drivers::wm8940::Wm8940;
use druzhba_common::error;
use embassy_executor::Spawner;
use embassy_stm32::i2c::I2c;
use embassy_stm32::peripherals::{DMA1_CH5, DMA1_CH6, I2C1};

use crate::state::input::Wm8940Signal;

pub fn spawn_tasks(
    spawner: &Spawner,
    wm8940: Wm8940<I2c<'static, I2C1, DMA1_CH6, DMA1_CH5>>,
    wm8940_signal: &'static Wm8940Signal,
) {
    spawner.must_spawn(wm8940_task(wm8940, wm8940_signal));
}

#[embassy_executor::task]
async fn wm8940_task(
    mut wm8940: Wm8940<I2c<'static, I2C1, DMA1_CH6, DMA1_CH5>>,
    wm8940_signal: &'static Wm8940Signal,
) {
    if wm8940.init().await.is_err() {
        error::error("WM8940 initialization failed").await;
        return;
    }

    loop {
        let config = wm8940_signal.wait().await;

        if config.enable {
            if let Err(_) = wm8940
                .write_register(
                    druzhba_common::drivers::wm8940::Register::LeftDacVolume,
                    config.dac_volume_left as u16 | 0x100,
                )
                .await
            {
                error::error("WM8940 DAC left volume write failed").await;
                continue;
            }

            if let Err(_) = wm8940
                .write_register(
                    druzhba_common::drivers::wm8940::Register::RightDacVolume,
                    config.dac_volume_right as u16 | 0x100,
                )
                .await
            {
                error::error("WM8940 DAC right volume write failed").await;
                continue;
            }

            if let Err(_) = wm8940
                .write_register(
                    druzhba_common::drivers::wm8940::Register::LeftAdcVolume,
                    config.adc_volume_left as u16 | 0x100,
                )
                .await
            {
                error::error("WM8940 ADC left volume write failed").await;
                continue;
            }

            if let Err(_) = wm8940
                .write_register(
                    druzhba_common::drivers::wm8940::Register::RightAdcVolume,
                    config.adc_volume_right as u16 | 0x100,
                )
                .await
            {
                error::error("WM8940 ADC right volume write failed").await;
                continue;
            }
        } else {
            if let Err(_) = wm8940
                .write_register(druzhba_common::drivers::wm8940::Register::Power1, 0)
                .await
            {
                error::error("WM8940 power down failed").await;
            }
        }
    }
}
