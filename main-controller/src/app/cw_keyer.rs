use crate::app::types::CwKeyMode;

#[derive(PartialEq)]
enum KeyerState {
    Idle,
    Dit,
    DitPause,
    Dah,
    DahPause,
    Hanging,
}

pub struct KeyerOutput {
    pub key_changed: Option<bool>,
    pub tx_changed: Option<bool>,
}

pub struct CwKeyer {
    state: KeyerState,
    tick_counter: u16,
    dit_memory: bool,
    dah_memory: bool,
    key_active: bool,
    tx_active: bool,
    hang_counter: u16,
    mode: CwKeyMode,
    dit_ticks: u16,
    dah_ticks: u16,
    weight_pct: u8,
    hang_ticks: u16,
    straight_prev: bool,
}

impl CwKeyer {
    pub fn new() -> Self {
        let dit_ticks = 1200 / 20;
        Self {
            state: KeyerState::Idle,
            tick_counter: 0,
            dit_memory: false,
            dah_memory: false,
            key_active: false,
            tx_active: false,
            hang_counter: 0,
            mode: CwKeyMode::IambicB,
            dit_ticks,
            dah_ticks: dit_ticks * 3,
            weight_pct: 50,
            hang_ticks: 500,
            straight_prev: false,
        }
    }

    pub fn set_mode(&mut self, mode: CwKeyMode) {
        self.mode = mode;
    }

    pub fn set_wpm(&mut self, wpm: u8) {
        self.dit_ticks = 1200 / wpm as u16;
        self.dah_ticks = self.dit_ticks * 3;
    }

    pub fn set_weight(&mut self, weight: u8) {
        self.weight_pct = weight;
    }

    pub fn set_hang_ticks(&mut self, ticks: u16) {
        self.hang_ticks = ticks;
    }

    fn on_ticks_dit(&self) -> u16 {
        self.dit_ticks * self.weight_pct as u16 / 50
    }

    fn on_ticks_dah(&self) -> u16 {
        self.dah_ticks * self.weight_pct as u16 / 50
    }

    fn off_ticks(&self) -> u16 {
        self.dit_ticks * (100 - self.weight_pct) as u16 / 50
    }

    pub fn tick(&mut self, dit_pressed: bool, dah_pressed: bool) -> KeyerOutput {
        if self.mode == CwKeyMode::Straight {
            return self.tick_straight(dit_pressed);
        }
        self.tick_iambic(dit_pressed, dah_pressed)
    }

    fn tick_straight(&mut self, dit_pressed: bool) -> KeyerOutput {
        let mut key_changed = None;
        let mut tx_changed = None;

        match self.state {
            KeyerState::Idle => {
                if dit_pressed && !self.straight_prev {
                    self.state = KeyerState::Dit;
                    self.key_active = true;
                    self.tx_active = true;
                    key_changed = Some(true);
                    tx_changed = Some(true);
                }
            }
            KeyerState::Dit => {
                if !dit_pressed {
                    self.state = KeyerState::Hanging;
                    self.hang_counter = self.hang_ticks;
                    self.key_active = false;
                    key_changed = Some(false);
                }
            }
            KeyerState::Hanging => {
                if dit_pressed {
                    self.state = KeyerState::Dit;
                    self.key_active = true;
                    key_changed = Some(true);
                } else if self.hang_counter == 0 {
                    self.state = KeyerState::Idle;
                    self.tx_active = false;
                    tx_changed = Some(false);
                } else {
                    self.hang_counter -= 1;
                }
            }
            _ => {
                self.state = KeyerState::Idle;
                self.key_active = false;
                self.tx_active = false;
            }
        }

        self.straight_prev = dit_pressed;
        KeyerOutput {
            key_changed,
            tx_changed,
        }
    }

    fn tick_iambic(&mut self, dit_pressed: bool, dah_pressed: bool) -> KeyerOutput {
        let mut key_changed = None;
        let mut tx_changed = None;
        let is_b = self.mode == CwKeyMode::IambicB;

        match self.state {
            KeyerState::Idle => {
                if dit_pressed {
                    self.enter_dit(&mut key_changed, &mut tx_changed);
                } else if dah_pressed {
                    self.enter_dah(&mut key_changed, &mut tx_changed);
                }
                self.dit_memory = false;
                self.dah_memory = false;
            }
            KeyerState::Dit => {
                if dah_pressed {
                    self.dah_memory = true;
                }
                self.tick_counter -= 1;
                if self.tick_counter == 0 {
                    self.state = KeyerState::DitPause;
                    self.tick_counter = self.off_ticks();
                    self.key_active = false;
                    key_changed = Some(false);
                }
            }
            KeyerState::DitPause => {
                if is_b {
                    if dit_pressed {
                        self.dit_memory = true;
                    }
                    if dah_pressed {
                        self.dah_memory = true;
                    }
                }
                self.tick_counter -= 1;
                if self.tick_counter == 0 {
                    if self.dah_memory {
                        self.dah_memory = false;
                        self.dit_memory = false;
                        self.enter_dah_continue(&mut key_changed);
                    } else if dit_pressed {
                        self.dit_memory = false;
                        self.enter_dit_continue(&mut key_changed);
                    } else if dah_pressed {
                        self.dah_memory = false;
                        self.enter_dah_continue(&mut key_changed);
                    } else {
                        self.state = KeyerState::Hanging;
                        self.hang_counter = self.hang_ticks;
                    }
                }
            }
            KeyerState::Dah => {
                if dit_pressed {
                    self.dit_memory = true;
                }
                self.tick_counter -= 1;
                if self.tick_counter == 0 {
                    self.state = KeyerState::DahPause;
                    self.tick_counter = self.off_ticks();
                    self.key_active = false;
                    key_changed = Some(false);
                }
            }
            KeyerState::DahPause => {
                if is_b {
                    if dit_pressed {
                        self.dit_memory = true;
                    }
                    if dah_pressed {
                        self.dah_memory = true;
                    }
                }
                self.tick_counter -= 1;
                if self.tick_counter == 0 {
                    if self.dit_memory {
                        self.dit_memory = false;
                        self.dah_memory = false;
                        self.enter_dit_continue(&mut key_changed);
                    } else if dah_pressed {
                        self.dah_memory = false;
                        self.enter_dah_continue(&mut key_changed);
                    } else if dit_pressed {
                        self.dit_memory = false;
                        self.enter_dit_continue(&mut key_changed);
                    } else {
                        self.state = KeyerState::Hanging;
                        self.hang_counter = self.hang_ticks;
                    }
                }
            }
            KeyerState::Hanging => {
                if dit_pressed {
                    self.enter_dit_continue(&mut key_changed);
                } else if dah_pressed {
                    self.enter_dah_continue(&mut key_changed);
                } else if self.hang_counter == 0 {
                    self.state = KeyerState::Idle;
                    self.tx_active = false;
                    tx_changed = Some(false);
                } else {
                    self.hang_counter -= 1;
                }
            }
        }

        KeyerOutput {
            key_changed,
            tx_changed,
        }
    }

    fn enter_dit(&mut self, key_changed: &mut Option<bool>, tx_changed: &mut Option<bool>) {
        self.state = KeyerState::Dit;
        self.tick_counter = self.on_ticks_dit();
        self.key_active = true;
        *key_changed = Some(true);
        if !self.tx_active {
            self.tx_active = true;
            *tx_changed = Some(true);
        }
    }

    fn enter_dah(&mut self, key_changed: &mut Option<bool>, tx_changed: &mut Option<bool>) {
        self.state = KeyerState::Dah;
        self.tick_counter = self.on_ticks_dah();
        self.key_active = true;
        *key_changed = Some(true);
        if !self.tx_active {
            self.tx_active = true;
            *tx_changed = Some(true);
        }
    }

    fn enter_dit_continue(&mut self, key_changed: &mut Option<bool>) {
        self.state = KeyerState::Dit;
        self.tick_counter = self.on_ticks_dit();
        self.key_active = true;
        *key_changed = Some(true);
    }

    fn enter_dah_continue(&mut self, key_changed: &mut Option<bool>) {
        self.state = KeyerState::Dah;
        self.tick_counter = self.on_ticks_dah();
        self.key_active = true;
        *key_changed = Some(true);
    }
}
