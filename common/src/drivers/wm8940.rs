//! WM8940 Audio Codec Driver
//! I2C control interface

use embedded_hal_async::i2c::I2c;

const WM8940_I2C_ADDR: u8 = 0x1A;

// Register addresses
#[allow(dead_code)]
#[repr(u8)]
pub enum Register {
    SoftwareReset = 0x00,
    Power1 = 0x01,
    Power2 = 0x02,
    Power3 = 0x03,
    Interface = 0x04,
    Companding = 0x05,
    ClockGen = 0x06,
    AdditionalControl = 0x07,
    GpioStuff = 0x08,
    JackDetect1 = 0x09,
    DacControl = 0x0A,
    LeftDacVolume = 0x0B,
    RightDacVolume = 0x0C,
    JackDetect2 = 0x0D,
    AdcControl = 0x0E,
    LeftAdcVolume = 0x0F,
    RightAdcVolume = 0x10,
    EqGain1 = 0x12,
    EqGain2 = 0x13,
    EqGain3 = 0x14,
    EqGain4 = 0x15,
    EqGain5 = 0x16,
    DacLimiter1 = 0x18,
    DacLimiter2 = 0x19,
    NotchFilter1 = 0x1B,
    NotchFilter2 = 0x1C,
    NotchFilter3 = 0x1D,
    NotchFilter4 = 0x1E,
    AlcControl1 = 0x20,
    AlcControl2 = 0x21,
    AlcControl3 = 0x22,
    NoiseGate = 0x23,
    Plln = 0x24,
    Pllk1 = 0x25,
    Pllk2 = 0x26,
    Pllk3 = 0x27,
    Attenuation = 0x28,
    InputControl = 0x2C,
    LeftInputPgaGainControl = 0x2D,
    RightInputPgaGainControl = 0x2E,
    LeftAdcBoostControl = 0x2F,
    RightAdcBoostControl = 0x30,
    OutputControl = 0x31,
    LeftMixerControl = 0x32,
    RightMixerControl = 0x33,
    LeftHeadphoneVolumeControl = 0x34,
    RightHeadphoneVolumeControl = 0x35,
    LeftSpeakerVolumeControl = 0x36,
    RightSpeakerVolumeControl = 0x37,
    Out3MixerControl = 0x38,
    Out4MixerControl = 0x39,
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

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        // Software reset
        self.reset().await?;

        // Basic initialization sequence
        // Enable VMID, VREF, and bias
        self.write_register(Register::Power1, 0x01C0).await?;

        // Enable left and right DAC
        self.write_register(Register::Power2, 0x0180).await?;

        // Enable left and right output mixer
        self.write_register(Register::Power3, 0x006C).await?;

        Ok(())
    }
}
