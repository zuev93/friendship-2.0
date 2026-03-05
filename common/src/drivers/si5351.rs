use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

const REG_DEVICE_STATUS: u8 = 0;
const REG_OUTPUT_ENABLE: u8 = 3;
const REG_CLK0_CONTROL: u8 = 16;
const REG_CLK1_CONTROL: u8 = 17;
const REG_MSNA_BASE: u8 = 26;
const REG_MSNB_BASE: u8 = 34;
const REG_MS0_BASE: u8 = 42;
const REG_MS1_BASE: u8 = 50;
const REG_PLL_RESET: u8 = 177;
const REG_CRYSTAL_LOAD: u8 = 183;

const CRYSTAL_LOAD_8PF: u8 = 0b01_000000;
const PLL_RESET_A: u8 = 0x20;
const PLL_RESET_B: u8 = 0x80;

const XTAL_FREQ: u32 = 25_000_000;
const PLL_VCO_MIN: u32 = 600_000_000;
const PLL_VCO_MAX: u32 = 900_000_000;
const MULTISYNTH_DIVIDER_MAX: u32 = 900;
const MULTISYNTH_DIVIDER_MIN: u32 = 6;
const MULTISYNTH_C_MAX: u32 = 1_048_575;

#[derive(Clone, Copy, PartialEq)]
pub enum PllSource {
    PllA,
    PllB,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ClkOutput {
    Clk0,
    Clk1,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DriveStrength {
    Drive2mA,
    Drive4mA,
    Drive6mA,
    Drive8mA,
}

impl DriveStrength {
    fn bits(self) -> u8 {
        match self {
            DriveStrength::Drive2mA => 0x00,
            DriveStrength::Drive4mA => 0x01,
            DriveStrength::Drive6mA => 0x02,
            DriveStrength::Drive8mA => 0x03,
        }
    }
}

struct PllParams {
    a: u32,
    b: u32,
    c: u32,
}

struct MsParams {
    a: u32,
    b: u32,
    c: u32,
    r_div: u8,
}

pub struct Si5351<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    i2c: &'static Mutex<PlatformMutex, I2C>,
    plla_freq: u32,
    pllb_freq: u32,
    clk0_enabled: bool,
    clk1_enabled: bool,
    clk0_drive: DriveStrength,
    clk1_drive: DriveStrength,
}

impl<I2C> Si5351<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(address: u8, i2c: &'static Mutex<PlatformMutex, I2C>) -> Self {
        Self {
            address,
            i2c,
            plla_freq: 0,
            pllb_freq: 0,
            clk0_enabled: false,
            clk1_enabled: false,
            clk0_drive: DriveStrength::Drive8mA,
            clk1_drive: DriveStrength::Drive8mA,
        }
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.wait_for_ready().await?;

        self.write_reg(REG_OUTPUT_ENABLE, 0xFF).await?;

        self.write_reg(REG_CLK0_CONTROL, 0x80).await?;
        self.write_reg(REG_CLK1_CONTROL, 0x80).await?;
        self.write_reg(18, 0x80).await?;

        self.write_reg(REG_CRYSTAL_LOAD, CRYSTAL_LOAD_8PF).await?;

        Ok(())
    }

    pub async fn set_frequency(
        &mut self,
        pll: PllSource,
        output: ClkOutput,
        freq_hz: u32,
    ) -> Result<(), I2C::Error> {
        let target_vco = self.calculate_optimal_vco(freq_hz);

        let pll_params = self.calculate_pll_params(target_vco);
        let actual_vco = self.pll_frequency(&pll_params);

        let ms_params = self.calculate_ms_params(actual_vco, freq_hz);

        let (pll_base, pll_reset_bit) = match pll {
            PllSource::PllA => (REG_MSNA_BASE, PLL_RESET_A),
            PllSource::PllB => (REG_MSNB_BASE, PLL_RESET_B),
        };

        let old_pll_freq = match pll {
            PllSource::PllA => self.plla_freq,
            PllSource::PllB => self.pllb_freq,
        };

        self.write_pll_params(pll_base, &pll_params).await?;

        let ms_base = match output {
            ClkOutput::Clk0 => REG_MS0_BASE,
            ClkOutput::Clk1 => REG_MS1_BASE,
        };
        self.write_ms_params(ms_base, &ms_params).await?;

        let clk_reg = match output {
            ClkOutput::Clk0 => REG_CLK0_CONTROL,
            ClkOutput::Clk1 => REG_CLK1_CONTROL,
        };
        let drive = match output {
            ClkOutput::Clk0 => self.clk0_drive,
            ClkOutput::Clk1 => self.clk1_drive,
        };
        let pll_bit = match pll {
            PllSource::PllA => 0x00,
            PllSource::PllB => 0x20,
        };
        let clk_control = 0x0C | pll_bit | drive.bits();
        self.write_reg(clk_reg, clk_control).await?;

        if actual_vco != old_pll_freq {
            self.write_reg(REG_PLL_RESET, pll_reset_bit).await?;
            match pll {
                PllSource::PllA => self.plla_freq = actual_vco,
                PllSource::PllB => self.pllb_freq = actual_vco,
            }
        }

        match output {
            ClkOutput::Clk0 => self.clk0_enabled = true,
            ClkOutput::Clk1 => self.clk1_enabled = true,
        }
        self.update_output_enable().await?;

        Ok(())
    }

    pub async fn disable(&mut self, output: ClkOutput) -> Result<(), I2C::Error> {
        let clk_reg = match output {
            ClkOutput::Clk0 => REG_CLK0_CONTROL,
            ClkOutput::Clk1 => REG_CLK1_CONTROL,
        };
        self.write_reg(clk_reg, 0x80).await?;

        match output {
            ClkOutput::Clk0 => self.clk0_enabled = false,
            ClkOutput::Clk1 => self.clk1_enabled = false,
        }
        self.update_output_enable().await?;

        Ok(())
    }

    pub async fn set_drive_strength(
        &mut self,
        output: ClkOutput,
        strength: DriveStrength,
    ) -> Result<(), I2C::Error> {
        match output {
            ClkOutput::Clk0 => self.clk0_drive = strength,
            ClkOutput::Clk1 => self.clk1_drive = strength,
        }

        let clk_reg = match output {
            ClkOutput::Clk0 => REG_CLK0_CONTROL,
            ClkOutput::Clk1 => REG_CLK1_CONTROL,
        };
        let current = self.read_reg(clk_reg).await?;
        let updated = (current & 0xFC) | strength.bits();
        self.write_reg(clk_reg, updated).await?;

        Ok(())
    }

    fn calculate_optimal_vco(&self, freq_hz: u32) -> u32 {
        let mut best_vco = PLL_VCO_MAX;
        let mut best_remainder = u32::MAX;

        let div_min = PLL_VCO_MIN / freq_hz;
        let div_max = PLL_VCO_MAX / freq_hz;

        let start = if div_min < MULTISYNTH_DIVIDER_MIN {
            MULTISYNTH_DIVIDER_MIN
        } else {
            div_min
        };
        let end = if div_max > MULTISYNTH_DIVIDER_MAX {
            MULTISYNTH_DIVIDER_MAX
        } else {
            div_max
        };

        let mut div = start;
        while div <= end {
            let even_div = if div % 2 != 0 { div + 1 } else { div };
            if even_div > end {
                break;
            }
            let vco = freq_hz * even_div;
            if vco >= PLL_VCO_MIN && vco <= PLL_VCO_MAX {
                let remainder = vco % XTAL_FREQ;
                if remainder < best_remainder {
                    best_remainder = remainder;
                    best_vco = vco;
                    if remainder == 0 {
                        break;
                    }
                }
            }
            div = even_div + 2;
        }

        best_vco
    }

    fn calculate_pll_params(&self, target_vco: u32) -> PllParams {
        let a = target_vco / XTAL_FREQ;
        let remainder = target_vco % XTAL_FREQ;

        if remainder == 0 {
            return PllParams { a, b: 0, c: 1 };
        }

        let c = MULTISYNTH_C_MAX;
        let b = ((remainder as u64) * (c as u64) / (XTAL_FREQ as u64)) as u32;

        PllParams { a, b, c }
    }

    fn pll_frequency(&self, params: &PllParams) -> u32 {
        let freq =
            (XTAL_FREQ as u64) * (params.a as u64) + (XTAL_FREQ as u64) * (params.b as u64) / (params.c as u64);
        freq as u32
    }

    fn calculate_ms_params(&self, vco_freq: u32, target_freq: u32) -> MsParams {
        let mut r_div: u8 = 0;
        let mut actual_target = target_freq;

        if target_freq < 500_000 {
            let ratio = 500_000 / target_freq;
            let mut r = 1u32;
            let mut r_val = 0u8;
            while r < ratio && r_val < 7 {
                r *= 2;
                r_val += 1;
            }
            actual_target = target_freq * r;
            r_div = r_val;
        }

        let a = vco_freq / actual_target;
        let remainder = vco_freq % actual_target;

        if remainder == 0 {
            return MsParams {
                a,
                b: 0,
                c: 1,
                r_div,
            };
        }

        let c = MULTISYNTH_C_MAX;
        let b = ((remainder as u64) * (c as u64) / (actual_target as u64)) as u32;

        MsParams { a, b, c, r_div }
    }

    async fn write_pll_params(&self, base: u8, params: &PllParams) -> Result<(), I2C::Error> {
        let p1: u32 = 128 * params.a + ((128 * params.b) / params.c) - 512;
        let p2: u32 = 128 * params.b - params.c * ((128 * params.b) / params.c);
        let p3: u32 = params.c;

        let buf = [
            base,
            ((p3 >> 8) & 0xFF) as u8,
            (p3 & 0xFF) as u8,
            ((p1 >> 16) & 0x03) as u8,
            ((p1 >> 8) & 0xFF) as u8,
            (p1 & 0xFF) as u8,
            (((p3 >> 12) & 0xF0) | ((p2 >> 16) & 0x0F)) as u8,
            ((p2 >> 8) & 0xFF) as u8,
            (p2 & 0xFF) as u8,
        ];

        self.i2c.lock().await.write(self.address, &buf).await
    }

    async fn write_ms_params(&self, base: u8, params: &MsParams) -> Result<(), I2C::Error> {
        let p1: u32 = 128 * params.a + ((128 * params.b) / params.c) - 512;
        let p2: u32 = 128 * params.b - params.c * ((128 * params.b) / params.c);
        let p3: u32 = params.c;

        let r_div_bits = (params.r_div & 0x07) << 4;

        let buf = [
            base,
            ((p3 >> 8) & 0xFF) as u8,
            (p3 & 0xFF) as u8,
            (((p1 >> 16) & 0x03) as u8) | r_div_bits,
            ((p1 >> 8) & 0xFF) as u8,
            (p1 & 0xFF) as u8,
            (((p3 >> 12) & 0xF0) | ((p2 >> 16) & 0x0F)) as u8,
            ((p2 >> 8) & 0xFF) as u8,
            (p2 & 0xFF) as u8,
        ];

        self.i2c.lock().await.write(self.address, &buf).await
    }

    async fn wait_for_ready(&self) -> Result<(), I2C::Error> {
        for _ in 0..100u32 {
            let status = self.read_reg(REG_DEVICE_STATUS).await?;
            if status & 0x80 == 0 {
                return Ok(());
            }
            Timer::after_millis(10).await;
        }
        Ok(())
    }

    async fn update_output_enable(&self) -> Result<(), I2C::Error> {
        let mut val = 0xFF;
        if self.clk0_enabled {
            val &= !0x01;
        }
        if self.clk1_enabled {
            val &= !0x02;
        }
        self.write_reg(REG_OUTPUT_ENABLE, val).await
    }

    async fn write_reg(&self, reg: u8, val: u8) -> Result<(), I2C::Error> {
        self.i2c.lock().await.write(self.address, &[reg, val]).await
    }

    async fn read_reg(&self, reg: u8) -> Result<u8, I2C::Error> {
        let mut buf = [0u8];
        let mut i2c = self.i2c.lock().await;
        i2c.write(self.address, &[reg]).await?;
        i2c.read(self.address, &mut buf).await?;
        Ok(buf[0])
    }
}
