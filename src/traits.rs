use crate::{
    clock::Clock,
    common::MonoSample,
    gui::{IsViewable, ViewableMessage},
    messages::MessageBounds,
};

pub trait NewIsController: NewUpdateable + Terminates + HasUid + std::fmt::Debug {}
pub trait NewIsEffect: TransformsAudio + NewUpdateable + HasUid + std::fmt::Debug {}
pub trait NewIsInstrument: SourcesAudio + NewUpdateable + HasUid + std::fmt::Debug {}

#[derive(Debug)]
pub enum BoxedEntity<M> {
    Controller(Box<dyn NewIsController<Message = M>>),
    Effect(Box<dyn NewIsEffect<Message = M>>),
    Instrument(Box<dyn NewIsInstrument<Message = M>>),
}

pub trait NewUpdateable {
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
    fn param_id_for_name(&self, param_name: &str) -> usize {
        usize::MAX
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

pub trait MakesIsViewable: std::fmt::Debug {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>>;
}

#[cfg(test)]
pub mod tests {
    use super::{
        EvenNewerCommand, HasUid, NewIsEffect, NewIsInstrument, NewUpdateable, SourcesAudio,
        TransformsAudio,
    };
    use crate::{
        common::{MonoSample, MONO_SAMPLE_SILENCE},
        instruments::oscillators::Oscillator,
        messages::{tests::TestMessage, MessageBounds},
        midi::{MidiChannel, MidiUtils},
        Clock,
    };
    use midly::MidiMessage;
    use rand::random;
    use std::marker::PhantomData;

    #[derive(Debug)]
    pub struct TestInstrument<M: MessageBounds> {
        uid: usize,

        sound_source: Oscillator,
        pub is_playing: bool,
        midi_channel: MidiChannel,
        pub received_count: usize,
        pub handled_count: usize,

        pub debug_messages: Vec<(f32, MidiChannel, MidiMessage)>,

        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsInstrument for TestInstrument<M> {}
    impl<M: MessageBounds> NewUpdateable for TestInstrument<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }
    }
    impl NewUpdateable for TestInstrument<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Midi(channel, message) => {
                    self.new_handle_midi(clock, channel, message);
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

                sound_source: Default::default(),
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

        fn new_handle_midi(&mut self, clock: &Clock, channel: MidiChannel, message: MidiMessage) {
            assert_eq!(self.midi_channel, channel);
            self.debug_messages.push((clock.beats(), channel, message));
            self.received_count += 1;

            match message {
                MidiMessage::NoteOn { key, vel: _ } => {
                    self.is_playing = true;
                    self.sound_source
                        .set_frequency(MidiUtils::note_to_frequency(key.as_int()));
                }
                MidiMessage::NoteOff { key: _, vel: _ } => {
                    self.is_playing = false;
                }
                _ => {}
            }
        }
    }
    impl<M: MessageBounds> SourcesAudio for TestInstrument<M> {
        fn source_audio(&mut self, clock: &Clock) -> MonoSample {
            if self.is_playing {
                self.sound_source.source_audio(clock)
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
    impl<M: MessageBounds> NewIsEffect for TestEffect<M> {}
    impl<M: MessageBounds> TransformsAudio for TestEffect<M> {
        fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
            -input_sample
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestEffect<M> {
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
