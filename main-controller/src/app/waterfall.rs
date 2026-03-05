use crate::app::types::{Frequency, WATERFALL_BINS};

pub const NO_DATA: i8 = -128;

pub struct WaterfallSweeper {
    span_hz: u32,
    current_bin: usize,
    bins: [i8; WATERFALL_BINS],
}

impl WaterfallSweeper {
    pub fn new(span_hz: u32) -> Self {
        Self {
            span_hz,
            current_bin: 0,
            bins: [NO_DATA; WATERFALL_BINS],
        }
    }

    pub fn set_span(&mut self, span_hz: u32) {
        if span_hz != self.span_hz {
            self.span_hz = span_hz;
            self.reset();
        }
    }

    pub fn reset(&mut self) {
        self.current_bin = 0;
        self.bins = [NO_DATA; WATERFALL_BINS];
    }

    pub fn span_hz(&self) -> u32 {
        self.span_hz
    }

    pub fn next_bin_frequency(&self, vfo_freq: Frequency) -> Frequency {
        let half_span = self.span_hz / 2;
        let step = self.span_hz / WATERFALL_BINS as u32;
        let start = vfo_freq.saturating_sub(half_span);
        start + self.current_bin as u32 * step
    }

    pub fn store_rssi(&mut self, rssi_raw: i8) {
        if self.current_bin < WATERFALL_BINS {
            self.bins[self.current_bin] = rssi_raw;
            self.current_bin += 1;
            if self.current_bin >= WATERFALL_BINS {
                self.current_bin = 0;
            }
        }
    }

    pub fn bins(&self) -> &[i8; WATERFALL_BINS] {
        &self.bins
    }
}
