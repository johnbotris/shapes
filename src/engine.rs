use crate::constants::*;
use crate::opts::Opts;
use crate::util::SampleTimer;
use crate::vec2;

use std::convert::TryFrom;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cpal::{Sample, SampleRate};
use wmidi::{MidiMessage, Note, U7};

pub enum Message {
    /// Note, Velocity except velocity is a value between 0 and 1
    NoteOn(wmidi::Note, f32),
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

    // TODO check if channel is right channel?
    match midi {
        MidiMessage::NoteOn(channel, note, velocity) => {
            // TODO We should get the level as the logarithm cause i think linearly mapping velocity doesn't sound right
            let level = u8::from(velocity) as f32 / 127.0;
            sender.send(Message::NoteOn(note, level)).unwrap();
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
    let corners = opts.corners;
    let mod_rate = opts.mod_rate;
    let mod_amount = opts.mod_amount;

    let mut voices = (0..num_voices)
        .map(|_| Voice {
            note: Note::C0,
            level: 0.0,
            envelope: Envelope::new(attack, decay, sustain, release),
            lfo_timer: SampleTimer::new(samplerate.0),
        })
        .collect::<Vec<Voice>>();

    let mut next_voice_idx = 0;

    let mut audio = move |timer: &SampleTimer| {
        while let Ok(message) = receiver.try_recv() {
            match message {
                Message::NoteOn(note, level) => {
                    let voice: &mut Voice = match voices.iter_mut().find(|v| v.note == note) {
                        Some(voice) => voice,
                        None => {
                            let ref mut voice = voices[next_voice_idx % num_voices];
                            next_voice_idx += 1;
                            voice
                        }
                    };

                    voice.note = note;
                    voice.level = level;
                    voice.envelope.hold(timer);
                    voice.lfo_timer.reset();
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

        for voice in voices.iter_mut() {
            let level = voice.envelope.get(timer);
            if level > 0.0 {
                let lfo = f32::sin(2.0 * core::f32::consts::PI * phase(mod_rate, &voice.lfo_timer))
                    * mod_amount;
                let (l, r) = vec2::scale(
                    polygon(corners + lfo, phase(voice.note.to_freq_f32(), timer)),
                    level * voice.level,
                );

                voice.lfo_timer += 1;
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
