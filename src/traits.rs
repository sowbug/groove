use crate::{
    clock::Clock,
    common::{MonoSample, Ww, MONO_SAMPLE_SILENCE},
    gui::{IsViewable, ViewableMessage},
    midi::{MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL, MIDI_CHANNEL_RECEIVE_NONE},
};
use std::collections::HashMap;
use std::fmt::Debug;

/// Provides audio in the form of digital samples.
pub trait SourcesAudio: Debug + IsMutable {
    // Lots of implementers don't care about clock here,
    // but some do (oscillators, LFOs), and it's a lot cleaner
    // to pass a bit of extra information here than to either
    // create a separate optional method supplying it (which
    // everyone would have to call anyway), or define a whole
    // new trait that breaks a bunch of simple paths elsewhere.
    fn source_audio(&mut self, clock: &Clock) -> MonoSample;
}
pub trait IsMutable {
    fn is_muted(&self) -> bool;
    fn set_muted(&mut self, is_muted: bool);
}

/// Can do something with audio samples. When it needs to do its
/// work, it asks its SourcesAudio for their samples.
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
                    if s.borrow().is_muted() {
                        MONO_SAMPLE_SILENCE
                    } else {
                        s.borrow_mut().source_audio(clock)
                    }
                } else {
                    MONO_SAMPLE_SILENCE
                }
            })
            .sum::<f32>()
    }
}

/// TransformsAudio can be thought of as SourcesAudio + SinksAudio, but it's
/// an important third traits because it exposes the business logic that
/// happens between the sinking and sourcing, which is useful for testing.
pub trait TransformsAudio {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample;
}

// Convenience generic for effects
impl<T: SinksAudio + TransformsAudio + IsMutable + Debug> SourcesAudio for T {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let input = self.gather_source_audio(clock);
        self.transform_audio(input)
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
        // TODO: is there a good reason for channel != sink.midi_channel()? If not, why is it a param?
        self.midi_sinks_mut().entry(channel).or_default().push(sink);
    }
    fn issue_midi(&self, clock: &Clock, message: &MidiMessage) {
        if self.midi_sinks().contains_key(&MIDI_CHANNEL_RECEIVE_ALL) {
            for sink in self.midi_sinks().get(&MIDI_CHANNEL_RECEIVE_ALL).unwrap() {
                if let Some(sink_up) = sink.upgrade() {
                    sink_up.borrow_mut().handle_midi(clock, message);
                }
            }
        }
        if self.midi_sinks().contains_key(&message.channel) {
            for sink in self.midi_sinks().get(&message.channel).unwrap() {
                if let Some(sink_up) = sink.upgrade() {
                    sink_up.borrow_mut().handle_midi(clock, message);
                }
            }
        }
    }
}
pub trait SinksMidi: Debug {
    fn midi_channel(&self) -> MidiChannel;
    fn set_midi_channel(&mut self, midi_channel: MidiChannel);

    fn handle_midi(&mut self, clock: &Clock, message: &MidiMessage) {
        if self.midi_channel() == MIDI_CHANNEL_RECEIVE_NONE {
            return;
        }
        if self.midi_channel() == MIDI_CHANNEL_RECEIVE_ALL || self.midi_channel() == message.channel
        {
            // TODO: SourcesMidi is already going through trouble to respect channels. Is this redundant?
            self.handle_midi_for_channel(clock, message);
        }
    }
    fn handle_midi_for_channel(&mut self, clock: &Clock, message: &MidiMessage);
}

/// A WatchesClock is something that needs to be called for every time
/// slice. This sounds like SourcesAudio; indeed SourcesAudio do not
/// (and *cannot*) implement WatchesClock because they're already called
/// on every time slice to provide an audio sample. A WatchesClock has no
/// extrinsic reason to be called, so the trait exists to make sure that
/// whatever intrinsic reason for being called is satisfied.
pub trait WatchesClock: Debug + Terminates {
    /// WatchesClock::tick() must be called exactly once for every sample, and
    /// implementers can assume that they won't be asked to provide any
    /// information until tick() has been called for the time slice.
    fn tick(&mut self, clock: &Clock);
}

// Something that Terminates has a point in time where it would be OK never
// being called or continuing to exist.
//
// If you're required to implement Terminates, but you don't know when
// you need to terminate, then you should always return true. For example,
// an arpeggiator is a WatchesClock, which means it is also a Terminates,
// but it would be happy to keep responding to MIDI input forever. It should
// return true.
//
// The reason to choose true rather than false is that the caller uses is_finished()
// to determine whether a song is complete. If a Terminates never returns true,
// the loop will never end. Thus, "is_finished" is more like "is unaware of any
// reason to continue existing" rather than "is certain there is no more work to do."
pub trait Terminates {
    fn is_finished(&self) -> bool;
}

// WORKING ASSERTION: WatchesClock should not also SourcesAudio, because
// WatchesClock gets a clock tick, whereas SourcesAudio gets a sources_audio(), and
// both are time slice-y. Be on the lookout for anything that claims to need both.
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
pub mod tests {
    use super::{SinksAudio, SourcesAudio, SourcesControl};
    use crate::{
        clock::Clock,
        clock::WatchedClock,
        common::{MonoSample, MONO_SAMPLE_SILENCE},
        effects::gain::Gain,
        envelopes::AdsrEnvelope,
        midi::MidiMessage,
        oscillators::Oscillator,
        settings::patches::{EnvelopeSettings, WaveformType},
        traits::{MakesControlSink, SinksMidi, SourcesMidi, Terminates, WatchesClock},
        utils::tests::{
            TestArpeggiator, TestAudioSink, TestAudioSource, TestClockWatcher, TestControlSource,
            TestControlSourceContinuous, TestControllable, TestKeyboard, TestMidiSink,
            TestMidiSource, TestOrchestrator, TestSynth, TestTimer, TestTrigger,
        },
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_orchestration() {
        let mut clock = WatchedClock::new();
        let mut orchestrator = TestOrchestrator::new();
        let envelope = AdsrEnvelope::new_wrapped_with(&EnvelopeSettings::default());
        let oscillator = Oscillator::new_wrapped_with(WaveformType::Sine);
        oscillator
            .borrow_mut()
            .set_frequency(MidiMessage::note_to_frequency(60));
        let envelope_synth_clone = Rc::clone(&envelope);
        let synth = Rc::new(RefCell::new(TestSynth::new_with(
            oscillator,
            envelope_synth_clone,
        )));
        let effect = Gain::new_wrapped();
        let source = Rc::downgrade(&synth);
        effect.borrow_mut().add_audio_source(source);
        let source = Rc::downgrade(&effect);
        orchestrator.add_audio_source(source);

        // An Oscillator provides an audio signal. TestControlSourceContinuous adapts that audio
        // signal to a series of control events. GainLevelController adapts the control events
        // to Gain level changes.
        let mut audio_to_controller =
            TestControlSourceContinuous::new_with(Box::new(Oscillator::new()));
        if let Some(effect_controller) = effect
            .borrow()
            .make_control_sink(Oscillator::CONTROL_PARAM_FREQUENCY)
        {
            audio_to_controller.add_control_sink(effect_controller);
        };

        let timer = TestTimer::new_with(2.0);
        clock.add_watcher(Rc::new(RefCell::new(timer)));

        // TestTrigger provides an event at a certain time. EnvelopeNoteController adapts the event
        // to internal ADSR events.
        let mut trigger_on = TestTrigger::new(1.0, 1.0);
        if let Some(envelope_controller) = envelope
            .borrow()
            .make_control_sink(AdsrEnvelope::CONTROL_PARAM_NOTE)
        {
            trigger_on.add_control_sink(envelope_controller);
        };
        clock.add_watcher(Rc::new(RefCell::new(trigger_on)));

        let mut trigger_off = TestTrigger::new(1.5, 0.0);
        if let Some(envelope_controller) = envelope
            .borrow()
            .make_control_sink(AdsrEnvelope::CONTROL_PARAM_NOTE)
        {
            trigger_off.add_control_sink(envelope_controller);
        };
        clock.add_watcher(Rc::new(RefCell::new(trigger_off)));

        let mut samples = Vec::<MonoSample>::new();
        orchestrator.start(&mut clock, &mut samples);
        assert_eq!(samples.len(), 2 * 44100);

        // envelope hasn't been triggered yet
        assert_eq!(samples[0], 0.0);

        // envelope should be triggered at 1-second mark. We check two consecutive samples just in
        // case the oscillator happens to cross over between negative and positive right at that moment.
        assert!(samples[44100] != 0.0 || samples[44100 + 1] != 0.0);
    }

    #[test]
    fn test_clock_watcher() {
        let mut clock = Clock::new_test();
        let mut clock_watcher = TestClockWatcher::new(1.0);

        loop {
            clock.tick();
            clock_watcher.tick(&mut clock);
            if clock_watcher.is_finished() {
                break;
            }
        }
        assert!(clock.seconds() >= 1.0);
    }

    #[test]
    fn test_audio_source() {
        let mut s = TestAudioSource::new();
        assert_eq!(s.source_audio(&Clock::new()), MONO_SAMPLE_SILENCE);
    }

    #[test]
    fn test_audio_sink() {
        let mut sink = TestAudioSink::new();
        let source = Rc::new(RefCell::new(TestAudioSource::new()));
        assert!(sink.sources().is_empty());
        let source = Rc::downgrade(&source);
        sink.add_audio_source(source);
        assert_eq!(sink.sources().len(), 1);
    }

    #[test]
    fn test_automation_source_and_sink() {
        // By itself, TestAutomationSource doesn't do much, so we test both Source/Sink together.
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
        let sink_down = Rc::downgrade(&sink);
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
        let sink = Rc::downgrade(&instrument);
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
}
