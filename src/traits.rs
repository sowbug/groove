use crate::{clock::Clock, common::MonoSample, messages::MessageBounds};

pub trait IsController: Updateable + Terminates + HasUid + std::fmt::Debug {}
pub trait IsEffect: TransformsAudio + Updateable + HasUid + std::fmt::Debug {}
pub trait IsInstrument: SourcesAudio + Updateable + HasUid + std::fmt::Debug {}

#[derive(Debug)]
pub enum BoxedEntity<M> {
    Controller(Box<dyn IsController<Message = M>>),
    Effect(Box<dyn IsEffect<Message = M>>),
    Instrument(Box<dyn IsInstrument<Message = M>>),
}

pub trait Updateable {
    type Message: MessageBounds;

    #[allow(unused_variables)]
    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::none()
    }
    #[allow(unused_variables)]
    fn handle_message(&mut self, clock: &Clock, message: Self::Message) {
        todo!()
    }
    #[allow(unused_variables)]
    fn param_id_for_name(&self, name: &str) -> usize {
        todo!()
    }
}
pub trait HasUid {
    fn uid(&self) -> usize;
    fn set_uid(&mut self, uid: usize);
}

/// Provides audio in the form of digital samples.
pub trait SourcesAudio: std::fmt::Debug {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample;
}

/// TransformsAudio can be thought of as SourcesAudio + SinksAudio, but it's an
/// important third trait because it exposes the business logic that happens
/// between the sinking and sourcing, which is useful for testing.
pub trait TransformsAudio: std::fmt::Debug {
    fn transform_audio(&mut self, clock: &Clock, input_sample: MonoSample) -> MonoSample;
}

// Something that Terminates has a point in time where it would be OK never
// being called or continuing to exist.
//
// If you're required to implement Terminates, but you don't know when you need
// to terminate, then you should always return true. For example, an arpeggiator
// is a WatchesClock, which means it is also a Terminates, but it would be happy
// to keep responding to MIDI input forever. It should return true.
//
// The reason to choose true rather than false is that the caller uses
// is_finished() to determine whether a song is complete. If a Terminates never
// returns true, the loop will never end. Thus, "is_finished" is more like "is
// unaware of any reason to continue existing" rather than "is certain there is
// no more work to do."
pub trait Terminates: std::fmt::Debug {
    fn is_finished(&self) -> bool;
}

#[derive(Debug)]
pub struct EvenNewerCommand<T>(pub Internal<T>);

#[derive(Debug)]
pub enum Internal<T> {
    None,
    Single(T),
    Batch(Vec<T>),
}

impl<T> EvenNewerCommand<T> {
    pub const fn none() -> Self {
        Self(Internal::None)
    }

    pub const fn single(action: T) -> Self {
        Self(Internal::Single(action))
    }

    pub fn batch(commands: impl IntoIterator<Item = EvenNewerCommand<T>>) -> Self {
        let mut batch = Vec::new();

        for EvenNewerCommand(command) in commands {
            match command {
                Internal::None => {}
                Internal::Single(command) => batch.push(command),
                Internal::Batch(commands) => batch.extend(commands),
            }
        }

        Self(Internal::Batch(batch))
    }
}

#[cfg(test)]
pub mod tests {
    use super::{
        EvenNewerCommand, HasUid, IsEffect, IsInstrument, Updateable, SourcesAudio,
        TransformsAudio,
    };
    use crate::{
        common::{MonoSample, MONO_SAMPLE_SILENCE},
        instruments::oscillators::Oscillator,
        messages::{tests::TestMessage, MessageBounds},
        midi::{MidiChannel, MidiUtils},
        settings::patches::WaveformType,
        Clock,
    };
    use midly::MidiMessage;
    use rand::random;
    use std::{marker::PhantomData, str::FromStr};
    use strum_macros::{Display, EnumString};

    #[derive(Display, Debug, EnumString)]
    #[strum(serialize_all = "kebab_case")]
    pub(crate) enum TestInstrumentControlParams {
        // -1.0 is Sawtooth, 1.0 is Square, anything else is Sine.
        Waveform,
    }

    /// A simple implementation of IsInstrument that's useful for testing and
    /// debugging. Uses a default Oscillator to produce sound, and its
    /// "envelope" is just a boolean that responds to MIDI NoteOn/NoteOff.
    ///
    /// To act as a controller target, it has one parameter: Oscillator
    /// waveform.
    #[derive(Debug)]
    pub struct TestInstrument<M: MessageBounds> {
        uid: usize,

        oscillator: Oscillator,
        pub is_playing: bool,
        midi_channel: MidiChannel,
        pub received_count: usize,
        pub handled_count: usize,

        pub debug_messages: Vec<(f32, MidiChannel, MidiMessage)>,

        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> IsInstrument for TestInstrument<M> {}
    impl<M: MessageBounds> Updateable for TestInstrument<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }

        fn param_id_for_name(&self, param_name: &str) -> usize {
            if let Ok(param) = TestInstrumentControlParams::from_str(param_name) {
                param as usize
            } else {
                usize::MAX
            }
        }
    }
    impl Updateable for TestInstrument<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                Self::Message::UpdateF32(_param_id, value) => {
                    self.set_waveform(value);
                }
                Self::Message::Midi(channel, message) => {
                    self.handle_midi(clock, channel, message);
                }
                _ => todo!(),
            }
            EvenNewerCommand::none()
        }
    }
    impl<M: MessageBounds> HasUid for TestInstrument<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }
    impl<M: MessageBounds> Default for TestInstrument<M> {
        fn default() -> Self {
            Self {
                uid: Default::default(),

                oscillator: Default::default(),
                is_playing: Default::default(),
                midi_channel: Self::TEST_MIDI_CHANNEL,
                received_count: Default::default(),
                handled_count: Default::default(),
                debug_messages: Default::default(),
                _phantom: Default::default(),
            }
        }
    }
    impl<M: MessageBounds> TestInstrument<M> {
        pub const TEST_MIDI_CHANNEL: u8 = 42;

        pub fn new() -> Self {
            Self {
                midi_channel: Self::TEST_MIDI_CHANNEL,
                ..Default::default()
            }
        }
        pub fn new_with(midi_channel: MidiChannel) -> Self {
            Self {
                midi_channel,
                ..Default::default()
            }
        }

        #[allow(dead_code)]
        pub fn dump_messages(&self) {
            dbg!(&self.debug_messages);
        }

        fn handle_midi(&mut self, clock: &Clock, channel: MidiChannel, message: MidiMessage) {
            assert_eq!(self.midi_channel, channel);
            self.debug_messages.push((clock.beats(), channel, message));
            self.received_count += 1;

            match message {
                MidiMessage::NoteOn { key, vel: _ } => {
                    self.is_playing = true;
                    self.oscillator
                        .set_frequency(MidiUtils::note_to_frequency(key.as_int()));
                }
                MidiMessage::NoteOff { key: _, vel: _ } => {
                    self.is_playing = false;
                }
                _ => {}
            }
        }

        fn waveform(&self) -> f32 {
            match self.oscillator.waveform() {
                WaveformType::Sawtooth => -1.0,
                WaveformType::Square => 1.0,
                _ => 0.0,
            }
        }

        fn set_waveform(&mut self, value: f32) {
            self.oscillator.set_waveform(if value == -1.0 {
                WaveformType::Sawtooth
            } else {
                if value == 1.0 {
                    WaveformType::Square
                } else {
                    WaveformType::Sine
                }
            });
        }
    }
    impl<M: MessageBounds> SourcesAudio for TestInstrument<M> {
        fn source_audio(&mut self, clock: &Clock) -> MonoSample {
            if self.is_playing {
                self.oscillator.source_audio(clock)
            } else {
                MONO_SAMPLE_SILENCE
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestEffect<M: MessageBounds> {
        uid: usize,
        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> IsEffect for TestEffect<M> {}
    impl<M: MessageBounds> TransformsAudio for TestEffect<M> {
        fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
            -input_sample
        }
    }
    impl<M: MessageBounds> Updateable for TestEffect<M> {
        type Message = M;
    }
    impl<M: MessageBounds> HasUid for TestEffect<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }

    // TODO: restore tests that test basic trait behavior, then figure out how
    // to run everyone implementing those traits through that behavior. For now,
    // this one just tests that a generic instrument doesn't panic when accessed
    // for non-consecutive time slices.
    #[test]
    fn test_sources_audio_random_access() {
        let mut instrument = TestInstrument::<TestMessage>::default();
        for _ in 0..100 {
            let mut clock = Clock::new();
            clock.debug_set_samples(random());
            let _ = instrument.source_audio(&clock);
        }
    }
}
