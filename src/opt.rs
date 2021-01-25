use cpal::{ ChannelCount, SampleRate };
use structopt::StructOpt;
use anyhow::{anyhow, Result};
use std::str::FromStr;

#[derive(StructOpt, Debug)] #[structopt(about)]
pub struct Opts {
    /// How many output channels
    #[structopt(short, long, default_value = "2")]
    pub channels: ChannelCount,

    #[structopt(short, long, default_value = "48000", parse(try_from_str = parse_sample_rate))]
    pub sample_rate: SampleRate,

    #[structopt(short, long, default_value = "pulse")]
    pub device: String,


    #[structopt(subcommand)]
    pub command: Option<Command>,

    /// Output gain level
    #[structopt(short, long, default_value = "0.5")]
    pub gain: f32

}

#[derive(StructOpt, Debug)]
pub enum Command {
    /// [Default]
    Run,

    ListDevices
}

/// Get and also validate CLI options
pub fn getopts() -> Result<Opts> {
    let opts = Opts::from_args();

    Ok(opts)
}

fn parse_sample_rate(input: &str) -> Result<SampleRate> {
    Ok(SampleRate(u32::from_str(input)?))
}
