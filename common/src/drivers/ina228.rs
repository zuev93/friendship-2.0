use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embedded_hal_async::i2c::I2c;

const REG_CONFIG: u8 = 0x00;
const REG_ADC_CONFIG: u8 = 0x01;
const REG_SHUNT_CAL: u8 = 0x02;
const REG_VBUS: u8 = 0x05;
const REG_CURRENT: u8 = 0x07;
const REG_POWER: u8 = 0x08;

const CFG_RST: u16 = 1 << 15;
const CFG_ADCRANGE_163MV: u16 = 0;

const ADC_AVG_4: u16 = 0b001 << 0;
const ADC_AVG_64: u16 = 0b011 << 0;
const ADC_VBUSCT_1052US: u16 = 0b100 << 6;
const ADC_VSHCT_1052US: u16 = 0b100 << 3;
const ADC_VTCT_1052US: u16 = 0b100 << 9;
const ADC_MODE_CONTINUOUS_ALL: u16 = 0b1111 << 12;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ina228Reading {
    pub bus_voltage_mv: u32,
    pub current_ma: i32,
    pub power_mw: u32,
}

pub struct Ina228<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    shunt_cal: u16,
    current_lsb_na: u32,
    i2c: &'static Mutex<PlatformMutex, I2C>,
}

impl<I2C> Ina228<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(
        address: u8,
        shunt_resistor_mohm: u16,
        max_current_ma: u32,
        i2c: &'static Mutex<PlatformMutex, I2C>,
    ) -> Self {
        let current_lsb_na = (max_current_ma as u64 * 1_000_000) / (1u64 << 19);
        let current_lsb_na = current_lsb_na.max(1) as u32;

        let shunt_cal = (13107_200_000u64 * shunt_resistor_mohm as u64 * current_lsb_na as u64)
            / 1_000_000_000u64;
        let shunt_cal = (shunt_cal & 0x7FFF) as u16;

        Self {
            address,
            shunt_cal,
            current_lsb_na,
            i2c,
        }
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.write_reg16(REG_CONFIG, CFG_RST).await?;

        let config = CFG_ADCRANGE_163MV;
        self.write_reg16(REG_CONFIG, config).await?;

        let adc_config = ADC_MODE_CONTINUOUS_ALL
            | ADC_VTCT_1052US
            | ADC_VBUSCT_1052US
            | ADC_VSHCT_1052US
            | ADC_AVG_64;
        self.write_reg16(REG_ADC_CONFIG, adc_config).await?;

        self.write_reg16(REG_SHUNT_CAL, self.shunt_cal).await?;

        Ok(())
    }

    pub async fn read_bus_voltage_mv(&self) -> Result<u32, I2C::Error> {
        let raw = self.read_reg24(REG_VBUS).await?;
        let raw20 = raw >> 4;
        let uv = raw20 as u64 * 195_3125 / 10_000;
        Ok((uv / 1000) as u32)
    }

    pub async fn read_current_ma(&self) -> Result<i32, I2C::Error> {
        let raw = self.read_reg24(REG_CURRENT).await?;
        let raw20 = (raw as i32) >> 4;
        let na = raw20 as i64 * self.current_lsb_na as i64;
        Ok((na / 1_000_000) as i32)
    }

    pub async fn read_power_mw(&self) -> Result<u32, I2C::Error> {
        let raw = self.read_reg24(REG_POWER).await?;
        let power_lsb_nw = self.current_lsb_na as u64 * 3200 / 1000;
        let nw = raw as u64 * power_lsb_nw;
        Ok((nw / 1_000_000) as u32)
    }

    pub async fn set_fast_mode(&self) -> Result<(), I2C::Error> {
        let adc_config = ADC_MODE_CONTINUOUS_ALL
            | ADC_VTCT_1052US
            | ADC_VBUSCT_1052US
            | ADC_VSHCT_1052US
            | ADC_AVG_4;
        self.write_reg16(REG_ADC_CONFIG, adc_config).await
    }

    pub async fn set_normal_mode(&self) -> Result<(), I2C::Error> {
        let adc_config = ADC_MODE_CONTINUOUS_ALL
            | ADC_VTCT_1052US
            | ADC_VBUSCT_1052US
            | ADC_VSHCT_1052US
            | ADC_AVG_64;
        self.write_reg16(REG_ADC_CONFIG, adc_config).await
    }

    pub async fn read_all(&mut self) -> Result<Ina228Reading, I2C::Error> {
        let bus_voltage_mv = self.read_bus_voltage_mv().await?;
        let current_ma = self.read_current_ma().await?;
        let power_mw = self.read_power_mw().await?;
        Ok(Ina228Reading {
            bus_voltage_mv,
            current_ma,
            power_mw,
        })
    }

    async fn write_reg16(&self, reg: u8, value: u16) -> Result<(), I2C::Error> {
        let data = [reg, (value >> 8) as u8, value as u8];
        self.i2c.lock().await.write(self.address, &data).await
    }

    async fn read_reg24(&self, reg: u8) -> Result<u32, I2C::Error> {
        let mut buf = [0u8; 3];
        self.i2c
            .lock()
            .await
            .write_read(self.address, &[reg], &mut buf)
            .await?;
        Ok(((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32))
    }
}
