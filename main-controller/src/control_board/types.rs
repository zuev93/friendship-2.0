use embassy_stm32::{
    i2c::{mode as i2c_mode, I2c},
    mode,
};

pub type ControlBoardI2C = I2c<'static, mode::Blocking, i2c_mode::Master>;
