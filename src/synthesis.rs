use anyhow::{anyhow, Result};
use core::f32::consts::PI;
use std::time::{Duration, Instant};

use crate::vec2::{self, Vec2};

pub struct LinearPluck {
    pub release: Duration,
    triggered: Instant,
}

impl LinearPluck {
    pub fn new(release: Duration) -> Self {
        Self {
            release,
            triggered: Instant::now() + release,
        }
    }

    pub fn trigger(&mut self, now: Instant) {
        self.triggered = now
    }

    pub fn level(&self, now: Instant) -> f32 {
        let elapsed = now - self.triggered;
        match elapsed.as_millis() {
            0 => 1.0,
            millis => self.release.as_millis() as f32 / millis as f32,
        }
    }
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

pub fn phase(sample: u64, samplerate: f32, freq: f32) -> f32 {
    (sample % (samplerate / freq) as u64) as f32 * freq / samplerate
}

pub fn circle(p: f32) -> Vec2 {
    let theta = 2.0 * PI * p;
    (f32::sin(theta), f32::cos(theta))
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
