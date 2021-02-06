use crate::util::SampleTimer;
use crate::vec2::{self, Vec2};

use anyhow::{anyhow, Result};
use core::f32::consts::PI;
use std::time::Duration;
use wmidi::Note;

const PLUCK_RAMP: f32 = 0.005;

pub struct Envelope {
    pub sample_start: u64,
    pub gate: bool,
    pub release: Duration,
}

impl Envelope {
    pub fn init() -> Self {
        Self {
            sample_start: u64::MAX,
            gate: false,
            release: Duration::from_secs(0),
        }
    }
    pub fn new(sample_start: u64, gate: bool, release: Duration) -> Self {
        Self {
            sample_start,
            gate,
            release,
        }
    }
    pub fn get(&self, timer: &SampleTimer) -> f32 {
        let elapsed = timer.time_since(self.sample_start);
        if elapsed <= PLUCK_RAMP {
            elapsed / PLUCK_RAMP
        } else {
            f32::max(
                1.0 - (elapsed - PLUCK_RAMP) / self.release.as_secs_f32(),
                0.0,
            )
        }
    }
}

pub struct Voice {
    pub note: Note,
    pub envelope: Envelope,
}

#[derive(Debug)]
pub enum UnisonMode {
    Unison,
    Poly,
}

impl std::str::FromStr for UnisonMode {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<UnisonMode> {
        match s.to_lowercase().as_str() {
            "u" | "unison" => Ok(UnisonMode::Unison),
            "p" | "poly" | "polyphonic" => Ok(UnisonMode::Poly),
            _ => Err(anyhow!("Invalid value \"{}\" for UnisonMode", s)),
        }
    }
}

pub fn phase(freq: f32, counter: &SampleTimer) -> f32 {
    (counter.sample() % (counter.samplerate() / freq) as u64) as f32 * freq / counter.samplerate()
}

pub fn circle(p: f32) -> Vec2 {
    let theta = 2.0 * PI * p;
    (f32::sin(theta), f32::cos(theta))
}

pub fn sin(p: f32) -> Vec2 {
    let theta = 2.0 * PI * p;
    let v = f32::sin(theta);
    (v, v)
}

pub fn polygon(n: f32, p: f32) -> Vec2 {
    let step = 1.0 / n;
    let steps = p / step;
    let current = steps.floor();
    let current_p = step * current;
    let progress = steps - current;
    let next_p = current_p + step;
    let c1 = circle(current_p);
    let c2 = circle(next_p);
    vec2::lerp(c1, c2, progress)
}

fn binaural_beats(sample: u64, f1: f32, f2: f32, samplerate: f32) -> Vec2 {
    (0.0, 0.0)
}
