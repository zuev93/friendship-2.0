use crate::consts::AUDIO_BUFFER_SIZE;

const VOX_AUDIO_DELAY_BUFFERS: usize = 4;
const GAIN_SCALE: u32 = 8;
const CALIBRATION_BUFFERS: u8 = 20;

#[derive(PartialEq)]
enum VoxState {
    Idle,
    Active,
    Hanging,
}

pub struct VoxResult {
    pub delayed_mic: [u16; AUDIO_BUFFER_SIZE],
    pub transition: Option<bool>,
}

pub struct VoxProcessor {
    state: VoxState,
    hang_counter: u32,
    hang_limit: u32,
    noise_floor: u32,
    calibration_counter: u8,

    delay_ring: [[u16; AUDIO_BUFFER_SIZE]; VOX_AUDIO_DELAY_BUFFERS],
    delay_idx: usize,

    enabled: bool,
    gain_raw: u16,
    delay_raw: u16,
    anti_trip_raw: u16,
    voice_mode: bool,
}

impl VoxProcessor {
    pub fn new() -> Self {
        Self {
            state: VoxState::Idle,
            hang_counter: 0,
            hang_limit: 500 * 48000 / (AUDIO_BUFFER_SIZE as u32 * 1000),
            noise_floor: 500,
            calibration_counter: 0,

            delay_ring: [[32768; AUDIO_BUFFER_SIZE]; VOX_AUDIO_DELAY_BUFFERS],
            delay_idx: 0,

            enabled: false,
            gain_raw: 500,
            delay_raw: 500,
            anti_trip_raw: 500,
            voice_mode: false,
        }
    }

    pub fn process(
        &mut self,
        mic: &[u16; AUDIO_BUFFER_SIZE],
        rx: &[u16; AUDIO_BUFFER_SIZE],
        usb_tx_active: bool,
    ) -> VoxResult {
        let oldest_idx = self.delay_idx;
        let delayed_mic = self.delay_ring[oldest_idx];

        self.delay_ring[self.delay_idx] = *mic;
        self.delay_idx = (self.delay_idx + 1) % VOX_AUDIO_DELAY_BUFFERS;

        if !self.enabled || !self.voice_mode || usb_tx_active {
            if self.state != VoxState::Idle {
                self.state = VoxState::Idle;
                return VoxResult {
                    delayed_mic,
                    transition: Some(false),
                };
            }
            return VoxResult {
                delayed_mic,
                transition: None,
            };
        }

        let mut mic_peak: u32 = 0;
        for &s in mic.iter() {
            let diff = (s as i32 - 32768).unsigned_abs();
            if diff > mic_peak {
                mic_peak = diff;
            }
        }

        let mut rx_peak: u32 = 0;
        for &s in rx.iter() {
            let diff = (s as i32 - 32768).unsigned_abs();
            if diff > rx_peak {
                rx_peak = diff;
            }
        }

        let gain_offset = self.gain_raw as u32 * GAIN_SCALE;
        let anti_trip = rx_peak * self.anti_trip_raw as u32 / 1000;
        let effective_threshold = self.noise_floor + gain_offset + anti_trip;
        let threshold_on = effective_threshold + effective_threshold / 5;
        let threshold_hold = effective_threshold;

        let mut transition: Option<bool> = None;

        match self.state {
            VoxState::Idle => {
                self.noise_floor = self.noise_floor - self.noise_floor / 256 + mic_peak / 256;

                if self.calibration_counter > 0 {
                    self.calibration_counter -= 1;
                } else if mic_peak > threshold_on {
                    self.state = VoxState::Active;
                    transition = Some(true);
                }
            }
            VoxState::Active => {
                if mic_peak < threshold_hold {
                    self.state = VoxState::Hanging;
                    self.hang_counter = self.hang_limit;
                }
            }
            VoxState::Hanging => {
                if mic_peak > threshold_on {
                    self.state = VoxState::Active;
                } else if self.hang_counter == 0 {
                    self.state = VoxState::Idle;
                    transition = Some(false);
                } else {
                    self.hang_counter -= 1;
                }
            }
        }

        VoxResult {
            delayed_mic,
            transition,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            self.noise_floor = 500;
            self.calibration_counter = CALIBRATION_BUFFERS;
        } else if self.state != VoxState::Idle {
            self.state = VoxState::Idle;
        }
    }

    pub fn set_gain(&mut self, raw: u16) {
        self.gain_raw = raw;
    }

    pub fn set_delay(&mut self, ms: u16) {
        self.delay_raw = ms;
        self.hang_limit = ms as u32 * 48000 / (AUDIO_BUFFER_SIZE as u32 * 1000);
    }

    pub fn set_anti_trip(&mut self, raw: u16) {
        self.anti_trip_raw = raw;
    }

    pub fn set_voice_mode(&mut self, voice: bool) {
        self.voice_mode = voice;
    }

    pub fn is_active(&self) -> bool {
        self.state != VoxState::Idle
    }
}
