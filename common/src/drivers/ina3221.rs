use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embedded_hal::i2c::I2c;

/// INA3221 register map.
const REG_CONFIG: u8 = 0x00;
const REG_SHUNT_1: u8 = 0x01;
const REG_BUS_1: u8 = 0x02;
const REG_SHUNT_2: u8 = 0x03;
const REG_BUS_2: u8 = 0x04;
const REG_SHUNT_3: u8 = 0x05;
const REG_BUS_3: u8 = 0x06;

// Configuration bits: continuous shunt + bus, default averaging and conversion times.
const CFG_AVG_64: u16 = 0b011 << 9;
const CFG_VBUSCT_1100US: u16 = 0b100 << 6;
const CFG_VSHCT_1100US: u16 = 0b100 << 3;
const CFG_MODE_SHUNT_BUS_CONTINUOUS: u16 = 0b111;

/// Single-channel reading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ina3221ChannelData {
    pub bus_voltage_mv: u16,
    pub shunt_current_ma: i16,
}

/// All channels in one shot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ina3221Measurements {
    pub ch1: Ina3221ChannelData,
    pub ch2: Ina3221ChannelData,
    pub ch3: Ina3221ChannelData,
}

pub struct Ina3221<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    shunt_resistor_milliohm: [u16; 3],
    i2c: &'static Mutex<PlatformMutex, I2C>,
}

impl<I2C> Ina3221<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(
        address: u8,
        shunt_resistor_milliohm: [u16; 3],
        i2c: &'static Mutex<PlatformMutex, I2C>,
    ) -> Self {
        Self {
            address,
            shunt_resistor_milliohm,
            i2c,
        }
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        let config =
            CFG_AVG_64 | CFG_VBUSCT_1100US | CFG_VSHCT_1100US | CFG_MODE_SHUNT_BUS_CONTINUOUS;
        self.write_register(REG_CONFIG, config).await
    }

    pub async fn read_all(&mut self) -> Result<Ina3221Measurements, I2C::Error> {
        let bus1 = self.read_register(REG_BUS_1).await?;
        let shunt1 = self.read_register(REG_SHUNT_1).await?;

        let bus2 = self.read_register(REG_BUS_2).await?;
        let shunt2 = self.read_register(REG_SHUNT_2).await?;

        let bus3 = self.read_register(REG_BUS_3).await?;
        let shunt3 = self.read_register(REG_SHUNT_3).await?;

        Ok(Ina3221Measurements {
            ch1: Ina3221ChannelData {
                bus_voltage_mv: bus_voltage_from_raw(bus1),
                shunt_current_ma: shunt_current_from_raw(shunt1, self.shunt_resistor_milliohm[0]),
            },
            ch2: Ina3221ChannelData {
                bus_voltage_mv: bus_voltage_from_raw(bus2),
                shunt_current_ma: shunt_current_from_raw(shunt2, self.shunt_resistor_milliohm[1]),
            },
            ch3: Ina3221ChannelData {
                bus_voltage_mv: bus_voltage_from_raw(bus3),
                shunt_current_ma: shunt_current_from_raw(shunt3, self.shunt_resistor_milliohm[2]),
            },
        })
    }

    async fn write_register(&self, reg: u8, value: u16) -> Result<(), I2C::Error> {
        let data = [reg, (value >> 8) as u8, value as u8];
        self.i2c.lock().await.write(self.address, &data)
    }

    async fn read_register(&self, reg: u8) -> Result<i16, I2C::Error> {
        let mut lock = self.i2c.lock().await;
        lock.write(self.address, &[reg])?;

        let mut buffer = [0u8; 2];
        lock.read(self.address, &mut buffer)?;

        let raw = ((buffer[0] as u16) << 8) | (buffer[1] as u16);
        Ok(raw as i16)
    }
}

fn bus_voltage_from_raw(raw: i16) -> u16 {
    // INA3221 bus voltage LSB is 8 mV; clamp negatives to zero.
    let raw_mv = (raw as i32).max(0) * 8;
    raw_mv as u16
}

fn shunt_current_from_raw(raw: i16, shunt_milliohm: u16) -> i16 {
    // INA3221 shunt voltage LSB is 40 uV. I = V/R.
    let microvolts = raw as i32 * 40;
    if shunt_milliohm == 0 {
        return 0;
    }
    let milliamps = microvolts / shunt_milliohm as i32;
    milliamps.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}
