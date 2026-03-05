use crate::consts::AUDIO_BUFFER_SIZE;
use crate::cordic_math::{with_cordic, CordicMutex};

const SAMPLE_RATE_HZ: u32 = 48_000;
const TONE_FREQ_HZ: u32 = 1000;
const BEEP_FREQ_HZ: u32 = 1500;
const BEEP_BUFFERS: u32 =
    (BUTTON_BEEP_MS as u32 * SAMPLE_RATE_HZ) / (AUDIO_BUFFER_SIZE as u32 * 1000);
const BUTTON_BEEP_MS: u32 = 120;
const SIDETONE_ENVELOPE_STEP: u16 = 455;
const TWO_PI: f32 = 2.0 * core::f32::consts::PI;

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
    cordic: &'static CordicMutex,
    osc: ToneOsc,
    tone_button: bool,
    beep_remaining: u32,
    warmup: WarmupState,
    sidetone_osc: ToneOsc,
    sidetone_active: bool,
    sidetone_envelope: u16,
}

impl ToneGenerator {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        Self {
            cordic,
            osc: ToneOsc::new(TONE_FREQ_HZ),
            tone_button: false,
            beep_remaining: 0,
            warmup: WarmupState::new(),
            sidetone_osc: ToneOsc::new(700),
            sidetone_active: false,
            sidetone_envelope: 0,
        }
    }

    pub fn set_warmup(&mut self, active: bool) {
        if active {
            self.warmup.start();
        } else {
            self.warmup.stop();
        }
    }

    pub fn set_tone_active(&mut self, active: bool) {
        self.tone_button = active;
    }

    pub fn trigger_beep(&mut self) {
        self.beep_remaining = BEEP_BUFFERS;
    }

    pub fn set_sidetone_active(&mut self, active: bool) {
        self.sidetone_active = active;
    }

    pub fn set_sidetone_freq(&mut self, freq: u16) {
        self.sidetone_osc.set_freq(freq as u32);
    }

    pub fn next_buffer(&mut self) -> [u16; AUDIO_BUFFER_SIZE] {
        let mut freq = None;

        if let Some(f) = self.warmup.current_freq() {
            freq = Some(f);
        } else if self.tone_button {
            freq = Some(TONE_FREQ_HZ);
        } else if self.beep_remaining > 0 {
            freq = Some(BEEP_FREQ_HZ);
            self.beep_remaining -= 1;
        }

        let mut buffer = [0u16; AUDIO_BUFFER_SIZE];

        let sidetone_rendering = self.sidetone_active || self.sidetone_envelope > 0;

        if freq.is_some() && !sidetone_rendering {
            let f = freq.unwrap();
            self.osc.set_freq(f);
            for sample in buffer.iter_mut() {
                *sample = self.osc.next_sine_sample(self.cordic);
            }
        } else if sidetone_rendering {
            for sample in buffer.iter_mut() {
                if self.sidetone_active {
                    self.sidetone_envelope = self
                        .sidetone_envelope
                        .saturating_add(SIDETONE_ENVELOPE_STEP)
                        .min(u16::MAX);
                } else {
                    self.sidetone_envelope = self
                        .sidetone_envelope
                        .saturating_sub(SIDETONE_ENVELOPE_STEP);
                }
                let raw = self.sidetone_osc.next_sine_sample(self.cordic);
                *sample = ((raw as u32 * self.sidetone_envelope as u32) >> 16) as u16;
            }
        }

        self.warmup.on_buffer_done();
        buffer
    }
}

struct ToneOsc {
    phase: f32,
    phase_step: f32,
}

impl ToneOsc {
    fn new(freq: u32) -> Self {
        let mut osc = Self {
            phase: 0.0,
            phase_step: 0.0,
        };
        osc.set_freq(freq);
        osc
    }

    fn set_freq(&mut self, freq: u32) {
        self.phase_step = TWO_PI * freq as f32 / SAMPLE_RATE_HZ as f32;
    }

    fn next_sine_sample(&mut self, cordic: &'static CordicMutex) -> u16 {
        let val = with_cordic(cordic, |c| c.sinf(self.phase));
        self.phase += self.phase_step;
        if self.phase >= TWO_PI {
            self.phase -= TWO_PI;
        }
        ((val * 32767.0) as i32 + 32768) as u16
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
