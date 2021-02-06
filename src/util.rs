pub struct SampleTimer {
    sample: u64,
    samplerate: u32,
}

impl SampleTimer {
    pub fn new(samplerate: u32) -> Self {
        Self {
            sample: 0,
            samplerate,
        }
    }

    pub fn inc(&mut self, amt: u64) {
        self.sample += amt
    }

    pub fn reset(&mut self) {
        self.sample = 0
    }

    pub fn time_since(&self, sample_in_past: u64) -> f32 {
        if sample_in_past == u64::MAX {
            return f32::MAX; // TODO i hate this logic
        }
        (self.sample - sample_in_past) as f32 / self.samplerate as f32
    }

    pub fn sample(&self) -> u64 {
        self.sample
    }

    pub fn samplerate(&self) -> f32 {
        self.samplerate as f32
    }
}

impl std::ops::AddAssign<u64> for SampleTimer {
    fn add_assign(&mut self, amt: u64) {
        self.inc(amt)
    }
}
