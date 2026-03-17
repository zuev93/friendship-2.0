use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embedded_hal_async::i2c::I2c;

#[repr(u8)]
pub enum Register {
    ModeControl1 = 0x01,
    DacControl = 0x02,
    DacVolumeA = 0x03,
    DacVolumeB = 0x04,
    DacMixing = 0x05,
    AdcControl = 0x06,
    ModeControl2 = 0x07,
    ChipId = 0x08,
}

const MODE_CTRL2_POWER_DOWN_CP_EN: u8 = 0x03;
const MODE_CTRL2_CP_EN: u8 = 0x02;

const MODE_CTRL1_SLAVE: u8 = 0x00;
const MODE_CTRL1_QUAD_SPEED: u8 = 0x03;
const MODE_CTRL1_RATIO_128: u8 = 0x02 << 5;

const DAC_I2S_24BIT: u8 = 0x09;
const ADC_I2S_24BIT: u8 = 0x00;

const DAC_CTRL_MUTE_A: u8 = 1 << 3;
const DAC_CTRL_MUTE_B: u8 = 1 << 4;

const ADC_CTRL_HPF_FREEZE: u8 = 1 << 5;

pub struct Cs4272<I2C>
where
    I2C: I2c + 'static,
{
    i2c_addr: u8,
    i2c: &'static Mutex<PlatformMutex, I2C>,
    dac_ctrl_cache: u8,
    adc_ctrl_cache: u8,
}

impl<I2C> Cs4272<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(i2c_addr: u8, i2c: &'static Mutex<PlatformMutex, I2C>) -> Self {
        Self {
            i2c_addr,
            i2c,
            dac_ctrl_cache: DAC_I2S_24BIT,
            adc_ctrl_cache: ADC_I2S_24BIT,
        }
    }

    pub async fn write_register(&mut self, reg: Register, value: u8) -> Result<(), I2C::Error> {
        let map_byte = reg as u8;
        self.i2c
            .lock()
            .await
            .write(self.i2c_addr, &[map_byte, value])
            .await
    }

    pub async fn read_register(&mut self, reg: Register) -> Result<u8, I2C::Error> {
        let map_byte = reg as u8;
        let mut buffer = [0u8; 1];
        self.i2c
            .lock()
            .await
            .write_read(self.i2c_addr, &[map_byte], &mut buffer)
            .await?;
        Ok(buffer[0])
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.write_register(Register::ModeControl2, MODE_CTRL2_POWER_DOWN_CP_EN)
            .await?;

        const SLAVE_QUAD_128: u8 = MODE_CTRL1_SLAVE | MODE_CTRL1_RATIO_128 | MODE_CTRL1_QUAD_SPEED;
        self.write_register(Register::ModeControl1, SLAVE_QUAD_128)
            .await?;

        self.dac_ctrl_cache = DAC_I2S_24BIT;
        self.write_register(Register::DacControl, self.dac_ctrl_cache)
            .await?;

        self.write_register(Register::DacVolumeA, 0x00).await?;
        self.write_register(Register::DacVolumeB, 0x00).await?;

        self.adc_ctrl_cache = ADC_I2S_24BIT;
        self.write_register(Register::AdcControl, self.adc_ctrl_cache)
            .await?;

        self.write_register(Register::ModeControl2, MODE_CTRL2_CP_EN)
            .await?;

        Ok(())
    }

    pub async fn set_dac_volume(&mut self, attenuation_half_db: u8) -> Result<(), I2C::Error> {
        self.write_register(Register::DacVolumeA, attenuation_half_db)
            .await?;
        self.write_register(Register::DacVolumeB, attenuation_half_db)
            .await
    }

    pub async fn set_mute(&mut self, mute: bool) -> Result<(), I2C::Error> {
        if mute {
            self.dac_ctrl_cache |= DAC_CTRL_MUTE_A | DAC_CTRL_MUTE_B;
        } else {
            self.dac_ctrl_cache &= !(DAC_CTRL_MUTE_A | DAC_CTRL_MUTE_B);
        }
        self.write_register(Register::DacControl, self.dac_ctrl_cache)
            .await
    }

    pub async fn set_hpf(&mut self, enabled: bool) -> Result<(), I2C::Error> {
        if enabled {
            self.adc_ctrl_cache &= !ADC_CTRL_HPF_FREEZE;
        } else {
            self.adc_ctrl_cache |= ADC_CTRL_HPF_FREEZE;
        }
        self.write_register(Register::AdcControl, self.adc_ctrl_cache)
            .await
    }
}
