const SMOOTHING_ALPHA: f32 = 0.1;
const S9_DBM: f32 = -73.0;
const S_STEP_DB: f32 = 6.0;

pub struct Smeter {
    smoothed_dbm: f32,
    ad8367_gain_db: f32,
    calibration_offset: f32,
}

impl Smeter {
    pub const fn new() -> Self {
        Self {
            smoothed_dbm: -120.0,
            ad8367_gain_db: 0.0,
            calibration_offset: 0.0,
        }
    }

    pub fn update(&mut self, digital_level_db: f32, ad8367_gain_db: f32) {
        self.ad8367_gain_db = ad8367_gain_db;
        let dbm = digital_level_db - ad8367_gain_db + self.calibration_offset;
        self.smoothed_dbm = SMOOTHING_ALPHA * dbm + (1.0 - SMOOTHING_ALPHA) * self.smoothed_dbm;
    }

    pub fn dbm(&self) -> f32 {
        self.smoothed_dbm
    }

    pub fn s_units(&self) -> f32 {
        let diff = self.smoothed_dbm - S9_DBM;
        9.0 + diff / S_STEP_DB
    }

    pub fn s_string(&self) -> (u8, i8) {
        let s = self.s_units();
        if s <= 9.0 {
            let s_val = (s.clamp(0.0, 9.0)) as u8;
            (s_val, 0)
        } else {
            let over_db = ((s - 9.0) * S_STEP_DB) as i8;
            (9, over_db)
        }
    }

    pub fn set_calibration(&mut self, offset: f32) {
        self.calibration_offset = offset;
    }
}
