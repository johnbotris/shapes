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
        // let (mut left, mut right) = (0.0, 0.0);
        // let numvoices = 1;
        // let generator = (1 .. numvoices + 1)
        //     .map(|v| (125.0, v as f32 / numvoices as f32))
        //     .map(|(freq, amp)| vec2::scale(square(phase(sample, samplerate, freq)), amp));
        // for (l, r) in generator {
        //     (left, right) = (left + l, right + r);
        // }


        let point = polygon(4.7, phase(sample, samplerate, 500.0));
        vec2::scale(point, 0.5)
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


fn phase(sample: u64, samplerate: f32, freq: f32) -> f32 {
    (sample % (samplerate / freq) as u64) as f32 * freq / samplerate
}

fn circle(p: f32) -> Vec2 {
    let theta = 2.0 * PI * p;
    (f32::sin(theta), f32::cos(theta))
}

fn polygon(n: f32, p: f32) -> Vec2 {

    let step = 1.0/ n;
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

use vec2::{Vec2};

mod vec2 {
    pub type Vec2 = (f32, f32);

    pub fn lerp(a: Vec2, b: Vec2, alpha: f32) -> Vec2 {
        add(scale(a, alpha), scale(b, 1.0 - alpha))
    }

    pub fn add(a: Vec2, b: Vec2) -> Vec2 {
        (a.0 + b.0, a.1 + b.1)
    }

    pub fn scale(v: Vec2, s: f32) -> Vec2 {
        (v.0 * s, v.1 * s)
    }

}


#[cfg(test)]
mod test {

    use super::*;
    use super::vec2::*;

    #[test]
    fn squares() {

        for i in 0 .. 50 {
            square(i as f32/ 50.0);
        }

    }
}
