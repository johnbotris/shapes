use crate::maths;
use crate::util::SampleTimer;
use crate::vec2::{self, Vec2};

use anyhow::{anyhow, Result};
use core::f32::consts::PI;
use std::time::Duration;
use wmidi::Note;

pub enum EnvelopeState {
    Held(f32, u64),
    Released(f32, u64),
    Bypass,
    Off,
}

pub struct Envelope {
    state: EnvelopeState,
    attack: Duration,
    decay: Duration,
    sustain_level: f32,
    release: Duration,
}

impl Envelope {
    pub fn new(attack: Duration, decay: Duration, sustain_level: f32, release: Duration) -> Self {
        Self {
            state: EnvelopeState::Off,
            attack,
            decay,
            sustain_level,
            release,
        }
    }

    // TODO how slow is this actually?
    pub fn get(&self, timer: &SampleTimer) -> f32 {
        use EnvelopeState::*;

        match &self.state {
            Held(level_at_hold, start) => {
                let elapsed = timer.time_since(*start);
                let attack = self.attack.as_secs_f32();
                let decay = self.decay.as_secs_f32();

                let attack_completed = (elapsed / attack).clamp(0.0, 1.0);
                let decay_completed = ((elapsed - attack) / decay).clamp(0.0, 1.0);

                let attack_amount = maths::lerp(*level_at_hold, 1.0, attack_completed);
                let decay_amount = maths::lerp(0.0, 1.0 - self.sustain_level, decay_completed);

                attack_amount - decay_amount
            }
            Released(level_at_release, start) => {
                let elapsed = timer.time_since(*start);
                let completed = (elapsed / self.release.as_secs_f32()).clamp(0.0, 1.0);
                maths::lerp(*level_at_release, 0.0, completed)
            }
            Bypass => 1.0,
            Off => 0.0,
        }
    }

    pub fn hold(&mut self, timer: &SampleTimer) {
        let level = self.get(timer);
        self.state = EnvelopeState::Held(level, timer.sample());
    }

    pub fn release(&mut self, timer: &SampleTimer) {
        let level = self.get(timer);
        self.state = EnvelopeState::Released(level, timer.sample());
    }

    pub fn disable(&mut self) {
        self.state = EnvelopeState::Off;
    }

    pub fn bypass(&mut self) {
        self.state = EnvelopeState::Bypass;
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
