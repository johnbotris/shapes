#![feature(unboxed_closures)]
#![feature(trait_alias)]
#![feature(never_type)]
#![feature(box_syntax)]
#![feature(destructuring_assignment)]
#![feature(str_split_once)]

use core::f32::consts::PI;
use std::{thread, time};

use anyhow::{anyhow, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Host, Sample, SampleFormat,
};
use midir::{MidiInput, MidiInputPort};

const MIDI_INPUT_NAME: &str = env!("CARGO_PKG_NAME");

mod opts {
    use anyhow::Result;
    use cpal::{ChannelCount, SampleRate};
    use std::str::FromStr;
    use structopt::StructOpt;

    #[derive(StructOpt, Debug)]
    #[structopt(about)]
    pub struct Opts {
        /// How many output channels
        #[structopt(short, long, default_value = "2")]
        pub channels: ChannelCount,

        #[structopt(short, long, default_value = "48000", parse(try_from_str = parse_sample_rate))]
        pub sample_rate: SampleRate,

        /// Output device to connect to
        #[structopt(short, long, default_value = "pulse")]
        pub device: String,

        /// Name of the MIDI input port to connect to
        #[structopt(short = "p", long)]
        pub port_name: Option<String>,

        /// Index of the MIDI input port to connect to
        #[structopt(short = "i", long)]
        pub port_index: Option<usize>,

        /// Master gain factor
        #[structopt(short, long, default_value = "0.5")]
        pub gain: f32,

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
    }

    /// Get and also validate CLI options
    pub fn getopts() -> Opts {
        Opts::from_args()
    }

    fn parse_sample_rate(input: &str) -> Result<SampleRate> {
        Ok(SampleRate(u32::from_str(input)?))
    }
}

fn init_logging(opts: &opts::Opts) {
    use log::LevelFilter::*;
    use std::cmp::{max, min};

    let default = 3;
    let verbose = opts.verbose as i64;
    let quiet = opts.quiet as i64;
    let level = min(max(default + verbose - quiet, 0), 5);
    assert!(level <= 5);
    match simple_logger::SimpleLogger::new()
        .with_level([Off, Error, Warn, Info, Debug, Trace][level as usize])
        .init()
    {
        Ok(_) => log::trace!("Logging initialized"),
        Err(e) => eprintln!("Failed to initialize logging: {}", e),
    }
}

fn main() {
    let opts = opts::getopts();

    let host = cpal::default_host();

    if opts.list_outputs {
        let devices = host.output_devices();
        match devices {
            Ok(devices) => {
                // ALSA generates a fuckton of annoying and useless error output on startup.
                // Maybe could be fixed through proper system configuration but if this is what happens with
                // the default configuration it's probably better to just suppress
                let _alsa_gag = gag::Gag::stderr().unwrap();
                println!("Available audio outputs");
                for (idx, device) in devices.enumerate() {
                    println!(
                        "{}: {}",
                        idx,
                        device.name().unwrap_or("unknown".to_string())
                    );
                }
            }
            Err(e) => eprintln!("Error: Unable to enumerate devices: {}", e),
        };
    }

    if opts.list_inputs {
        let input = MidiInput::new(MIDI_INPUT_NAME).unwrap();
        let ports = input.ports();

        if opts.list_outputs {
            println!("")
        }
        println!("Available MIDI inputs:");
        for (i, port) in ports.iter().enumerate() {
            let name = input.port_name(port).unwrap_or(String::from("<unknown>"));
            if let Some((device, _name)) = name.split_once(':') {
                println!("{}: {}", i, device);
            };
        }
    }

    if opts.list_inputs || opts.list_outputs {
        return;
    }

    init_logging(&opts);

    match run(host, opts) {
        Ok(_) => {} // unreachable
        Err(e) => log::error!("Fatal error: {}", e),
    }
}

fn run(host: Host, opts: opts::Opts) -> Result<!> {
    let mut input = MidiInput::new(MIDI_INPUT_NAME)?;
    input.ignore(midir::Ignore::None);
    let ports: &[MidiInputPort] = &input.ports();
    let port = if let Some(name) = &opts.port_name {
        log::debug!("Connecting to port with name {}", name);
        ports
            .iter()
            .find(|port| input.port_name(port) == Ok(name.clone()))
            .ok_or(anyhow!("No MIDI port named {}", name))?
    } else if let Some(index) = opts.port_index {
        log::debug!("Connecting to port with index {}", index);
        ports
            .get(index)
            .ok_or(anyhow!("Port index {} out of range", index))?
    } else {
        log::debug!("Connecting to first available port");
        match ports {
            [] => Err(anyhow!("No available MIDI ports")),
            [port, ..] => Ok(port),
        }?
    };

    let port_name = input.port_name(&port).unwrap_or(String::from("<unknown>"));

    let _connection = input
        .connect(&port, MIDI_INPUT_NAME, handle_midi_input, ())
        .map_err(|e| {
            anyhow!(
                "Couldn't connect to MIDI output port \"{}\": {}",
                port_name,
                e
            )
        })?;

    log::info!("Reading MIDI input from {}", port_name);

    let device = {
        let _alsa_gag = gag::Gag::stderr().unwrap();
        if opts.device == "default" {
            host.default_output_device()
        } else {
            host.output_devices()?
                .find(|d| d.name().map(|name| name == opts.device).unwrap_or(false))
        }
        .ok_or(anyhow!(
            "Couldn't connect to output device \"{}\"",
            opts.device
        ))?
    };

    log::info!(
        "Outputting to \"{}\"",
        device.name().unwrap_or(String::from("unknown"))
    );

    let supported_config = {
        let _alsa_gag = gag::Gag::stderr().unwrap();
        device
            .supported_output_configs()?
            .filter(|config| config.sample_format() == SampleFormat::F32)
            .filter(|config| {
                config.min_sample_rate() <= opts.sample_rate
                    && config.max_sample_rate() >= opts.sample_rate
            })
            .find(|config| config.channels() == opts.channels)
            .map(|config| config.with_sample_rate(opts.sample_rate))
            .or(device.default_output_config().ok())
            .ok_or(anyhow!("no supported output device config"))?
    };

    let sample_format = supported_config.sample_format();
    let config = supported_config.config();

    log::info!("channels: {}", config.channels);
    log::info!("sample rate: {}", config.sample_rate.0);
    log::info!("sample format: {:?}", sample_format);

    let errfun = |err| log::warn!("an error occurred on the output audio stream: {}", err);
    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(
            &config,
            do_audio::<f32>(
                config.channels as usize,
                config.sample_rate.0 as f32,
                opts.gain,
            ),
            errfun,
        ),
        SampleFormat::I16 => device.build_output_stream(
            &config,
            do_audio::<i16>(
                config.channels as usize,
                config.sample_rate.0 as f32,
                opts.gain,
            ),
            errfun,
        ), // run::<i16>(device, config)?,
        SampleFormat::U16 => device.build_output_stream(
            &config,
            do_audio::<u16>(
                config.channels as usize,
                config.sample_rate.0 as f32,
                opts.gain,
            ),
            errfun,
        ), // run::<u16>(device, config)?,
    }?;

    stream.play()?;
    loop {
        thread::sleep(time::Duration::from_secs(5));
    }
}

fn handle_midi_input(timestamp: u64, message: &[u8], data: &mut ()) {
    log::trace!(
        "Midi input received: timstamp: {}, message: {:?}",
        timestamp,
        message
    );
}

fn do_audio<T: Sample>(
    channel_count: usize,
    samplerate: f32,
    gain: f32,
) -> impl FnMut(&mut [T], &cpal::OutputCallbackInfo) -> () {
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

use vec2::Vec2;

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

    use super::{vec2::*, *};

    #[test]
    fn squares() {
        for i in 0..50 {
            square(i as f32 / 50.0);
        }
    }
}
