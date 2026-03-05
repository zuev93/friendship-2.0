use druzhba_common::drivers::wm8940::{Register, Wm8940};
use druzhba_common::error;
use embassy_executor::Spawner;
use embassy_stm32::i2c::{self, I2c};
use embassy_stm32::mode;

use druzhba_front_panel_controller::state::input::Wm8940Signal;

pub fn spawn_tasks(
    spawner: &Spawner,
    wm8940: Wm8940<I2c<'static, mode::Async, i2c::Master>>,
    wm8940_signal: &'static Wm8940Signal,
) {
    spawner.must_spawn(wm8940_task(wm8940, wm8940_signal));
}

#[embassy_executor::task]
async fn wm8940_task(
    mut wm8940: Wm8940<I2c<'static, mode::Async, i2c::Master>>,
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
                .set_volume(Register::DacVolume, config.dac_volume)
                .await
            {
                error::error("WM8940 DAC volume write failed").await;
                continue;
            }

            if let Err(_) = wm8940
                .set_volume(Register::AdcVolume, config.adc_volume)
                .await
            {
                error::error("WM8940 ADC volume write failed").await;
                continue;
            }
        } else {
            if let Err(_) = wm8940.power_down().await {
                error::error("WM8940 power down failed").await;
            }
        }
    }
}
