use crate::{
    clock::Clock,
    common::{MonoSample, Ww, MONO_SAMPLE_SILENCE},
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

/// Controls SinksControl through SinksControlParam.
pub trait SourcesControl {
    fn control_sinks(&self) -> &[Box<dyn SinksControl>];
    fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>>;

    fn add_control_sink(&mut self, sink: Box<dyn SinksControl>) {
        self.control_sinks_mut().push(sink);
    }
    fn issue_control(&mut self, clock: &Clock, value: f32) {
        for sink in self.control_sinks_mut() {
            sink.handle_control(clock, value);
        }
    }
}

pub trait SinksControl: Debug {
    fn handle_control(&mut self, clock: &Clock, value: f32);
}

pub trait MakesControlSink: Debug {
    fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>>;
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
    /// WatchesClock::tick() must be called exactly once for every sample, and
    /// implementers can assume that they won't be asked to provide any
    /// information until tick() has been called for the time slice.
    fn tick(&mut self, clock: &Clock);
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

// WORKING ASSERTION: WatchesClock should not also SourcesAudio, because
// WatchesClock gets a clock tick, whereas SourcesAudio gets a sources_audio(),
// and both are time slice-y. Be on the lookout for anything that claims to need
// both.
pub trait IsMidiInstrument: SourcesAudio + SinksMidi + MakesIsViewable {} // TODO + MakesControlSink
pub trait IsEffect:
    SourcesAudio + SinksAudio + TransformsAudio + MakesControlSink + MakesIsViewable
{
}
pub trait IsMidiEffect:
    SourcesMidi + SinksMidi + WatchesClock + MakesControlSink + MakesIsViewable
{
}
pub trait IsController: SourcesControl + WatchesClock {}

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
pub mod tests {
    use super::{IsMidiInstrument, SinksAudio, SourcesAudio, SourcesControl};
    use crate::{
        clock::Clock,
        clock::WatchedClock,
        common::{rrc, rrc_clone, rrc_downgrade, MonoSample, Rrc, MONO_SAMPLE_SILENCE},
        control::ControlTrip,
        effects::{arpeggiator::Arpeggiator, bitcrusher::Bitcrusher, filter::Filter, gain::Gain},
        envelopes::AdsrEnvelope,
        midi::{sequencer::MidiSequencer, MidiChannel, MidiUtils},
        oscillators::Oscillator,
        patterns::PatternSequencer,
        settings::patches::{EnvelopeSettings, SynthPatch, WaveformType},
        synthesizers::{
            drumkit_sampler::Sampler as DrumkitSampler,
            sampler::Sampler,
            welsh::{PatchName, Synth},
        },
        traits::{MakesControlSink, SinksMidi, SourcesMidi, Terminates, WatchesClock},
        utils::tests::{
            TestArpeggiator, TestAudioSink, TestAudioSource, TestClockWatcher, TestControlSource,
            TestControlSourceContinuous, TestControllable, TestKeyboard, TestMidiSink,
            TestMidiSource, TestOrchestrator, TestSynth, TestTimer, TestTrigger,
        },
        TimeSignature,
    };
    use rand::random;

    #[test]
    fn test_orchestration() {
        let mut clock = WatchedClock::new();
        let mut orchestrator = TestOrchestrator::new();
        let envelope = AdsrEnvelope::new_wrapped_with(&EnvelopeSettings::default());
        let oscillator = Oscillator::new_wrapped_with(WaveformType::Sine);
        oscillator
            .borrow_mut()
            .set_frequency(MidiUtils::note_to_frequency(60));
        let envelope_synth_clone = rrc_clone(&envelope);
        let synth = rrc(TestSynth::new_with(oscillator, envelope_synth_clone));
        let effect = Gain::new_wrapped();
        let source = rrc_downgrade(&synth);
        effect.borrow_mut().add_audio_source(source);
        let source = rrc_downgrade(&effect);
        orchestrator.add_audio_source(source);

        // An Oscillator provides an audio signal. TestControlSourceContinuous
        // adapts that audio signal to a series of control events.
        // GainLevelController adapts the control events to Gain level changes.
        let mut audio_to_controller =
            TestControlSourceContinuous::new_with(Box::new(Oscillator::new()));
        if let Some(effect_controller) = effect
            .borrow()
            .make_control_sink(Oscillator::CONTROL_PARAM_FREQUENCY)
        {
            audio_to_controller.add_control_sink(effect_controller);
        };

        let timer = TestTimer::new_with(2.0);
        clock.add_watcher(rrc(timer));

        // TestTrigger provides an event at a certain time.
        // EnvelopeNoteController adapts the event to internal ADSR events.
        let mut trigger_on = TestTrigger::new(1.0, 1.0);
        if let Some(envelope_controller) = envelope
            .borrow()
            .make_control_sink(AdsrEnvelope::CONTROL_PARAM_NOTE)
        {
            trigger_on.add_control_sink(envelope_controller);
        };
        clock.add_watcher(rrc(trigger_on));

        let mut trigger_off = TestTrigger::new(1.5, 0.0);
        if let Some(envelope_controller) = envelope
            .borrow()
            .make_control_sink(AdsrEnvelope::CONTROL_PARAM_NOTE)
        {
            trigger_off.add_control_sink(envelope_controller);
        };
        clock.add_watcher(rrc(trigger_off));

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

    sources_audio_tests! {
        adsr_envelope: AdsrEnvelope,
        bitcrusher: Bitcrusher,
        drumkit_sampler: DrumkitSampler,
        filter: Filter,
        gain: Gain,
        limiter: crate::effects::limiter::Limiter,
        mixer: crate::effects::mixer::Mixer,
        oscillator: Oscillator,
        sampler: Sampler,
        stepped_envelope: crate::envelopes::SteppedEnvelope,
        test_audio_source: TestAudioSource,
        welsh_synth: Synth,
    }

    sinks_audio_tests! {
        sinks_audio_bitcrusher: Bitcrusher,
        sinks_audio_filter: Filter,
        sinks_audio_gain: Gain,
        sinks_audio_limiter: crate::effects::limiter::Limiter,
        sinks_audio_mixer: crate::effects::mixer::Mixer,
    }

    #[test]
    fn test_audio_sink() {
        let mut sink = TestAudioSink::new();
        let source = rrc(TestAudioSource::new());
        assert!(sink.sources().is_empty());
        let source = rrc_downgrade(&source);
        sink.add_audio_source(source);
        assert_eq!(sink.sources().len(), 1);
    }

    #[test]
    fn test_automation_source_and_sink() {
        // By itself, TestAutomationSource doesn't do much, so we test both
        // Source/Sink together.
        let mut source = TestControlSource::new();
        let sink = TestControllable::new_wrapped();

        // Can we add a sink to the source?
        assert!(source.control_sinks().is_empty());
        if let Some(controllable_controller) = sink
            .borrow()
            .make_control_sink(TestControllable::CONTROL_PARAM_DEFAULT)
        {
            source.add_control_sink(controllable_controller);
        };
        assert!(!source.control_sinks().is_empty());

        // Does the source propagate to its sinks?
        assert_eq!(sink.borrow().value, 0.0);
        source.handle_test_event(42.0);
        assert_eq!(sink.borrow().value, 42.0);
    }

    #[test]
    fn test_midi_source_and_sink() {
        let mut source = TestMidiSource::new();
        let sink = TestMidiSink::new_wrapped();

        assert!(source.midi_sinks().is_empty());
        let sink_down = rrc_downgrade(&sink);
        source.add_midi_sink(sink.borrow().midi_channel(), sink_down);
        assert!(!source.midi_sinks().is_empty());

        let clock = Clock::new_test();
        assert!(!sink.borrow().is_playing);
        source.source_some_midi(&clock);
        assert!(sink.borrow().is_playing);
    }

    #[test]
    fn test_keyboard_to_automation_to_midi() {
        let mut keyboard_interface = TestKeyboard::new();
        let arpeggiator = TestArpeggiator::new_wrapped_with(TestMidiSink::TEST_MIDI_CHANNEL);
        let instrument = TestMidiSink::new_wrapped();

        if let Some(arpeggiator_controller) = arpeggiator
            .borrow()
            .make_control_sink(TestArpeggiator::CONTROL_PARAM_TEMPO)
        {
            keyboard_interface.add_control_sink(arpeggiator_controller);
        };
        let sink = rrc_downgrade(&instrument);
        arpeggiator
            .borrow_mut()
            .add_midi_sink(instrument.borrow().midi_channel(), sink);

        assert_eq!(arpeggiator.borrow().tempo, 0.0);
        keyboard_interface.handle_keypress(1); // set tempo to 50%
        assert_eq!(arpeggiator.borrow().tempo, 0.5);

        let clock = Clock::new_test();

        assert!(!instrument.borrow().is_playing);
        arpeggiator.borrow_mut().tick(&clock);
        assert!(instrument.borrow().is_playing);
    }

    /// Add concrete instances of WatchesClock here for anyone to use for
    /// testing.
    fn watches_clock_instances_for_testing() -> Vec<Rrc<dyn WatchesClock>> {
        let target = Bitcrusher::new_wrapped_with(4)
            .borrow_mut()
            .make_control_sink(Bitcrusher::CONTROL_PARAM_BITS_TO_CRUSH)
            .unwrap();
        vec![
            Arpeggiator::new_wrapped_with(0, 0),
            rrc(ControlTrip::new(target)),
            PatternSequencer::new_wrapped_with(&TimeSignature::new_defaults()),
            rrc(MidiSequencer::new()),
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
            Filter::new_wrapped_with(&crate::effects::filter::FilterType::BandPass {
                sample_rate: 13245,
                cutoff: 2343.9,
                bandwidth: 4354.3,
            }),
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
                let source = rrc_downgrade(&source);
                orchestrator.add_audio_source(source);
            }
        }

        for _ in 0..100 {
            let mut clock = Clock::new();
            clock.debug_set_samples(random());
            let _ = orchestrator.main_mixer.source_audio(&clock);
        }
    }
}
