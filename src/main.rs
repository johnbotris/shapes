#![feature(unboxed_closures)]
#![feature(trait_alias)]
#![feature(never_type)]
#![feature(box_syntax)]
#![feature(destructuring_assignment)]

mod opt;

use core::f32::consts::PI;
use crate::opt::{getopts, Opts, Command};
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

    let audio = move |sample| {
            let (mut left, mut right) = (0.0, 0.0);
            let numvoices = 1;
            let generator = (1 .. numvoices + 1)
                .map(|v| (25.0, v as f32 / numvoices as f32))
                .map(|(freq, amp)| vec_mul(circle(sample, freq, samplerate), amp));

            for (l, r) in generator {
                (left, right) = (left + l, right + r);
            }
            (left, right)
        };

    let master_gain = gain;
    let mut sample_counter = 0;
    move |data: &mut [T], _info: &cpal::OutputCallbackInfo| {
        for frame in data.chunks_mut(2) {
            let (l, r) = audio(sample_counter);

            for (dst, src) in frame.iter_mut().zip(&[l, r]) {
                *dst = Sample::from(&(src * master_gain))
            }

            sample_counter += 1;
        }
    }
}

fn vec_mul((x, y): (f32, f32), amt: f32) -> (f32, f32) {
    (x * amt, y * amt)
}

fn circle(sample: u64, freq: f32, samplerate: f32) -> (f32, f32) {
    let modsample = sample % (samplerate / freq) as u64;
    let theta = 2.0 * PI * modsample as f32 * freq / samplerate;
    (f32::sin(theta), f32::cos(theta))
}

fn binaural_beats(sample: u64, f1: f32, f2: f32, samplerate: f32) -> (f32, f32) {
    (0.0, 0.0)
}

