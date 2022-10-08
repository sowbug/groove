use std::fmt::Debug;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{
    clock::Clock,
    common::{MonoSample, MONO_SAMPLE_SILENCE},
    midi::{MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL, MIDI_CHANNEL_RECEIVE_NONE},
};

/// Provides audio in the form of digital samples.
pub trait SourcesAudio: Debug {
    // Lots of implementers don't care about clock here,
    // but some do (oscillators, LFOs), and it's a lot cleaner
    // to pass a bit of extra information here than to either
    // create a separate optional method supplying it (which
    // everyone would have to call anyway), or define a whole
    // new trait that breaks a bunch of simple paths elsewhere.
    fn source_audio(&mut self, clock: &Clock) -> MonoSample;
}

/// Can do something with audio samples. When it needs to do its
/// work, it asks its SourcesAudio for their samples.
pub trait SinksAudio {
    fn sources(&self) -> &[Rc<RefCell<dyn SourcesAudio>>];
    fn sources_mut(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>>;

    fn add_audio_source(&mut self, source: Rc<RefCell<dyn SourcesAudio>>) {
        self.sources_mut().push(source);
    }

    fn gather_source_audio(&mut self, clock: &Clock) -> MonoSample {
        if self.sources_mut().is_empty() {
            return MONO_SAMPLE_SILENCE;
        }
        self.sources_mut()
            .iter_mut()
            .map(|source| source.borrow_mut().source_audio(clock))
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
impl<T: SinksAudio + TransformsAudio + Debug> SourcesAudio for T {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let input = self.gather_source_audio(clock);
        self.transform_audio(input)
    }
}

/// Controls SinksControl through SinksControlParam.
pub trait SourcesControl {
    fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>];
    fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>>;

    fn add_control_sink(&mut self, sink: Weak<RefCell<dyn SinksControl>>) {
        self.control_sinks_mut().push(sink);
    }
    fn issue_control(&mut self, clock: &Clock, param: &SinksControlParam) {
        for sink in self.control_sinks_mut() {
            if let Some(sink_up) = sink.upgrade() {
                sink_up.borrow_mut().handle_control(clock, param);
            }
        }
    }
}

pub trait SinksControl: Debug {
    fn handle_control(&mut self, clock: &Clock, param: &SinksControlParam);
}

pub enum SinksControlParam {
    Primary {
        value: f32,
    },
    #[allow(dead_code)]
    Secondary {
        value: f32,
    },
}

pub trait SourcesMidi {
    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>;
    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>;

    fn add_midi_sink(&mut self, channel: MidiChannel, sink: Weak<RefCell<dyn SinksMidi>>) {
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
///
/// WatchesClock::tick() must be called exactly once for every sample, and
/// they can assume that they won't be asked to provide anything until tick()
/// has been called for the time slice. For example, nobody will ask an Envelope
/// for its amplitude until it's gotten a chance to calculate its amplitude for
/// the time slice via tick().
pub trait WatchesClock: Debug {
    /// returns true if we had a finite amount of known work that has finished.
    /// TODO: if we return true, then do we still expect to be called for the
    /// result of our work during this cycle? E.g., source_audio()
    ///
    /// If you're not sure what you should return, you should return true.
    /// This is because false prevents outer loops from ending.
    fn tick(&mut self, clock: &Clock) -> bool;
}

// WORKING ASSERTION: WatchesClock should not also SourcesAudio, because
// WatchesClock gets a clock tick, whereas SourcesAudio gets a sources_audio(), and
// both are time slice-y. Be on the lookout for anything that claims to need both.
pub trait IsMidiInstrument: SourcesAudio + SinksMidi {}
pub trait IsEffect: SourcesAudio + SinksAudio + TransformsAudio + SinksControl {}
pub trait IsMidiEffect: SourcesMidi + SinksMidi + WatchesClock {}
pub trait IsController: SourcesControl + WatchesClock {}

#[cfg(test)]
pub mod tests {

    use assert_approx_eq::assert_approx_eq;
    use convert_case::{Case, Casing};
    use plotters::prelude::*;
    use std::cell::RefCell;
    use std::collections::{HashMap, VecDeque};
    use std::fs;
    use std::rc::{Rc, Weak};

    use crate::clock::{ClockTimeUnit, WatchedClock};
    use crate::midi::{MidiChannel, MidiMessage, MidiMessageType, MidiNote};
    use crate::preset::EnvelopePreset;
    use crate::utils::tests::{
        TestControlSourceContinuous, TestOrchestrator, TestSynth, TestTimer, TestTrigger,
    };
    use crate::{clock::Clock, envelopes::AdsrEnvelope, oscillators::Oscillator};
    use crate::{common::MonoSample, settings::ClockSettings};
    use crate::{common::MONO_SAMPLE_SILENCE, effects::gain::Gain};

    use super::{
        IsController, SinksAudio, SinksControl, SinksControlParam, SinksMidi, SourcesAudio,
        SourcesControl, SourcesMidi, WatchesClock,
    };

    pub fn canonicalize_filename(filename: &str) -> String {
        const OUT_DIR: &str = "out";
        let result = fs::create_dir_all(OUT_DIR);
        if result.is_err() {
            panic!();
        }
        let snake_filename = filename.to_case(Case::Snake);
        format!("{}/{}.wav", OUT_DIR, snake_filename)
    }

    pub fn canonicalize_fft_filename(filename: &str) -> String {
        const OUT_DIR: &str = "out";
        let snake_filename = filename.to_case(Case::Snake);
        format!("{}/{}-spectrum", OUT_DIR, snake_filename)
    }

    pub(crate) fn write_source_to_file(source: &mut dyn SourcesAudio, basename: &str) {
        let clock_settings = ClockSettings::new_defaults();
        let mut samples = Vec::<MonoSample>::new();
        let mut clock = Clock::new_with(&clock_settings);
        while clock.seconds() < 2.0 {
            samples.push(source.source_audio(&clock));
            clock.tick();
        }
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock_settings.sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
        for sample in samples.iter() {
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
        }
        generate_fft_for_samples(
            &clock_settings,
            &samples,
            &canonicalize_fft_filename(basename),
        );
    }

    pub(crate) fn write_orchestration_to_file(
        orchestrator: &mut TestOrchestrator,
        clock: &mut WatchedClock,
        basename: &str,
    ) {
        let mut samples = Vec::<MonoSample>::new();
        orchestrator.start(clock, &mut samples);
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.inner_clock().settings().sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
        for sample in samples.iter() {
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
        }
        generate_fft_for_samples(
            clock.inner_clock().settings(),
            &samples,
            &canonicalize_fft_filename(basename),
        );
    }

    pub(crate) fn write_effect_to_file(
        effect: &mut dyn SourcesAudio,
        opt_controller: &mut dyn IsController,
        basename: &str,
    ) {
        let clock_settings = ClockSettings::new_defaults();
        let mut clock = Clock::new_with(&clock_settings);
        let mut samples = Vec::<MonoSample>::new();
        while clock.seconds() < 2.0 {
            opt_controller.tick(&clock);

            let effect_sample = effect.source_audio(&clock);
            samples.push(effect_sample);
            clock.tick();
        }

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.settings().sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
        for sample in samples.iter() {
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
        }
        generate_fft_for_samples(
            &clock_settings,
            &samples,
            &canonicalize_fft_filename(basename),
        );
    }

    use spectrum_analyzer::scaling::divide_by_N;
    use spectrum_analyzer::windows::hann_window;
    use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};

    use std::error::Error;
    fn generate_chart(
        data: &Vec<(f32, f32)>,
        min_domain: f32,
        max_domain: f32,
        min_range: f32,
        max_range: f32,
        filename: &str,
    ) -> Result<(), Box<dyn Error>> {
        let out_filename = format!("{}.png", filename);
        let root = BitMapBackend::new(out_filename.as_str(), (640, 360)).into_drawing_area();
        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .margin(0)
            .x_label_area_size(20)
            .y_label_area_size(0)
            .build_cartesian_2d(
                IntoLogRange::log_scale(min_domain..max_domain),
                IntoLogRange::log_scale(min_range..max_range),
            )?;
        chart.configure_mesh().disable_mesh().draw()?;
        chart.draw_series(LineSeries::new(data.iter().map(|t| (t.0, t.1)), &BLUE))?;

        root.present()?;

        Ok(())
    }

    pub(crate) fn generate_fft_for_samples(
        clock_settings: &ClockSettings,
        samples: &Vec<f32>,
        filename: &str,
    ) {
        const HANN_WINDOW_LENGTH: usize = 2048;
        assert!(samples.len() >= HANN_WINDOW_LENGTH);
        let hann_window = hann_window(&samples[0..HANN_WINDOW_LENGTH]);
        let spectrum_hann_window = samples_fft_to_spectrum(
            &hann_window,
            clock_settings.sample_rate() as u32,
            FrequencyLimit::All,
            Some(&divide_by_N),
        )
        .unwrap();

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut data = Vec::<(f32, f32)>::new();
        for hwd in spectrum_hann_window.data().iter() {
            let mut y = hwd.1.val();
            if y == 0.0 {
                y = f32::EPSILON;
            }
            data.push((hwd.0.val(), y));
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }

        let _ = generate_chart(
            &data,
            0.0,
            clock_settings.sample_rate() as f32 / 2.0,
            min_y,
            max_y,
            filename,
        );
    }

    #[test]
    fn test_simple_orchestrator() {
        let mut clock = WatchedClock::new();
        let mut orchestrator = TestOrchestrator::new();
        let envelope = Rc::new(RefCell::new(AdsrEnvelope::new_with(
            &EnvelopePreset::default(),
        )));
        let oscillator = Rc::new(RefCell::new(Oscillator::new_with(
            crate::common::WaveformType::Sine,
        )));
        oscillator
            .borrow_mut()
            .set_frequency(MidiMessage::note_to_frequency(60));
        let envelope_synth_clone = Rc::clone(&envelope);
        let synth = Rc::new(RefCell::new(TestSynth::new_with(
            oscillator,
            envelope_synth_clone,
        )));
        let effect = Rc::new(RefCell::new(Gain::new()));
        let source = Rc::clone(&synth);
        effect.borrow_mut().add_audio_source(source);
        let weak_effect = Rc::downgrade(&effect);
        orchestrator.add_audio_source(effect);

        let mut controller = TestControlSourceContinuous::new_with(Box::new(Oscillator::new()));
        controller.add_control_sink(weak_effect);

        let timer = TestTimer::new(2.0);
        clock.add_watcher(Rc::new(RefCell::new(timer)));

        let mut trigger_on = TestTrigger::new(1.0, 1.0);
        let weak_envelope_on = Rc::downgrade(&envelope);
        trigger_on.add_control_sink(weak_envelope_on);
        clock.add_watcher(Rc::new(RefCell::new(trigger_on)));

        let mut trigger_off = TestTrigger::new(1.5, 0.0);
        let weak_envelope_off = Rc::downgrade(&envelope);
        trigger_off.add_control_sink(weak_envelope_off);
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

    /// Keeps asking for time slices until end of specified lifetime.
    #[derive(Debug, Default)]
    struct TestWatchesClock {
        lifetime_seconds: f32,
    }

    impl WatchesClock for TestWatchesClock {
        fn tick(&mut self, clock: &Clock) -> bool {
            clock.seconds() >= self.lifetime_seconds
        }
    }

    impl TestWatchesClock {
        pub fn new(lifetime_seconds: f32) -> Self {
            Self { lifetime_seconds }
        }
    }

    #[derive(Debug, Default)]
    struct TestAudioSource {}

    impl SourcesAudio for TestAudioSource {
        fn source_audio(&mut self, _clock: &Clock) -> crate::common::MonoSample {
            0.
        }
    }

    impl TestAudioSource {
        fn new() -> Self {
            TestAudioSource {}
        }
    }

    #[derive(Debug, Default)]
    struct TestSinksAudio {
        sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
    }
    impl SinksAudio for TestSinksAudio {
        fn sources(&self) -> &[Rc<RefCell<dyn SourcesAudio>>] {
            &self.sources
        }
        fn sources_mut(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
            &mut self.sources
        }
    }
    impl TestSinksAudio {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    #[derive(Debug, Default)]
    struct TestAutomationSource {
        control_sinks: Vec<Weak<RefCell<dyn SinksControl>>>,
    }

    impl SourcesControl for TestAutomationSource {
        fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>> {
            &mut self.control_sinks
        }
    }

    impl TestAutomationSource {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        fn handle_test_event(&mut self, value: f32) {
            self.issue_control(&Clock::new(), &SinksControlParam::Primary { value });
        }
    }

    #[derive(Debug, Default)]
    struct TestAutomationSink {
        value: f32,
    }

    impl SinksControl for TestAutomationSink {
        fn handle_control(&mut self, _clock: &Clock, param: &SinksControlParam) {
            match param {
                SinksControlParam::Primary { value } => {
                    self.value = *value;
                }
                #[allow(unused_variables)]
                SinksControlParam::Secondary { value } => todo!(),
            }
        }
    }

    impl TestAutomationSink {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    #[derive(Debug, Default)]
    struct TestMidiSource {
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>,
    }

    impl SourcesMidi for TestMidiSource {
        fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &self.channels_to_sink_vecs
        }
        fn midi_sinks_mut(
            &mut self,
        ) -> &mut HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &mut self.channels_to_sink_vecs
        }
    }

    impl TestMidiSource {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        fn source_some_midi(&mut self, clock: &Clock) {
            let message =
                MidiMessage::new_note_on(TestSinksMidi::MIDI_CHANNEL, MidiNote::C4 as u8, 100);
            self.issue_midi(clock, &message);
        }
    }

    #[derive(Debug, Default)]
    struct TestSinksMidi {
        midi_channel: MidiChannel,
        is_note_on: bool,
    }

    impl SinksMidi for TestSinksMidi {
        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }

        fn handle_midi_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
            match message.status {
                MidiMessageType::NoteOn => {
                    self.is_note_on = true;
                }
                MidiMessageType::NoteOff => {
                    self.is_note_on = false;
                }
                MidiMessageType::ProgramChange => todo!(),
            }
        }

        fn midi_channel(&self) -> MidiChannel {
            Self::MIDI_CHANNEL
        }
    }

    impl TestSinksMidi {
        const MIDI_CHANNEL: MidiChannel = 7;

        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    // Gets called with native functions telling it about external keyboard events.
    // Translates those into automation events that influence an arpeggiator,
    // which controls a MIDI instrument.
    //
    // This shows how all these traits work together.
    #[derive(Debug, Default)]
    struct TestSimpleKeyboard {
        control_sinks: Vec<Weak<RefCell<dyn SinksControl>>>,
    }

    impl SourcesControl for TestSimpleKeyboard {
        fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>> {
            &mut self.control_sinks
        }
    }

    impl TestSimpleKeyboard {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        fn handle_keypress(&mut self, key: u8) {
            match key {
                1 => {
                    self.issue_control(&Clock::new(), &SinksControlParam::Primary { value: 0.5 });
                }
                _ => {}
            }
        }
    }

    #[derive(Debug, Default)]
    struct TestSimpleArpeggiator {
        tempo: f32,
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>,
    }

    impl SinksControl for TestSimpleArpeggiator {
        fn handle_control(&mut self, _clock: &Clock, param: &SinksControlParam) {
            match param {
                SinksControlParam::Primary { value } => self.tempo = *value,
                #[allow(unused_variables)]
                SinksControlParam::Secondary { value } => todo!(),
            }
        }
    }

    impl SourcesMidi for TestSimpleArpeggiator {
        fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &self.channels_to_sink_vecs
        }
        fn midi_sinks_mut(
            &mut self,
        ) -> &mut HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &mut self.channels_to_sink_vecs
        }
    }

    impl WatchesClock for TestSimpleArpeggiator {
        fn tick(&mut self, clock: &Clock) -> bool {
            // We don't actually pay any attention to self.tempo, but it's easy
            // enough to see that tempo could have influenced this MIDI message.
            self.issue_midi(
                &clock,
                &MidiMessage::new_note_on(TestSinksMidi::MIDI_CHANNEL, 60, 100),
            );
            true
        }
    }

    impl TestSimpleArpeggiator {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    #[derive(Debug)]
    pub struct TestValueChecker {
        pub values: VecDeque<f32>,
        pub target: Rc<RefCell<dyn SourcesAudio>>,
        pub checkpoint: f32,
        pub checkpoint_delta: f32,
        pub time_unit: ClockTimeUnit,
    }

    impl WatchesClock for TestValueChecker {
        fn tick(&mut self, clock: &Clock) -> bool {
            if clock.time_for(&self.time_unit) >= self.checkpoint {
                const SAD_FLOAT_DIFF: f32 = 1.0e-4;
                assert_approx_eq!(
                    self.target.borrow_mut().source_audio(clock),
                    self.values[0],
                    SAD_FLOAT_DIFF
                );
                self.checkpoint += self.checkpoint_delta;
                self.values.pop_front();
            }
            self.values.is_empty()
        }
    }

    #[test]
    fn test_time_slicer() {
        let mut clock = Clock::new_test();
        let mut time_slicer = TestWatchesClock::new(1.0);

        loop {
            clock.tick();
            if time_slicer.tick(&mut clock) {
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
        let mut sink = TestSinksAudio::new();
        let source = Rc::new(RefCell::new(TestAudioSource::new()));
        assert!(sink.sources().is_empty());
        sink.add_audio_source(source);
        assert_eq!(sink.sources().len(), 1);
    }

    #[test]
    fn test_automation_source_and_sink() {
        // By itself, TestAutomationSource doesn't do much, so we test both Source/Sink together.
        let mut source = TestAutomationSource::new();
        let sink = Rc::new(RefCell::new(TestAutomationSink::new()));

        // Can we add a sink to the source?
        assert!(source.control_sinks().is_empty());
        let sink_weak = Rc::downgrade(&sink);
        source.add_control_sink(sink_weak);
        assert!(!source.control_sinks().is_empty());

        // Does the source propagate to its sinks?
        assert_eq!(sink.borrow().value, 0.0);
        source.handle_test_event(42.0);
        assert_eq!(sink.borrow().value, 42.0);
    }

    #[test]
    fn test_midi_source_and_sink() {
        let mut source = TestMidiSource::new();
        let sink = Rc::new(RefCell::new(TestSinksMidi::new()));

        assert!(source.midi_sinks().is_empty());
        let sink_down = Rc::downgrade(&sink);
        source.add_midi_sink(7, sink_down);
        assert!(!source.midi_sinks().is_empty());

        let clock = Clock::new_test();
        assert!(!sink.borrow().is_note_on);
        source.source_some_midi(&clock);
        assert!(sink.borrow().is_note_on);
    }

    #[test]
    fn test_keyboard_to_automation_to_midi() {
        let mut keyboard_interface = TestSimpleKeyboard::new();
        let arpeggiator = Rc::new(RefCell::new(TestSimpleArpeggiator::new()));
        let instrument = Rc::new(RefCell::new(TestSinksMidi::new()));

        let arpeggiator_weak = Rc::downgrade(&arpeggiator);
        keyboard_interface.add_control_sink(arpeggiator_weak);
        let sink = Rc::downgrade(&instrument);
        arpeggiator
            .borrow_mut()
            .add_midi_sink(TestSinksMidi::MIDI_CHANNEL, sink);

        assert_eq!(arpeggiator.borrow().tempo, 0.0);
        keyboard_interface.handle_keypress(1); // set tempo to 50%
        assert_eq!(arpeggiator.borrow().tempo, 0.5);

        let clock = Clock::new_test();

        assert!(!instrument.borrow().is_note_on);
        arpeggiator.borrow_mut().tick(&clock);
        assert!(instrument.borrow().is_note_on);
    }
}
