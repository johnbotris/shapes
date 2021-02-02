pub struct SampleCounter {
    sample: u64,
    samplerate: u32,
}

impl SampleCounter {
    pub fn new(samplerate: u32) -> Self {
        Self {
            sample: 0,
            samplerate,
        }
    }

    pub fn inc(&mut self) {
        self.sample += 1
    }

    pub fn reset(&mut self) {
        self.sample = 0
    }

    pub fn get_secs(&self) -> f32 {
        self.sample as f32 / self.samplerate as f32
    }

    pub fn sample(&self) -> u64 {
        self.sample
    }

    pub fn samplerate(&self) -> f32 {
        self.samplerate as f32
    }
}
