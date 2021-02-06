use crate::constants::*;
use crate::opts::Opts;
use crate::util::SampleTimer;
use crate::vec2;

use std::convert::TryFrom;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cpal::{Sample, SampleRate};
use wmidi::{MidiMessage, Note};

pub enum Message {
    NoteOn(wmidi::Note),
    NoteOff(wmidi::Note),
}

pub fn handle_midi_input(timestamp: u64, message: &[u8], sender: &mut mpsc::Sender<Message>) {
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
            sender.send(Message::NoteOn(note)).unwrap();
        }
        MidiMessage::NoteOff(channel, note, velocity) => {
            sender.send(Message::NoteOff(note)).unwrap();
        }
        _ => {}
    }
}

pub fn do_audio<T: Sample>(
    channel_count: usize, // TODO
    samplerate: SampleRate,
    opts: &Opts,
    receiver: mpsc::Receiver<Message>,
) -> impl FnMut(&mut [T], &cpal::OutputCallbackInfo) -> () {
    use crate::synthesis::*;

    let envelope_duration = Duration::from_secs(1);

    let num_voices = if opts.voices == 0 {
        MAX_VOICES
    } else {
        (opts.voices as usize).clamp(1, MAX_VOICES)
    };

    let master_gain = opts.master_gain;

    let attack = opts.attack;
    let decay = opts.decay;
    let sustain = opts.sustain;
    let release = opts.release;

    let mut voices = (0..num_voices)
        .map(|_| Voice {
            note: Note::C0,
            envelope: Envelope::new(attack, decay, sustain, release),
        })
        .collect::<Vec<Voice>>();

    let mut next_voice_idx = 0;

    let lfo_rate = 1.0;
    let lfo_amt = 0.0;

    let mut audio = move |timer: &SampleTimer| {
        while let Ok(message) = receiver.try_recv() {
            match message {
                Message::NoteOn(note) => {
                    let voice: &mut Voice = match voices.iter_mut().find(|v| v.note == note) {
                        Some(voice) => voice,
                        None => {
                            let ref mut voice = voices[next_voice_idx % num_voices];
                            next_voice_idx += 1;
                            voice
                        }
                    };

                    voice.note = note;
                    voice.envelope.hold(timer);
                    next_voice_idx += 1;
                }
                Message::NoteOff(note) => {
                    for voice in &mut voices {
                        if voice.note == note {
                            voice.envelope.release(timer);
                        }
                    }
                }
            };
        }

        let (mut left, mut right) = (0.0, 0.0);

        let lfo = f32::sin(phase(lfo_rate, timer)) * lfo_amt;

        for voice in voices.iter() {
            let level = voice.envelope.get(timer);
            if level > 0.0 {
                let (l, r) = vec2::scale(
                    polygon(4.0 + lfo, phase(voice.note.to_freq_f32(), timer)),
                    level,
                );
                left += l;
                right += r;
            }
        }

        (left, right)
    };

    let mut timer = SampleTimer::new(samplerate.0);
    move |data: &mut [T], _info: &cpal::OutputCallbackInfo| {
        for frame in data.chunks_mut(2) {
            let (l, r) = audio(&timer);
            for (dst, src) in frame.iter_mut().zip(&[l, r]) {
                *dst = Sample::from(&(src * master_gain))
            }
            timer += 1;
        }
    }
}
