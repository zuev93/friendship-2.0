use embassy_time::{Duration, Instant};

use crate::app::types::Mode;
use crate::consts::AUDIO_BUFFER_SIZE;

const SAMPLE_RATE_HZ: u32 = 48_000;
const BUFFER_PERIOD: Duration =
    Duration::from_micros((AUDIO_BUFFER_SIZE as u64 * 1_000_000) / SAMPLE_RATE_HZ as u64);
const TONE_FREQ_HZ: u32 = 1000;
const BEEP_FREQ_HZ: u32 = 1500;
const BUTTON_BEEP_MS: u64 = 120;

const WARMUP_MELODY: &[(u32, u32)] = &[
    (659, 4),
    (587, 4),
    (524, 4),
    (392, 4),
    (523, 4),
    (587, 4),
    (659, 4),
    (784, 8),
    (659, 8),
    (0, 4),
];

pub struct ToneGenerator {
    osc: ToneOsc,
    tone_button: bool,
    beep_until: Option<Instant>,
    warmup: WarmupState,
}

impl ToneGenerator {
    pub fn new() -> Self {
        Self {
            osc: ToneOsc::new(TONE_FREQ_HZ),
            tone_button: false,
            beep_until: None,
            warmup: WarmupState::new(),
        }
    }

    pub fn buffer_period() -> Duration {
        BUFFER_PERIOD
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if mode == Mode::WarmUp {
            self.warmup.start();
        } else {
            self.warmup.stop();
        }
    }

    pub fn set_tone_active(&mut self, active: bool) {
        self.tone_button = active;
    }

    pub fn trigger_beep(&mut self) {
        self.beep_until = Some(Instant::now() + Duration::from_millis(BUTTON_BEEP_MS));
    }

    pub fn next_buffer(&mut self) -> [u16; AUDIO_BUFFER_SIZE] {
        let now = Instant::now();
        let mut freq = None;

        if let Some(f) = self.warmup.current_freq() {
            freq = Some(f);
        } else if self.tone_button {
            freq = Some(TONE_FREQ_HZ);
        } else if self.beep_until.map(|t| now < t).unwrap_or(false) {
            freq = Some(BEEP_FREQ_HZ);
        }

        let mut buffer = [0u16; AUDIO_BUFFER_SIZE];
        if let Some(f) = freq {
            self.osc.set_freq(f);
            for sample in buffer.iter_mut() {
                *sample = self.osc.next_sample();
            }
        }

        self.warmup.on_buffer_done();
        buffer
    }
}

struct ToneOsc {
    phase: u32,
    step: u32,
}

impl ToneOsc {
    fn new(freq: u32) -> Self {
        let mut osc = Self { phase: 0, step: 0 };
        osc.set_freq(freq);
        osc
    }

    fn set_freq(&mut self, freq: u32) {
        self.step = ((freq as u64) << 32) as u32 / SAMPLE_RATE_HZ;
    }

    fn next_sample(&mut self) -> u16 {
        self.phase = self.phase.wrapping_add(self.step);
        (self.phase >> 16) as u16
    }
}

struct WarmupState {
    active: bool,
    index: usize,
    remaining_buffers: u32,
}

impl WarmupState {
    fn new() -> Self {
        Self {
            active: false,
            index: 0,
            remaining_buffers: 0,
        }
    }

    fn start(&mut self) {
        self.active = true;
        self.index = 0;
        self.remaining_buffers = WARMUP_MELODY.get(0).map(|(_, c)| *c).unwrap_or(0);
    }

    fn stop(&mut self) {
        self.active = false;
        self.remaining_buffers = 0;
    }

    fn current_freq(&self) -> Option<u32> {
        if !self.active {
            return None;
        }
        WARMUP_MELODY.get(self.index).map(|(f, _)| *f)
    }

    fn on_buffer_done(&mut self) {
        if !self.active {
            return;
        }
        if self.remaining_buffers > 0 {
            self.remaining_buffers -= 1;
        }
        if self.remaining_buffers == 0 {
            self.index += 1;
            if let Some((_, count)) = WARMUP_MELODY.get(self.index) {
                self.remaining_buffers = *count;
            } else {
                self.stop();
            }
        }
    }
}
