#![feature(unboxed_closures)]
#![feature(trait_alias)]
#![feature(never_type)]
#![feature(box_syntax)]
#![feature(destructuring_assignment)]
#![feature(str_split_once)]

pub mod opts;
pub mod synthesis;
pub mod vec2;

use std::{
    thread,
    time::{self, Duration, Instant},
};

use anyhow::{anyhow, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Host, Sample, SampleFormat,
};
use midir::{MidiInput, MidiInputPort};

const MIDI_INPUT_NAME: &str = env!("CARGO_PKG_NAME");

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
    let port = if let Some(name) = &opts.midi_port {
        log::debug!("Connecting to port with name {}", name);
        ports
            .iter()
            .find(|port| {
                input
                    .port_name(port)
                    .map(|port_name| port_name.starts_with(name))
                    .unwrap_or(false)
            })
            .ok_or(anyhow!("No MIDI port named {}", name))?
    } else if let Some(index) = opts.midi_port_index {
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
                opts.master_gain,
            ),
            errfun,
        ),
        SampleFormat::I16 => device.build_output_stream(
            &config,
            do_audio::<i16>(
                config.channels as usize,
                config.sample_rate.0 as f32,
                opts.master_gain,
            ),
            errfun,
        ), // run::<i16>(device, config)?,
        SampleFormat::U16 => device.build_output_stream(
            &config,
            do_audio::<u16>(
                config.channels as usize,
                config.sample_rate.0 as f32,
                opts.master_gain,
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
    use synthesis::*;
    let audio = move |sample| {
        let point = polygon(4.7, phase(sample, samplerate, 500.0));
        vec2::scale(point, 0.5)
    };

    let mut envelope = LinearPluck::new(Duration::from_secs(1));

    let master_gain = gain;
    let mut sample_counter = 0;
    move |data: &mut [T], _info: &cpal::OutputCallbackInfo| {
        for frame in data.chunks_mut(2) {
            let (l, r) = audio(sample_counter);
            let now = Instant::now();
            let amp = envelope.level(now);
            if amp == 0.0 {
                envelope.trigger(now);
            }
            for (dst, src) in frame.iter_mut().zip(&[l, r]) {
                *dst = Sample::from(&(src * amp * master_gain))
            }

            sample_counter += 1;
        }
    }
}
