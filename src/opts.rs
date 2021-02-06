use crate::constants::*;
use anyhow::Result;
use cpal::{ChannelCount, SampleRate};
use std::convert::TryFrom;
use std::time::Duration;

use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(about)]
pub struct Opts {
    /// How many output channels. Currently ignored. Always 2
    #[structopt(short, long, default_value = "2")]
    pub channels: ChannelCount,

    #[structopt(short, long, default_value = "44100", parse(try_from_str = parse_sample_rate))]
    pub sample_rate: SampleRate,

    /// Audio buffer size, will use system default if unspecified
    #[structopt(short, long)]
    pub buffer_size: Option<u32>,

    /// Number of available voices.
    ///     When unison mode is "unison", 0 means a single voice.
    ///     When unison mode is "poly", 0 means maximum voices
    #[structopt(short = "o", long, default_value = "0")]
    pub voices: u64,

    /// Unison mode. options: u|unison, p|poly
    #[structopt(short, long, parse(try_from_str), default_value = "poly")]
    pub unison_mode: crate::synthesis::UnisonMode,

    /// Output device to connect to
    #[structopt(short, long, default_value = "pulse")]
    pub device: String,

    /// Name of the MIDI input port to connect to
    #[structopt(short = "p", long)]
    pub midi_port: Option<String>,

    /// Index of the MIDI input port to connect to
    #[structopt(long)]
    pub midi_port_index: Option<usize>,

    /// Master gain factor
    #[structopt(short = "g", long, default_value = "0.5")]
    pub master_gain: f32,

    /// List available audio output devices then exit
    #[structopt(long)]
    pub list_outputs: bool,

    /// List available MIDI input ports then exit
    #[structopt(long)]
    pub list_inputs: bool,

    /// Output more information, can be passed multiple times
    #[structopt(short, parse(from_occurrences))]
    pub verbose: u64,

    /// Output less information, can be passed multiple times
    #[structopt(short, parse(from_occurrences))]
    pub quiet: u64,

    /// Attack in seconds
    #[structopt(long, parse(try_from_str = parse_duration), default_value = "0.05")]
    pub attack: Duration,

    /// Decay in seconds
    #[structopt(long, parse(try_from_str = parse_duration), default_value = "0.05")]
    pub decay: Duration,

    /// Sustain level
    #[structopt(long, default_value = "0.9")]
    pub sustain: f32,

    /// Release in seconds
    #[structopt(long, parse(try_from_str = parse_duration), default_value = "1.0")]
    pub release: Duration,

    #[structopt(long, default_value = "4.0")]
    pub corners: f32,

    #[structopt(long, default_value = "0.0")]
    pub mod_rate: f32,

    #[structopt(long, default_value = "0.0")]
    pub mod_amount: f32,
}

/// Get and also validate CLI options
pub fn getopts() -> Opts {
    Opts::from_args()
}

fn parse_sample_rate(input: &str) -> Result<SampleRate> {
    Ok(SampleRate(u32::from_str(input)?))
}

fn parse_duration(input: &str) -> Result<Duration> {
    let seconds = f32::from_str(input)?;
    // Dont want no divide by zero errors, fix it up here, dont worry about it later ;-)
    Ok(Duration::from_secs_f32(seconds.max(f32::EPSILON)))
}
