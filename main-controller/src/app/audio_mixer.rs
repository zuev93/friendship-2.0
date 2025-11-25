use crate::consts::AUDIO_BUFFER_SIZE;

pub struct AudioMixer {}

impl AudioMixer {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn get_buffer_headphones(&self) -> [u16; AUDIO_BUFFER_SIZE] {
        // TODO implement me
        return [0u16; AUDIO_BUFFER_SIZE];
    }
    pub fn get_buffer_tx(&self) -> [u16; AUDIO_BUFFER_SIZE] {
        // TODO implement me
        return [0u16; AUDIO_BUFFER_SIZE];
    }
    pub fn get_buffer_speakers(&self) -> [u16; AUDIO_BUFFER_SIZE] {
        // TODO implement me
        return [0u16; AUDIO_BUFFER_SIZE];
    }

    pub fn set_buffer_rx(&self, _buffer: [u16; AUDIO_BUFFER_SIZE]) {
        // TODO implement me}
    }
    pub fn set_buffer_generator(&self, _buffer: [u16; AUDIO_BUFFER_SIZE]) {
        // TODO implement me}
    }
    pub fn set_buffer_mic(&self, _buffer: [u16; AUDIO_BUFFER_SIZE]) {
        // TODO implement me}
    }
}
