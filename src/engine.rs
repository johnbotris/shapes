use crate::util::SampleCounter;
use crate::vec2;

use std::convert::TryFrom;
use std::time::{Duration, Instant};

use cpal::{Sample, SampleRate};
use wmidi::MidiMessage;

pub fn handle_midi_input(timestamp: u64, message: &[u8], state: &mut ()) {
    log::trace!(
        "Midi input received: timstamp: {}, message: {:?}",
        timestamp,
        message
    );

    let midi = match MidiMessage::try_from(message) {
        Ok(msg) => msg,
        Err(err) => {
            log::warn!("Error parsing MIDI message: {}", err);
            return;
        }
    };

    match midi {
        MidiMessage::NoteOn(channel, note, velocity) => {
            println!("What? huh??");
        }
        MidiMessage::NoteOff(channel, note, velocity) => {
            println!("ok bye");
        }
        _ => {}
    }
}

pub fn do_audio<T: Sample>(
    channel_count: usize,
    samplerate: SampleRate,
    gain: f32,
) -> impl FnMut(&mut [T], &cpal::OutputCallbackInfo) -> () {
    use crate::synthesis::*;

    let mut counter = SampleCounter::new(samplerate.0);
    let envelope_duration = Duration::from_secs(1);
    let pitch = 125.67;

    let mut audio = move |sample| {
        let point = circle(phase(pitch, &counter));
        let level = envelope::linear_pluck(envelope_duration, &counter);
        let (l, r) = vec2::scale(point, level);
        counter.inc();
        if counter.get_secs() > envelope_duration.as_secs_f32() + 0.1 {
            counter.reset();
        }
        (l, r)
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
