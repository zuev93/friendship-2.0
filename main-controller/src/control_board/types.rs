use embassy_stm32::{
    i2c::{mode as i2c_mode, I2c},
    mode,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

pub type ControlBoardI2C = I2c<'static, mode::Async, i2c_mode::Master>;
pub type ControlBoardI2cMutex = &'static Mutex<ThreadModeRawMutex, ControlBoardI2C>;
