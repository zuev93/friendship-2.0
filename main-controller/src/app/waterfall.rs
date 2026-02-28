use crate::app::types::{Frequency, WaterfallLine, WATERFALL_BINS};

pub struct WaterfallSweeper {
    span_hz: u32,
    current_bin: usize,
    line: WaterfallLine,
}

impl WaterfallSweeper {
    pub fn new(span_hz: u32) -> Self {
        Self {
            span_hz,
            current_bin: 0,
            line: WaterfallLine::new(),
        }
    }

    pub fn next_bin_frequency(&self, vfo_freq: Frequency) -> Frequency {
        let half_span = self.span_hz / 2;
        let step = self.span_hz / WATERFALL_BINS as u32;
        let start = vfo_freq.saturating_sub(half_span);
        start + self.current_bin as u32 * step
    }

    pub fn store_rssi(&mut self, rssi_raw: i8) {
        if self.current_bin < WATERFALL_BINS {
            self.line.bins[self.current_bin] = rssi_raw;
            self.current_bin += 1;
            if self.current_bin >= WATERFALL_BINS {
                self.line.complete = true;
            }
        }
    }

    pub fn is_line_complete(&self) -> bool {
        self.line.complete
    }

    pub fn take_line(&mut self) -> WaterfallLine {
        let line = self.line;
        self.line = WaterfallLine::new();
        self.current_bin = 0;
        line
    }
}
