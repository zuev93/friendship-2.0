/*
 * AD9834 DDS (Direct Digital Synthesizer) Driver (async with DMA)
 */
use embassy_time::Timer;
use embedded_hal_async::spi::SpiBus;

const REG_CONTROL: u16 = 0x0000;

const CTRL_B28: u16 = 1 << 13; // 28-bit frequency load (1 = two 14-bit writes)
const CTRL_RESET: u16 = 1 << 8; // Reset internal registers
const CTRL_SLEEP1: u16 = 1 << 7; // Power down DAC
const CTRL_SLEEP12: u16 = 1 << 6; // Power down internal clock
const CTRL_OPBITEN: u16 = 1 << 5; // DAC power down control
const CTRL_SIGN_PIB: u16 = 1 << 4; // Sign bit output (MSB/2 of DAC data)
const CTRL_DIV2: u16 = 1 << 3; // Divide MCLK by 2 (0 = disable divider, 1 = enable)
const CTRL_MODE: u16 = 1 << 1; // Output mode (0 = sine, 1 = triangle)

const REG_FREQ0: u16 = 0x4000; // FREQ0 register (bits D15-D14 = 01)
const REG_FREQ1: u16 = 0x8000; // FREQ1 register (bits D15-D14 = 10)

pub struct AD9834Config {
    pub reference_clock_hz: u32,
    pub enable_doubler: bool,
}

pub struct AD9834 {
    control_reg: u16,
    effective_clock_hz: u32,
}

impl AD9834 {
    pub fn new(config: AD9834Config) -> Self {
        let effective_clock_hz = if config.enable_doubler {
            config.reference_clock_hz
        } else {
            config.reference_clock_hz / 2
        };

        let mut control_reg = CTRL_B28; // Default: 28-bit mode, FREQ0, PHASE0, sine wave

        if !config.enable_doubler {
            control_reg |= CTRL_DIV2;
        }

        Self {
            control_reg,
            effective_clock_hz,
        }
    }

    pub async fn init<SPI: SpiBus>(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        self.reset(spi).await?;
        Ok(())
    }

    pub async fn reset<SPI: SpiBus>(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        self.control_reg |= CTRL_RESET;
        self.write_register(spi, REG_CONTROL | self.control_reg)
            .await?;

        Timer::after_micros(100).await;

        self.control_reg &= !CTRL_RESET;
        self.write_register(spi, REG_CONTROL | self.control_reg)
            .await?;
        Ok(())
    }

    async fn write_register<SPI: SpiBus>(
        &mut self,
        spi: &mut SPI,
        value: u16,
    ) -> Result<(), SPI::Error> {
        let data = [
            (value >> 8) as u8, // MSB first
            (value & 0xFF) as u8,
        ];
        spi.write(&data).await?;
        Ok(())
    }

    fn calculate_tuning_word(&self, frequency_hz: u32) -> u32 {
        let freq_64 = frequency_hz as u64;
        let clock_64 = self.effective_clock_hz as u64;

        let tuning_word = (freq_64 * 268_435_456u64) / clock_64;

        (tuning_word & 0x0FFF_FFFF) as u32
    }

    pub async fn set_frequency<SPI: SpiBus>(
        &mut self,
        spi: &mut SPI,
        frequency_hz: u32,
    ) -> Result<(), SPI::Error> {
        self.set_frequency_register(spi, 0, frequency_hz).await?;
        Ok(())
    }

    pub async fn set_frequency_register<SPI: SpiBus>(
        &mut self,
        spi: &mut SPI,
        reg: u8,
        frequency_hz: u32,
    ) -> Result<(), SPI::Error> {
        let tuning_word = self.calculate_tuning_word(frequency_hz);

        let reg_addr = if reg == 0 { REG_FREQ0 } else { REG_FREQ1 };

        let lsb = (tuning_word & 0x3FFF) as u16; // Lower 14 bits
        let msb = ((tuning_word >> 14) & 0x3FFF) as u16; // Upper 14 bits

        self.write_register(spi, reg_addr | lsb).await?;
        self.write_register(spi, reg_addr | msb).await?;
        Ok(())
    }

    pub async fn set_waveform_sine<SPI: SpiBus>(
        &mut self,
        spi: &mut SPI,
    ) -> Result<(), SPI::Error> {
        self.control_reg &= !(CTRL_MODE | CTRL_OPBITEN | CTRL_SIGN_PIB);
        self.write_register(spi, REG_CONTROL | self.control_reg)
            .await?;
        Ok(())
    }

    pub async fn power_down<SPI: SpiBus>(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        self.control_reg |= CTRL_SLEEP1 | CTRL_SLEEP12;
        self.write_register(spi, REG_CONTROL | self.control_reg)
            .await?;
        Ok(())
    }

    pub async fn power_up<SPI: SpiBus>(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        self.control_reg &= !(CTRL_SLEEP1 | CTRL_SLEEP12);
        self.write_register(spi, REG_CONTROL | self.control_reg)
            .await?;
        Ok(())
    }
}
