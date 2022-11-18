use crate::{
    clock::Clock,
    common::{MonoSample, Ww, MONO_SAMPLE_SILENCE},
    control::{BigMessage, SmallMessage, SmallMessageGenerator},
    gui::{IsViewable, ViewableMessage},
    midi::{MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL, MIDI_CHANNEL_RECEIVE_NONE},
};
use std::collections::HashMap;
use std::fmt::Debug;

pub trait NewIsController: NewUpdateable + Terminates + HasUid + Debug {}
pub trait NewIsEffect: TransformsAudio + NewUpdateable + HasUid + Debug {}
pub trait NewIsInstrument: SourcesAudio + NewUpdateable + HasUid + Debug {}
pub trait Message: Clone + Debug + Default + 'static {} // TODO: that 'static scares me

#[derive(Debug)]
pub(crate) enum BoxedEntity<M> {
    Controller(Box<dyn NewIsController<Message = M>>),
    Effect(Box<dyn NewIsEffect<Message = M>>),
    Instrument(Box<dyn NewIsInstrument<Message = M>>),
}

pub trait NewUpdateable {
    type Message;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::none()
    }
    fn param_id_for_name(&self, param_name: &str) -> usize {
        usize::MAX
    }
}
pub trait HasUid {
    fn uid(&self) -> usize;
    fn set_uid(&mut self, uid: usize);
}

/// Provides audio in the form of digital samples.
pub trait SourcesAudio: Debug {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample;
}

/// TransformsAudio can be thought of as SourcesAudio + SinksAudio, but it's an
/// important third trait because it exposes the business logic that happens
/// between the sinking and sourcing, which is useful for testing.
pub trait TransformsAudio: Debug {
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
pub trait Terminates: Debug {
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

/// Can do something with audio samples. When it needs to do its work, it asks
/// its SourcesAudio for their samples.
#[deprecated]
pub trait SinksAudio {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>];
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>>;

    fn add_audio_source(&mut self, source: Ww<dyn SourcesAudio>) {
        self.sources_mut().push(source);
    }

    fn gather_source_audio(&mut self, clock: &Clock) -> MonoSample {
        if self.sources_mut().is_empty() {
            return MONO_SAMPLE_SILENCE;
        }
        self.sources_mut()
            .iter_mut()
            .map(|source| {
                if let Some(s) = source.upgrade() {
                    s.borrow_mut().source_audio(clock) // TODO: find a new home for HasMute
                } else {
                    MONO_SAMPLE_SILENCE
                }
            })
            .sum::<f32>()
    }
}

// Convenience generic for effects
impl<T: HasOverhead + SinksAudio + TransformsAudio + Debug> SourcesAudio for T {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let input = self.gather_source_audio(clock);

        // It's important for this to happen after the gather_source_audio(),
        // because we don't know whether the sources are depending on being
        // called for each time slice.
        if self.is_muted() {
            return MONO_SAMPLE_SILENCE;
        }
        if self.is_enabled() {
            self.transform_audio(clock, input)
        } else {
            input
        }
    }
}

// TODO: write tests for these two new traits.
//
// TODO: this is probably all wrong. The post_message() part is definitely
// wrong, now that update() can return Commands containing Messages. I'd like to
// see whether all this boilerplate can be handled by someone else, either
// centralized or as a truly effortless generic.
#[deprecated]
pub trait SourcesUpdates {
    fn target_uids(&self) -> &[usize];
    fn target_uids_mut(&mut self) -> &mut Vec<usize>;
    fn target_messages(&self) -> &[SmallMessageGenerator];
    fn target_messages_mut(&mut self) -> &mut Vec<SmallMessageGenerator>;
    fn add_target(&mut self, target_uid: usize, target_message: SmallMessageGenerator) {
        self.target_uids_mut().push(target_uid);
        self.target_messages_mut().push(target_message);
    }
    fn post_message(&mut self, value: f32) -> Vec<BigMessage> {
        let mut v = Vec::new();
        for uid in self.target_uids() {
            for message in self.target_messages() {
                v.push(BigMessage::SmallMessage(*uid, message(value)));
            }
        }
        v
    }
}

// TODO: this should have an associated type Message, but I can't figure out how
// to make it work.
#[deprecated]
pub trait SinksUpdates: Debug {
    // Idea: if someone asks for a message generator, then that's the clue we
    // need to register our UID. All that could be managed in that central
    // place.
    fn message_for(&self, param: &str) -> SmallMessageGenerator;
    fn update(&mut self, clock: &Clock, message: SmallMessage);
}

#[deprecated]
pub(crate) trait EvenNewerIsUpdateable: Terminates + Debug {
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

pub trait MakesIsViewable: Debug {
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
pub trait SinksMidi: Debug {
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
pub trait WatchesClock: Debug + Terminates {
    // type Message; // TODO: figure out how to do this!

    /// WatchesClock::tick() must be called exactly once for every sample, and
    /// implementers can assume that they won't be asked to provide any
    /// information until tick() has been called for the time slice.
    fn tick(&mut self, clock: &Clock) -> Vec<BigMessage>;
    // TODO: should be Box<> so stuff gets handed over more cheaply
}

/// Convenience struct that devices can use to populate fields that mini-traits
/// require.
#[deprecated]
#[derive(Clone, Debug, Default)]
pub struct Overhead {
    is_muted: bool,
    is_disabled: bool,
}

impl Overhead {
    pub(crate) fn is_muted(&self) -> bool {
        self.is_muted
    }
    pub(crate) fn set_muted(&mut self, is_muted: bool) -> bool {
        self.is_muted = is_muted;
        self.is_muted
    }
    pub(crate) fn is_enabled(&self) -> bool {
        !self.is_disabled
    }
    pub(crate) fn set_enabled(&mut self, is_enabled: bool) -> bool {
        self.is_disabled = !is_enabled;
        !self.is_disabled
    }
}

#[deprecated]
pub trait HasOverhead: HasMute + HasEnable {
    fn overhead(&self) -> &Overhead;
    fn overhead_mut(&mut self) -> &mut Overhead;
}
impl<T: HasOverhead> HasMute for T {
    fn is_muted(&self) -> bool {
        self.overhead().is_muted()
    }
    fn set_muted(&mut self, is_muted: bool) -> bool {
        self.overhead_mut().set_muted(is_muted)
    }
}
impl<T: HasOverhead> HasEnable for T {
    fn is_enabled(&self) -> bool {
        self.overhead().is_enabled()
    }
    fn set_enabled(&mut self, is_enabled: bool) -> bool {
        self.overhead_mut().set_enabled(is_enabled);
        self.overhead().is_enabled()
    }
}

/// Some SourcesAudio will need to be called each cycle even if we don't need
/// their audio (effects, for example). I think (not sure) that it's easier for
/// individual devices to track whether they're muted, and to make that
/// information externally available, so that we can still call them (and their
/// children, recursively) but ignore their output, compared to either expecting
/// them to return silence (which would let muted be an internal-only state), or
/// to have something up in the sky track everyone who's muted.
#[deprecated]
pub trait HasMute {
    fn is_muted(&self) -> bool;
    fn set_muted(&mut self, is_muted: bool) -> bool;
    fn toggle_muted(&mut self) -> bool {
        self.set_muted(!self.is_muted())
    }
}

// Whether this device can be switched on/off at runtime. The difference between
// muted and enabled is that a muted device kills the sound, even if non-muted
// devices are inputting sound to it. A disabled device, on the other hand,
// might pass through sound without changing it. For example, a disabled filter
// in the middle of a chain would become a passthrough.
#[deprecated]
pub trait HasEnable {
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, is_enabled: bool) -> bool;
    fn toggle_enabled(&mut self) -> bool {
        self.set_enabled(!self.is_enabled())
    }
}

// WORKING ASSERTION: WatchesClock should not also SourcesAudio, because
// WatchesClock gets a clock tick, whereas SourcesAudio gets a sources_audio(),
// and both are time slice-y. Be on the lookout for anything that claims to need
// both.
#[deprecated]
pub trait IsMidiInstrument: SourcesAudio + SinksMidi + MakesIsViewable {} // TODO + MakesControlSink
#[deprecated]
pub trait IsEffect:
    SourcesAudio + SinksAudio + TransformsAudio + SinksUpdates + HasOverhead + MakesIsViewable
{
}

#[deprecated]
pub trait IsMidiEffect: SourcesMidi + SinksMidi + WatchesClock + MakesIsViewable {}
#[deprecated]
pub trait IsController: SourcesUpdates + WatchesClock {}

#[cfg(test)]
pub mod tests {
    use super::{IsMidiInstrument, SinksAudio, SourcesAudio, WatchesClock};
    use crate::{
        clock::Clock,
        clock::WatchedClock,
        common::{rrc, rrc_clone, rrc_downgrade, Rrc, MONO_SAMPLE_SILENCE},
        control::{AdsrEnvelopeControlParams, BigMessage, GainControlParams},
        effects::{
            arpeggiator::Arpeggiator, bitcrusher::Bitcrusher, filter::BiQuadFilter, gain::Gain,
        },
        envelopes::AdsrEnvelope,
        messages::tests::TestMessage,
        midi::{
            sequencers::{BeatSequencer, MidiTickSequencer},
            MidiChannel, MidiUtils,
        },
        oscillators::Oscillator,
        settings::patches::{EnvelopeSettings, SynthPatch, WaveformType},
        synthesizers::{
            drumkit_sampler::Sampler as DrumkitSampler,
            sampler::Sampler,
            welsh::{PatchName, Synth},
        },
        traits::{SinksMidi, SinksUpdates, SourcesMidi, SourcesUpdates, Terminates},
        utils::{
            tests::{
                OldTestOrchestrator, TestArpeggiator, TestArpeggiatorControlParams,
                TestClockWatcher, TestControlSourceContinuous, TestMidiSink, TestSynth,
            },
            Timer, Trigger,
        },
    };
    use rand::random;

    #[test]
    fn test_orchestration() {
        let mut clock = WatchedClock::new();
        let mut orchestrator = OldTestOrchestrator::new();

        // Create a synth consisting of an oscillator and envelope.
        let envelope = AdsrEnvelope::new_wrapped_with(&EnvelopeSettings::default());
        let mut oscillator = Box::new(Oscillator::new_with(WaveformType::Sine));
        oscillator.set_frequency(MidiUtils::note_to_frequency(60));
        let synth = rrc(TestSynth::new_with(
            oscillator,
            rrc_clone::<AdsrEnvelope>(&envelope),
        ));

        // Create a gain effect, and plug the synth's audio output into it.
        let effect = Gain::new_wrapped();
        effect
            .borrow_mut()
            .add_audio_source(rrc_downgrade::<TestSynth<TestMessage>>(&synth));

        // Then plug the gain effect's audio out into the main mixer.
        orchestrator.add_audio_source(rrc_downgrade::<Gain>(&effect));

        // Let the system know how to find the gain effect again.
        const GAIN_UID: usize = 17;
        orchestrator
            .updateables
            .insert(GAIN_UID, rrc_downgrade::<Gain>(&effect));

        // Create a second Oscillator that provides an audio signal.
        // TestControlSourceContinuous adapts that audio signal to a series of
        // control events, and then posts messages that the gain effect handles
        // to adjust its level.

        // TODO: re-enable block below, soon!

        // let mut audio_to_controller =
        //     TestControlSourceContinuous::<TestMessage>::new_with(Box::new(Oscillator::new()));
        // let message = effect
        //     .borrow()
        //     .message_for(&GainControlParams::Ceiling.to_string());
        // audio_to_controller.add_target(GAIN_UID, message);

        // Trigger posts a message at a given time. We use it to trigger the
        // AdsrEnvelope note-on..
        let mut trigger_on = Trigger::<TestMessage>::new(1.0, 1.0);
        const ENVELOPE_UID: usize = 42;
        orchestrator
            .updateables
            .insert(ENVELOPE_UID, rrc_downgrade::<AdsrEnvelope>(&envelope));
        trigger_on.add_target(
            ENVELOPE_UID,
            envelope
                .borrow()
                .message_for(&AdsrEnvelopeControlParams::Note.to_string()),
        );
        clock.add_watcher(rrc(trigger_on));

        // Same thing, except the value sent is zero, which the AdsrEnvelope
        // interprets as note-off.
        let mut trigger_off = Trigger::<TestMessage>::new(1.5, 0.0);
        trigger_off.add_target(
            ENVELOPE_UID,
            envelope
                .borrow()
                .message_for(&AdsrEnvelopeControlParams::Note.to_string()),
        );
        clock.add_watcher(rrc(trigger_off));

        // Tell the orchestrator when to end its loop.
        let timer = Timer::<TestMessage>::new_with(2.0);
        clock.add_watcher(rrc(timer));

        // Run everything.
        let samples = orchestrator.run_until_completion(&mut clock);
        assert_eq!(samples.len(), 2 * 44100);

        // envelope hasn't been triggered yet
        assert_eq!(samples[0], 0.0);

        // envelope should be triggered at 1-second mark. We check two
        // consecutive samples just in case the oscillator happens to cross over
        // between negative and positive right at that moment.
        assert!(samples[44100] != 0.0 || samples[44100 + 1] != 0.0);
    }

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
