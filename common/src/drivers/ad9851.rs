/*
 * AD9851 DDS (Direct Digital Synthesizer) Driver (async with DMA)
 */

use embassy_time::Timer;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiBus;

pub struct AD9851Config {
    pub reference_clock_hz: u32,
    pub refclk_multiplier: u8,
}

impl Default for AD9851Config {
    fn default() -> Self {
        Self {
            reference_clock_hz: 20_000_000, // 20 MHz TCXO
            refclk_multiplier: 6,           // 6x multiplier = 120 MHz system clock
        }
    }
}

pub struct AD9851<FQ, RST>
where
    FQ: OutputPin,
    RST: OutputPin,
{
    fq_ud: FQ,
    reset: RST,
    system_clock_hz: u32,
    refclk_multiplier: u8,
}

impl<FQ, RST> AD9851<FQ, RST>
where
    FQ: OutputPin,
    RST: OutputPin,
{
    pub fn new(fq_ud: FQ, reset: RST, config: AD9851Config) -> Self {
        let system_clock_hz = config.reference_clock_hz * config.refclk_multiplier as u32;

        Self {
            fq_ud,
            reset,
            system_clock_hz,
            refclk_multiplier: config.refclk_multiplier,
        }
    }

    pub async fn init<SPI: SpiBus>(&mut self, spi: &mut SPI) -> Result<(), &'static str> {
        self.fq_ud
            .set_low()
            .map_err(|_| "Failed to set FQ_UD low")?;
        self.reset
            .set_low()
            .map_err(|_| "Failed to set RESET low")?;

        self.reset(spi)
            .await
            .map_err(|_| "Failed to reset AD9851")?;
        Ok(())
    }

    pub async fn reset<SPI: SpiBus>(&mut self, _spi: &mut SPI) -> Result<(), RST::Error> {
        self.reset.set_high()?;
        Timer::after_micros(10).await; // Min 3us, use 10us to be safe
        self.reset.set_low()?;
        Timer::after_micros(5).await; // Min 1us
        Ok(())
    }

    fn pulse_fq_ud(&mut self) -> Result<(), FQ::Error> {
        self.fq_ud.set_high()?;
        self.fq_ud.set_low()?;
        Ok(())
    }

    fn calculate_tuning_word(&self, frequency_hz: u32) -> u32 {
        let freq_64 = frequency_hz as u64;
        let sysclk_64 = self.system_clock_hz as u64;

        let tuning_word = (freq_64 * 4_294_967_296u64) / sysclk_64;
        tuning_word as u32
    }

    #[allow(dead_code)]
    pub async fn set_frequency<SPI: SpiBus>(
        &mut self,
        spi: &mut SPI,
        frequency_hz: u32,
    ) -> Result<(), SPI::Error> {
        let tuning_word = self.calculate_tuning_word(frequency_hz);

        let mut data = [0u8; 5];

        data[0] = (tuning_word & 0xFF) as u8;
        data[1] = ((tuning_word >> 8) & 0xFF) as u8;
        data[2] = ((tuning_word >> 16) & 0xFF) as u8;
        data[3] = ((tuning_word >> 24) & 0xFF) as u8;

        let multiplier_bits = match self.refclk_multiplier {
            1 => 0b00,
            2 => 0b01,
            4 => 0b10,
            6 => 0b11,
            _ => 0b11, // Default to 6x
        };
        data[4] = multiplier_bits << 2;

        spi.write(&data).await?;

        let _ = self.pulse_fq_ud();

        Ok(())
    }
}
