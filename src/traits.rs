use crate::{
    clock::Clock,
    common::{MonoSample, Ww, MONO_SAMPLE_SILENCE},
    control::{BigMessage, SmallMessage, SmallMessageGenerator},
    gui::{IsViewable, ViewableMessage},
    midi::{MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL, MIDI_CHANNEL_RECEIVE_NONE},
};
use std::collections::HashMap;
use std::fmt::Debug;

/// Provides audio in the form of digital samples.
pub trait SourcesAudio: HasMute + Debug {
    // Lots of implementers don't care about clock here, but some do
    // (oscillators, LFOs), and it's a lot cleaner to pass a bit of extra
    // information here than to either create a separate optional method
    // supplying it (which everyone would have to call anyway), or define a
    // whole new trait that breaks a bunch of simple paths elsewhere.
    //
    // TODO: I dream of removing the mut in &mut self. But some devices
    // legitimately change state when asked for their current state, and I
    // couldn't think of a way to give a chance to mutate that doesn't just make
    // everything more complicated.
    fn source_audio(&mut self, clock: &Clock) -> MonoSample;
}

/// Can do something with audio samples. When it needs to do its work, it asks
/// its SourcesAudio for their samples.
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
                    let sample = s.borrow_mut().source_audio(clock);
                    if s.borrow().is_muted() {
                        MONO_SAMPLE_SILENCE
                    } else {
                        sample
                    }
                } else {
                    MONO_SAMPLE_SILENCE
                }
            })
            .sum::<f32>()
    }
}

/// TransformsAudio can be thought of as SourcesAudio + SinksAudio, but it's an
/// important third traits because it exposes the business logic that happens
/// between the sinking and sourcing, which is useful for testing.
pub trait TransformsAudio {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample;
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
            self.transform_audio(input)
        } else {
            input
        }
    }
}

// TODO: write tests for these two new traits.
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
pub trait SinksUpdates: Debug {
    // Idea: if someone asks for a message generator, then that's the clue we
    // need to register our UID. All that could be managed in that central
    // place.
    fn message_for(&self, param: &str) -> SmallMessageGenerator;
    fn update(&mut self, clock: &Clock, message: SmallMessage);
}

pub trait MakesIsViewable: Debug {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>>;
}

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
pub trait WatchesClock: Debug + Terminates {
    // type Message; // TODO: figure out how to do this!

    /// WatchesClock::tick() must be called exactly once for every sample, and
    /// implementers can assume that they won't be asked to provide any
    /// information until tick() has been called for the time slice.
    fn tick(&mut self, clock: &Clock) -> Vec<BigMessage>;
    // TODO: should be Box<> so stuff gets handed over more cheaply
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
pub trait Terminates {
    fn is_finished(&self) -> bool;
}

/// Convenience struct that devices can use to populate fields that mini-traits
/// require.
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
pub trait IsMidiInstrument: SourcesAudio + SinksMidi + MakesIsViewable {} // TODO + MakesControlSink
pub trait IsEffect:
    SourcesAudio + SinksAudio + TransformsAudio + SinksUpdates + HasOverhead + MakesIsViewable
{
}

pub trait IsMidiEffect: SourcesMidi + SinksMidi + WatchesClock + MakesIsViewable {}
pub trait IsController: SourcesUpdates + WatchesClock {}

#[cfg(test)]
macro_rules! sources_audio_tests {
    ($($name:ident: $type:ty,)*) => {
    $(
        mod $name {
            use super::*;

            #[test]
            fn new_audio_source_is_silent() {
                let mut s = <$type>::default();
                assert_eq!(s.source_audio(&Clock::new()), MONO_SAMPLE_SILENCE);
            }
        }
    )*
    }
}

#[cfg(test)]
macro_rules! sinks_audio_tests {
    ($($name:ident: $type:ty,)*) => {
    $(
        mod $name {
            use super::*;

            #[test]
            fn new_audio_sink_can_be_instantiated() {
                let s = <$type>::default();
                assert_eq!(s.sources().len(), 0);
            }
        }
    )*
    }
}

#[cfg(test)]
macro_rules! sources_midi_tests {
    ($($name:ident: $type:ty,)*) => {
    $(
        mod $name {
            use super::*;

            #[test]
            fn new_midi_source_can_be_instantiated() {
                let s = <$type>::default();
                assert_eq!(s.midi_sinks().len(), 0);
            }
        }
    )*
    }
}

#[cfg(test)]
macro_rules! sinks_midi_tests {
    ($($name:ident: $type:ty,)*) => {
    $(
        mod $name {
            use super::*;

            #[test]
            fn new_midi_sink_can_be_instantiated() {
                let s = <$type>::default();
                assert_eq!(s.midi_channel(), 0);
            }
        }
    )*
    }
}

#[cfg(test)]
macro_rules! has_overhead_tests {
    ($($name:ident: $type:ty,)*) => {
    $(
        mod $name {
            use crate::traits::HasEnable;
            use crate::traits::HasMute;

            #[test]
            fn has_overhead_mute_enable_work() {
                let mut s = <$type>::default();
                assert_eq!(s.is_enabled(), true);
                assert_eq!(s.is_muted(), false);

                s.set_enabled(false);
                assert_eq!(s.is_enabled(), false);

                s.set_muted(true);
                assert_eq!(s.is_muted(), true);
            }
        }
    )*
    }
}

#[cfg(test)]
pub mod tests {
    use super::{IsMidiInstrument, SinksAudio, SourcesAudio, WatchesClock};
    use crate::{
        clock::Clock,
        clock::WatchedClock,
        common::{rrc, rrc_clone, rrc_downgrade, MonoSample, Rrc, MONO_SAMPLE_SILENCE},
        control::{AdsrEnvelopeControlParams, GainControlParams},
        effects::{
            arpeggiator::Arpeggiator, bitcrusher::Bitcrusher, filter::BiQuadFilter, gain::Gain,
        },
        envelopes::AdsrEnvelope,
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
        utils::tests::{
            TestArpeggiator, TestArpeggiatorControlParams, TestAudioSink, TestAudioSource,
            TestClockWatcher, TestControlSourceContinuous, TestKeyboard, TestMidiSink,
            TestMidiSource, TestOrchestrator, TestSynth, TestTimer, TestTrigger,
        },
    };
    use rand::random;

    #[test]
    fn test_orchestration() {
        let mut clock = WatchedClock::new();
        let mut orchestrator = TestOrchestrator::new();

        // Create a synth consisting of an oscillator and envelope.
        let envelope = AdsrEnvelope::new_wrapped_with(&EnvelopeSettings::default());
        let oscillator = Oscillator::new_wrapped_with(WaveformType::Sine);
        oscillator
            .borrow_mut()
            .set_frequency(MidiUtils::note_to_frequency(60));
        let synth = rrc(TestSynth::new_with(
            oscillator,
            rrc_clone::<AdsrEnvelope>(&envelope),
        ));

        // Create a gain effect, and plug the synth's audio output into it.
        let effect = Gain::new_wrapped();
        effect
            .borrow_mut()
            .add_audio_source(rrc_downgrade::<TestSynth>(&synth));

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
        let mut audio_to_controller =
            TestControlSourceContinuous::new_with(Box::new(Oscillator::new()));
        let message = effect
            .borrow()
            .message_for(&GainControlParams::Ceiling.to_string());
        audio_to_controller.add_target(GAIN_UID, message);

        // TestTrigger posts a message at a given time. We use it to trigger the
        // AdsrEnvelope note-on..
        let mut trigger_on = TestTrigger::new(1.0, 1.0);
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
        let mut trigger_off = TestTrigger::new(1.5, 0.0);
        trigger_off.add_target(
            ENVELOPE_UID,
            envelope
                .borrow()
                .message_for(&AdsrEnvelopeControlParams::Note.to_string()),
        );
        clock.add_watcher(rrc(trigger_off));

        // Tell the orchestrator when to end its loop.
        let timer = TestTimer::new_with(2.0);
        clock.add_watcher(rrc(timer));

        // Run everything.
        let mut samples = Vec::<MonoSample>::new();
        orchestrator.start(&mut clock, &mut samples);
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
        let mut clock_watcher = TestClockWatcher::new(1.0);

        loop {
            clock.tick();
            clock_watcher.tick(&clock);
            if clock_watcher.is_finished() {
                break;
            }
        }
        assert!(clock.seconds() >= 1.0);
    }

    // $ grep -R "HasOverhead for " src/ | grep -o "for.*$" | \
    //   grep -o -E "[A-Z][[:alpha:]]+" | sort -u`
    sources_audio_tests! {
        sources_audio_adsr_envelope: AdsrEnvelope,
        sources_audio_bitcrusher: Bitcrusher,
        sources_audio_drumkit_sampler: DrumkitSampler,
        sources_audio_filter: BiQuadFilter,
        sources_audio_gain: Gain,
        sources_audio_limiter: crate::effects::limiter::Limiter,
        sources_audio_mixer: crate::effects::mixer::Mixer,
        sources_audio_oscillator: Oscillator,
        sources_audio_sampler: Sampler,
        sources_audio_stepped_envelope: crate::envelopes::SteppedEnvelope,
        sources_audio_test_audio_source: TestAudioSource,
        sources_audio_synth: Synth,
    }

    sinks_audio_tests! {
        sinks_audio_bitcrusher: Bitcrusher,
        sinks_audio_filter: BiQuadFilter,
        sinks_audio_gain: Gain,
        sinks_audio_limiter: crate::effects::limiter::Limiter,
        sinks_audio_mixer: crate::effects::mixer::Mixer,
    }

    sources_midi_tests! {
        sources_midi_arpeggiator: Arpeggiator,
    }

    sinks_midi_tests! {
        sinks_midi_synth: Synth,
    }

    has_overhead_tests! {
        has_overhead_adsr_envelope: crate::envelopes::AdsrEnvelope,
        has_overhead_arpeggiator: crate::effects::arpeggiator::Arpeggiator,
        has_overhead_beat_sequencer: crate::midi::sequencers::BeatSequencer,
        has_overhead_bitcrusher: crate::effects::bitcrusher::Bitcrusher,
        has_overhead_drumkit_sampler: crate::synthesizers::drumkit_sampler::Sampler,
        has_overhead_filter: crate::effects::filter::BiQuadFilter,
        has_overhead_gain: crate::effects::gain::Gain,
        has_overhead_limiter: crate::effects::limiter::Limiter,
        has_overhead_midi_tick_sequencer: crate::midi::sequencers::MidiTickSequencer,
        has_overhead_mixer: crate::effects::mixer::Mixer,
        has_overhead_oscillator: crate::oscillators::Oscillator,
        has_overhead_sampler: crate::synthesizers::sampler::Sampler,
        has_overhead_stepped_envelope: crate::envelopes::SteppedEnvelope,
        has_overhead_synth: crate::synthesizers::welsh::Synth,
        has_overhead_test_audio_sink: crate::utils::tests::TestAudioSink,
        has_overhead_test_audio_source: crate::utils::tests::TestAudioSource,
        has_overhead_test_audio_source_always_loud: crate::utils::tests::TestAudioSourceAlwaysLoud,
        has_overhead_test_audio_source_always_same_level: crate::utils::tests::TestAudioSourceAlwaysSameLevel,
        has_overhead_test_audio_source_always_silent: crate::utils::tests::TestAudioSourceAlwaysSilent,
        has_overhead_test_audio_source_always_too_loud: crate::utils::tests::TestAudioSourceAlwaysTooLoud,
        has_overhead_test_audio_source_always_very_quiet: crate::utils::tests::TestAudioSourceAlwaysVeryQuiet,
        has_overhead_test_midi_sink: crate::utils::tests::TestMidiSink,
        has_overhead_test_synth: crate::utils::tests::TestSynth,
        has_overhead_voice: crate::synthesizers::welsh::Voice,
    }

    #[test]
    fn test_audio_sink() {
        let mut sink = TestAudioSink::new();
        let source = rrc(TestAudioSource::new());
        assert!(sink.sources().is_empty());
        sink.add_audio_source(rrc_downgrade::<TestAudioSource>(&source));
        assert_eq!(sink.sources().len(), 1);
    }

    #[test]
    fn test_midi_source_and_sink() {
        let mut source = TestMidiSource::new();
        let sink = TestMidiSink::new_wrapped();

        assert!(source.midi_sinks().is_empty());
        source.add_midi_sink(
            sink.borrow().midi_channel(),
            rrc_downgrade::<TestMidiSink>(&sink),
        );
        assert!(!source.midi_sinks().is_empty());

        let clock = Clock::new_test();
        assert!(!sink.borrow().is_playing);
        source.source_some_midi(&clock);
        assert!(sink.borrow().is_playing);
    }

    #[test]
    fn test_keyboard_to_automation_to_midi() {
        // A fake external device that produces messages.
        let mut keyboard_interface = TestKeyboard::new();

        // Should respond to messages by producing MIDI messages.
        let arpeggiator = TestArpeggiator::new_wrapped_with(TestMidiSink::TEST_MIDI_CHANNEL);
        let instrument = TestMidiSink::new_wrapped();

        let message = arpeggiator
            .borrow()
            .message_for(&TestArpeggiatorControlParams::Tempo.to_string());
        keyboard_interface.add_target(0, message);

        arpeggiator.borrow_mut().add_midi_sink(
            instrument.borrow().midi_channel(),
            rrc_downgrade::<TestMidiSink>(&instrument),
        );

        assert_eq!(arpeggiator.borrow().tempo, 0.0);

        let clock = Clock::new_test();

        let messages = keyboard_interface.handle_keypress(1); // set tempo to 50%
        messages.iter().for_each(|m| match m {
            crate::control::BigMessage::SmallMessage(uid, msg) => {
                assert_eq!(uid, &0);
                arpeggiator.borrow_mut().update(&clock, msg.clone());
            }
        });
        assert_eq!(arpeggiator.borrow().tempo, 0.5);

        assert!(!instrument.borrow().is_playing);
        arpeggiator.borrow_mut().tick(&clock);
        assert!(instrument.borrow().is_playing);
    }

    /// Add concrete instances of WatchesClock here for anyone to use for
    /// testing.
    fn watches_clock_instances_for_testing() -> Vec<Rrc<dyn WatchesClock>> {
        // TODO: figure out how to work this back into testing let target =
        // Bitcrusher::new_wrapped_with(4) .borrow_mut()
        //     .make_control_sink(&BitcrusherControlParams::BitsToCrush.to_string())
        //     .unwrap(); rrc(ControlTrip::new(
        //     crate::control::ControlTripTargetType::New(target), )),

        vec![
            Arpeggiator::new_wrapped_with(0, 0),
            BeatSequencer::new_wrapped(),
            rrc(MidiTickSequencer::new()),
        ]
    }

    #[test]
    fn test_clock_watcher_random_access() {
        let mut clock = WatchedClock::new();

        let mut watchers = watches_clock_instances_for_testing();
        while !watchers.is_empty() {
            clock.add_watcher(watchers.pop().unwrap());
        }

        // Regular start to finish, twice.
        for _ in 0..2 {
            for _ in 0..100 {
                clock.tick();
            }
            clock.reset();
        }

        // Backwards.
        clock.reset();
        for t in 0..100 {
            clock.inner_clock_mut().debug_set_samples(t);
            clock.tick();
        }

        // Random.
        for _ in 0..100 {
            clock.inner_clock_mut().debug_set_samples(random());
            clock.tick();
        }
    }

    /// Add concrete instances of SourcesAudio here for anyone to use for
    /// testing.
    fn sources_audio_instances_for_testing() -> Vec<Rrc<dyn SourcesAudio>> {
        const MIDI_CHANNEL: MidiChannel = 0;

        // If the instance is meaningfully testable after new(), put it here.
        let mut sources: Vec<Rrc<dyn SourcesAudio>> = vec![
            BiQuadFilter::new_wrapped_with(
                &crate::effects::filter::FilterParams::BandPass {
                    cutoff: 2343.9,
                    bandwidth: 4354.3,
                },
                13245,
            ),
            Gain::new_wrapped_with(0.5),
        ];

        // If the instance needs to be told to play a note, put it here.
        let midi_instruments: Vec<Rrc<dyn IsMidiInstrument>> = vec![
            DrumkitSampler::new_wrapped_from_files(MIDI_CHANNEL),
            Sampler::new_wrapped_with(MIDI_CHANNEL, 10000),
            Synth::new_wrapped_with(MIDI_CHANNEL, 44007, SynthPatch::by_name(&PatchName::Piano)),
        ];
        for instrument in midi_instruments {
            instrument.borrow_mut().handle_midi_for_channel(
                &Clock::new(),
                &0,
                &MidiUtils::note_on_c4(),
            );
            sources.push(instrument);
        }

        sources
    }

    #[test]
    fn test_sources_audio_random_access() {
        let mut orchestrator = TestOrchestrator::new();

        let mut sources = sources_audio_instances_for_testing();

        while !sources.is_empty() {
            let source = sources.pop();
            if let Some(source) = source {
                orchestrator.add_audio_source(rrc_downgrade(&source));
            }
        }

        for _ in 0..100 {
            let mut clock = Clock::new();
            clock.debug_set_samples(random());
            let _ = orchestrator.main_mixer.source_audio(&clock);
        }
    }
}
