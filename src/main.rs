#![feature(unboxed_closures)]
#![feature(trait_alias)]
#![feature(never_type)]
#![feature(box_syntax)]

mod opt;
mod ugen;

use crate::opt::{getopts, Opts, Command};
use crate::ugen::*;
use std::{thread, time};

use anyhow::{ anyhow, Result};
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use cpal::{Sample, SampleFormat, StreamConfig, Host, Device};

fn main() -> Result<()> {

    let opts = getopts()?;

    let host = cpal::default_host();

    match opts.command {
        Some(Command::Run) | None => run(host, opts)?,
        Some(Command::ListDevices) => {
            for (idx, device) in host.output_devices()?.enumerate() {
                println!("{}: {}", idx, device.name().unwrap_or("unknown".to_string()));
            }
            Ok(())
        },
    }
}

fn run(host: Host, opts: Opts) -> Result<!> {

    let device = if opts.device == "default" {
        host.default_output_device()
    } else {
        host.output_devices()?
            .find(|d| d.name().map(|name| name == opts.device)
              .unwrap_or(false))
    }.ok_or(anyhow!("Couldn't connect to device \"{}\"", opts.device))?;

    println!("connected to device: \"{}\"", device.name().unwrap_or(String::from("unknown")));

    let supported_config = device.supported_output_configs()?
        .filter(|config| config.sample_format() == SampleFormat::F32)
        .filter(|config| config.min_sample_rate() <= opts.sample_rate && config.max_sample_rate() >= opts.sample_rate)
        .find(|config| config.channels() == opts.channels)
        .map(|config| config.with_sample_rate(opts.sample_rate))
        .or(device.default_output_config().ok())
        .ok_or(anyhow!("no supported output device config"))?;


    let sample_format = supported_config.sample_format();
    let config = supported_config.config();

    println!("channels: {}", config.channels);
    println!("sample rate: {}", config.sample_rate.0);
    println!("sample format: {:?}", sample_format);

    let errfun = |err| eprintln!("an error occurred on the output audio stream: {}", err);
    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(&config, do_audio::<f32>(config.channels as usize, config.sample_rate.0 as f32, opts.gain), errfun),
        SampleFormat::I16 => device.build_output_stream(&config, do_audio::<i16>(config.channels as usize, config.sample_rate.0 as f32, opts.gain), errfun), // run::<i16>(device, config)?,
        SampleFormat::U16 => device.build_output_stream(&config, do_audio::<u16>(config.channels as usize, config.sample_rate.0 as f32, opts.gain), errfun), // run::<u16>(device, config)?,
    }?;

    stream.play()?;
    loop {
        thread::sleep(time::Duration::from_secs(5));
    }
}

fn do_audio<T: Sample>(channel_count: usize, samplerate: f32, gain: f32) -> impl FnMut(&mut [T], &cpal::OutputCallbackInfo) -> () {

    let base = 110.0;
    let voices = 100;
    let interval = 0.5;

    let mut ugens: Vec<Box<dyn Ugen>> = (0 .. voices)
        .map(|v| v as f32)
        .map(|v| box(SinUgen::new(base + interval * v, v / voices as f32, 0.0, samplerate)) as Box<dyn Ugen>)
        .collect();

    let mut channel = box(move |sample| {
            let mut sum = 0.0;
            for ugen in &mut ugens {
                sum += ugen.gen(sample);
            }
            sum
        });


    let mut channels: Vec<Box<dyn Ugen>> = vec![ 
        channel 
    ];

    let master_gain = gain;
    let mut sample_counter = 0;
    move |data: &mut [T], _info: &cpal::OutputCallbackInfo| {
        for frame in data.chunks_mut(channel_count) {
            for (idx, channel) in frame.iter_mut().enumerate() {
                let value = match channels.get_mut(idx) {
                    Some(ugen) => ugen.gen(sample_counter) * master_gain,
                    None => 0.0
                };
                *channel = Sample::from(&value)
            }
            sample_counter += 1;
        }
    }
}

