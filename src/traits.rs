use crate::{
    clock::Clock,
    common::{MonoSample, Ww},
    controllers::BigMessage,
    gui::{IsViewable, ViewableMessage},
    midi::{MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL, MIDI_CHANNEL_RECEIVE_NONE},
};
use std::collections::HashMap;

pub trait NewIsController: NewUpdateable + Terminates + HasUid + std::fmt::Debug {}
pub trait NewIsEffect: TransformsAudio + NewUpdateable + HasUid + std::fmt::Debug {}
pub trait NewIsInstrument: SourcesAudio + NewUpdateable + HasUid + std::fmt::Debug {}
pub trait MessageBounds: Clone + std::fmt::Debug + Default + 'static {} // TODO: that 'static scares me

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

#[deprecated]
pub(crate) trait EvenNewerIsUpdateable: Terminates + std::fmt::Debug {
    type Message;

    fn update(&mut self, message: Self::Message) -> EvenNewerCommand<Self::Message>;

    // Idea: if someone asks for a message generator, then that's the clue we
    // need to register our UID. All that could be managed in that central
    // place.
    fn message_for(&self, param_name: &str) -> Box<dyn MessageGeneratorT<Self::Message>>;
}

// https://boydjohnson.dev/blog/impl-debug-for-fn-type/ gave me enough clues to
// get through this.
pub trait MessageGeneratorT<M>: Fn(f32) -> M {}
impl<F, M> MessageGeneratorT<M> for F where F: Fn(f32) -> M {}
impl<M> std::fmt::Debug for dyn MessageGeneratorT<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MessageGenerator")
    }
}

pub trait MakesIsViewable: std::fmt::Debug {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>>;
}

#[deprecated]
pub trait SourcesMidi {
    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>;
    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>;

    fn midi_output_channel(&self) -> MidiChannel;
    fn set_midi_output_channel(&mut self, midi_channel: MidiChannel);

    fn add_midi_sink(&mut self, channel: MidiChannel, sink: Ww<dyn SinksMidi>) {
        // TODO: is there a good reason for channel != sink.midi_channel()? If
        // not, why is it a param?
        self.midi_sinks_mut().entry(channel).or_default().push(sink);
    }
    fn issue_midi(&self, clock: &Clock, channel: &MidiChannel, message: &MidiMessage) {
        if self.midi_sinks().contains_key(&MIDI_CHANNEL_RECEIVE_ALL) {
            for sink in self.midi_sinks().get(&MIDI_CHANNEL_RECEIVE_ALL).unwrap() {
                if let Some(sink_up) = sink.upgrade() {
                    sink_up.borrow_mut().handle_midi(clock, channel, message);
                }
            }
        }
        if self.midi_sinks().contains_key(channel) {
            for sink in self.midi_sinks().get(channel).unwrap() {
                if let Some(sink_up) = sink.upgrade() {
                    sink_up.borrow_mut().handle_midi(clock, channel, message);
                }
            }
        }
    }
}

#[deprecated]
pub trait SinksMidi: std::fmt::Debug {
    fn midi_channel(&self) -> MidiChannel;
    fn set_midi_channel(&mut self, midi_channel: MidiChannel);

    fn handle_midi(&mut self, clock: &Clock, channel: &MidiChannel, message: &MidiMessage) {
        if self.midi_channel() == MIDI_CHANNEL_RECEIVE_NONE {
            return;
        }
        if self.midi_channel() == MIDI_CHANNEL_RECEIVE_ALL || self.midi_channel() == *channel {
            // TODO: SourcesMidi is already going through trouble to respect
            // channels. Is this redundant?
            self.handle_midi_for_channel(clock, channel, message);
        }
    }
    fn handle_midi_for_channel(
        &mut self,
        clock: &Clock,
        channel: &MidiChannel,
        message: &MidiMessage,
    );
}

/// A WatchesClock is something that needs to be called for every time slice.
/// This sounds like SourcesAudio; indeed SourcesAudio do not (and *cannot*)
/// implement WatchesClock because they're already called on every time slice to
/// provide an audio sample. A WatchesClock has no extrinsic reason to be
/// called, so the trait exists to make sure that whatever intrinsic reason for
/// being called is satisfied.
#[deprecated]
pub trait WatchesClock: std::fmt::Debug + Terminates {
    // type Message; // TODO: figure out how to do this!

    /// WatchesClock::tick() must be called exactly once for every sample, and
    /// implementers can assume that they won't be asked to provide any
    /// information until tick() has been called for the time slice.
    fn tick(&mut self, clock: &Clock) -> Vec<BigMessage>;
    // TODO: should be Box<> so stuff gets handed over more cheaply
}

#[cfg(test)]
pub mod tests {
    use super::WatchesClock;
    use crate::{
        clock::Clock,
        clock::WatchedClock,
        common::{rrc, rrc_clone, rrc_downgrade},
        controllers::AdsrEnvelopeControlParams,
        effects::gain::Gain,
        envelopes::AdsrEnvelope,
        messages::tests::TestMessage,
        midi::MidiUtils,
        oscillators::Oscillator,
        settings::patches::{EnvelopeSettings, WaveformType},
        traits::Terminates,
        utils::{
            tests::{TestClockWatcher, TestSynth},
            Timer, Trigger,
        },
    };

    #[test]
    fn test_clock_watcher() {
        let mut clock = Clock::new_test();
        let mut clock_watcher = TestClockWatcher::<TestMessage>::new(1.0);

        loop {
            clock.tick();
            clock_watcher.tick(&clock);
            if clock_watcher.is_finished() {
                break;
            }
        }
        assert!(clock.seconds() >= 1.0);
    }

    // #[test]
    // fn test_clock_watcher_random_access() {
    //     let mut clock = WatchedClock::new();

    //     let mut watchers = watches_clock_instances_for_testing();
    //     while !watchers.is_empty() {
    //         clock.add_watcher(watchers.pop().unwrap());
    //     }

    //     // Regular start to finish, twice.
    //     for _ in 0..2 {
    //         for _ in 0..100 {
    //             clock.tick();
    //         }
    //         clock.reset();
    //     }

    //     // Backwards.
    //     clock.reset();
    //     for t in 0..100 {
    //         clock.inner_clock_mut().debug_set_samples(t);
    //         clock.tick();
    //     }

    //     // Random.
    //     for _ in 0..100 {
    //         clock.inner_clock_mut().debug_set_samples(random());
    //         clock.tick();
    //     }
    // }

    // /// Add concrete instances of SourcesAudio here for anyone to use for
    // /// testing.
    // fn sources_audio_instances_for_testing() -> Vec<Rrc<dyn SourcesAudio>> {
    //     const MIDI_CHANNEL: MidiChannel = 0;

    //     // If the instance is meaningfully testable after new(), put it here.
    //     let mut sources: Vec<Rrc<dyn SourcesAudio>> = vec![
    //         BiQuadFilter::new_wrapped_with(
    //             &crate::effects::filter::FilterParams::BandPass {
    //                 cutoff: 2343.9,
    //                 bandwidth: 4354.3,
    //             },
    //             13245,
    //         ),
    //         Gain::new_wrapped_with(0.5),
    //     ];

    //     // If the instance needs to be told to play a note, put it here.
    //     let midi_instruments: Vec<Rrc<dyn IsMidiInstrument>> = vec![
    //         DrumkitSampler::new_wrapped_from_files(MIDI_CHANNEL),
    //         Sampler::new_wrapped_with(MIDI_CHANNEL, 10000),
    //         Synth::new_wrapped_with(MIDI_CHANNEL, 44007, SynthPatch::by_name(&PatchName::Piano)),
    //     ];
    //     for instrument in midi_instruments {
    //         instrument.borrow_mut().handle_midi_for_channel(
    //             &Clock::new(),
    //             &0,
    //             &MidiUtils::note_on_c4(),
    //         );
    //         sources.push(instrument);
    //     }

    //     sources
    // }

    // #[test]
    // fn test_sources_audio_random_access() {
    //     let mut orchestrator = TestOrchestrator::new();

    //     let mut sources = sources_audio_instances_for_testing();

    //     while !sources.is_empty() {
    //         let source = sources.pop();
    //         if let Some(source) = source {
    //             orchestrator.add_audio_source(rrc_downgrade(&source));
    //         }
    //     }

    //     for _ in 0..100 {
    //         let mut clock = Clock::new();
    //         clock.debug_set_samples(random());
    //         let _ = orchestrator.main_mixer.source_audio(&clock);
    //     }
    // }
}
