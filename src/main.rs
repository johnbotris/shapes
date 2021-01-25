#![feature(unboxed_closures)]
#![feature(trait_alias)]
#![feature(never_type)]
#![feature(box_syntax)]

mod opt;

use crate::opt::{getopts, Opts, Command};
use core::f32::consts::PI;
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

    let mut ugens = [
        SinUgen::new(220.0, 1.0, 0.0, samplerate),
        SinUgen::new(220.1, 0.9, 0.0, samplerate),
        SinUgen::new(220.2, 0.8, 0.0, samplerate),
        SinUgen::new(220.3, 0.7, 0.0, samplerate),
        SinUgen::new(220.4, 0.6, 0.0, samplerate),
        SinUgen::new(220.5, 0.5, 0.0, samplerate),
        SinUgen::new(220.6, 0.4, 0.0, samplerate),
        SinUgen::new(220.7, 0.3, 0.0, samplerate),
        SinUgen::new(220.8, 0.2, 0.0, samplerate),
        SinUgen::new(220.9, 0.1, 0.0, samplerate),
    ];

    let left = box(move |sample| {
            let mut sum = 0.0;
            for ugen in &mut ugens {
                sum += ugen.gen(sample);
            }
            sum
        });

    let right = box(SinUgen::new(120.0, 1.0, 0.0, samplerate)); 

    let mut channels: Vec<Box<dyn Ugen + Send>> = vec![ 
        left, right
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
            println!("{}", sample_counter);
            sample_counter += 1;
        }
    }
}

/// Generates values of a given T
trait Ugen {
    /// Get the next value
    fn gen(&mut self, sample: usize) -> f32;
}

impl<F> Ugen for F 
where
    F: FnMut(usize) -> f32
{
    fn gen(&mut self, sample: usize) -> f32 {
        self(sample)
    }
}

struct RandomUgen {}

impl Ugen for RandomUgen {
    fn gen(&mut self, _sample: usize) -> f32 {
        rand::random::<f32>() * 2.0 - 1.0
    }
}


struct SinUgen {
    pub freq: f32,
    pub amp: f32,
    pub phase: f32,
    samplerate: f32,
}

impl SinUgen {
    fn new(freq: f32, amp: f32, phase: f32, samplerate: f32) -> Self {
        SinUgen {
            freq, 
            amp, 
            phase,
            samplerate,
        }
    }
}

impl Ugen for SinUgen {
    fn gen(&mut self, sample: usize) -> f32 {
        let samples_per_cycle = sample % (self.samplerate / self.freq) as usize;
        f32::sin(self.phase + (samples_per_cycle as f32) * self.freq * 2.0 * PI / (self.samplerate)) * self.amp 
    }
}

