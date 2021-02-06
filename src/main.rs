#![feature(unboxed_closures)]
#![feature(trait_alias)]
#![feature(never_type)]
#![feature(box_syntax)]
#![feature(destructuring_assignment)]
#![feature(str_split_once)]

mod constants;
mod engine;
mod maths;
mod opts;
mod queue;
mod synthesis;
mod util;
pub use maths::vec2;

use std::thread;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Host;
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

    // TODO make fallback options? or just error out if not supported config
    // TODO Ok yeah we definitely should validate before this point, alsa just spits out "INVALID ARGUMENT >:("
    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: opts.sample_rate,
        buffer_size: match opts.buffer_size {
            Some(frames) => cpal::BufferSize::Fixed(frames),
            None => cpal::BufferSize::Default,
        },
    };

    log::info!("channels: {}", config.channels);
    log::info!("sample rate: {}", config.sample_rate.0);
    log::info!(
        "buffer size: {}",
        match config.buffer_size {
            cpal::BufferSize::Fixed(frames) => frames.to_string(),
            cpal::BufferSize::Default => "default".to_string(),
        }
    );

    let (sender, receiver) = std::sync::mpsc::channel();

    let _connection = input
        .connect(&port, MIDI_INPUT_NAME, engine::handle_midi_input, sender)
        .map_err(|e| {
            anyhow!(
                "Couldn't connect to MIDI output port \"{}\": {}",
                port_name,
                e
            )
        })?;

    let errfun = |err| log::error!("Audio output stream error: {}", err);
    let stream = device.build_output_stream(
        &config,
        engine::do_audio::<f32>(
            config.channels as usize,
            config.sample_rate,
            &opts,
            receiver,
        ),
        errfun,
    )?;

    stream.play()?;
    loop {
        thread::park();
        log::trace!("main thread unparked");
    }
}
