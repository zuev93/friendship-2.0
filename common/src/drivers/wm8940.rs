use embedded_hal_async::i2c::I2c;

const WM8940_I2C_ADDR: u8 = 0x1A;

const VOLUME_UPDATE_FLAG: u16 = 0x100;

const POWER1_VMID_VREF_BIAS: u16 = 0x01C0;
const POWER2_DAC_ENABLED: u16 = 0x0180;
const POWER3_OUTPUT_MIXER_ENABLED: u16 = 0x006C;

#[repr(u8)]
pub enum Register {
    SoftwareReset = 0x00,
    Power1 = 0x01,
    Power2 = 0x02,
    Power3 = 0x03,
    DacVolume = 0x0B,
    AdcVolume = 0x0F,
}

pub struct Wm8940<I2C> {
    i2c: I2C,
}

impl<I2C> Wm8940<I2C>
where
    I2C: I2c,
{
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    pub async fn write_register(&mut self, reg: Register, value: u16) -> Result<(), I2C::Error> {
        let reg_addr = reg as u8;
        // WM8940 uses 9-bit register addresses and 9-bit data
        // Packed into 2 bytes: [address(7 bits) | data(9th bit), data(8 bits)]
        let byte1 = (reg_addr << 1) | ((value >> 8) & 0x01) as u8;
        let byte2 = value as u8;

        self.i2c.write(WM8940_I2C_ADDR, &[byte1, byte2]).await
    }

    pub async fn reset(&mut self) -> Result<(), I2C::Error> {
        self.write_register(Register::SoftwareReset, 0).await
    }

    pub async fn set_volume(&mut self, reg: Register, value: u8) -> Result<(), I2C::Error> {
        self.write_register(reg, value as u16 | VOLUME_UPDATE_FLAG)
            .await
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.reset().await?;
        self.write_register(Register::Power1, POWER1_VMID_VREF_BIAS)
            .await?;
        self.write_register(Register::Power2, POWER2_DAC_ENABLED)
            .await?;
        self.write_register(Register::Power3, POWER3_OUTPUT_MIXER_ENABLED)
            .await?;
        Ok(())
    }

    pub async fn power_down(&mut self) -> Result<(), I2C::Error> {
        self.write_register(Register::Power1, 0).await
    }
}
