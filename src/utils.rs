#[cfg(test)]
pub mod tests {
    use crate::{
        clock::{Clock, ClockTimeUnit, WatchedClock},
        common::{rrc, MonoSample, Rrc, Ww, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN, MONO_SAMPLE_SILENCE},
        effects::mixer::Mixer,
        envelopes::AdsrEnvelope,
        midi::{MidiChannel, MidiMessage, MidiMessageType, MidiNote, MIDI_CHANNEL_RECEIVE_ALL},
        oscillators::Oscillator,
        settings::patches::EnvelopeSettings,
        settings::patches::WaveformType,
        settings::ClockSettings,
        traits::{
            IsController, IsEffect, IsMutable, MakesControlSink, SinksAudio, SinksControl,
            SinksMidi, SourcesAudio, SourcesControl, SourcesMidi, Terminates, WatchesClock,
        },
    };
    use assert_approx_eq::assert_approx_eq;
    use convert_case::{Case, Casing};
   // use plotters::prelude::*;
    use spectrum_analyzer::{
        samples_fft_to_spectrum, scaling::divide_by_N, windows::hann_window, FrequencyLimit,
    };
    use std::cell::RefCell;
    use std::collections::{HashMap, VecDeque};
    use std::fs;
    use std::rc::{Rc, Weak};

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

    fn write_samples_to_wav_file(basename: &str, sample_rate: usize, samples: &Vec<MonoSample>) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: sample_rate as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
        for sample in samples.iter() {
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
        }
    }

    pub(crate) fn write_source_and_controlled_effect(
        basename: &str,
        waveform_type: WaveformType,
        effect_opt: Option<Rrc<dyn IsEffect>>,
        control_opt: Option<Rrc<dyn WatchesClock>>,
    ) {
        let mut c = WatchedClock::new();
        let sample_rate = c.inner_clock().settings().sample_rate();
        let mut o = TestOrchestrator::new();
        let osc = Oscillator::new_wrapped_with(waveform_type);
        if let Some(effect) = effect_opt {
            let osc_weak = Rc::downgrade(&osc);
            effect.borrow_mut().add_audio_source(osc_weak);
            let effect_weak = Rc::downgrade(&effect);
            o.add_audio_source(effect_weak);
        }
        c.add_watcher(rrc(TestTimer::new_with(2.0)));
        if let Some(control) = control_opt {
            let watcher = Rc::clone(&control);
            c.add_watcher(watcher);
        }
        let mut samples_out = Vec::<MonoSample>::new();
        o.start(&mut c, &mut samples_out);
        write_samples_to_wav_file(basename, sample_rate, &samples_out);
    }

    /////////////////////
    /// DEDUPLICATE vvvvv
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

    // use std::error::Error;
    // fn generate_chart(
    //     data: &Vec<(f32, f32)>,
    //     min_domain: f32,
    //     max_domain: f32,
    //     min_range: f32,
    //     max_range: f32,
    //     filename: &str,
    // ) -> Result<(), Box<dyn Error>> {
    //     let out_filename = format!("{}.png", filename);
    //     let root = BitMapBackend::new(out_filename.as_str(), (640, 360)).into_drawing_area();
    //     root.fill(&WHITE)?;

    //     let mut chart = ChartBuilder::on(&root)
    //         .margin(0)
    //         .x_label_area_size(20)
    //         .y_label_area_size(0)
    //         .build_cartesian_2d(
    //             IntoLogRange::log_scale(min_domain..max_domain),
    //             IntoLogRange::log_scale(min_range..max_range),
    //         )?;
    //     chart.configure_mesh().disable_mesh().draw()?;
    //     chart.draw_series(LineSeries::new(data.iter().map(|t| (t.0, t.1)), &BLUE))?;

    //     root.present()?;

    //     Ok(())
    // }

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

        // let _ = generate_chart(
        //     &data,
        //     0.0,
        //     clock_settings.sample_rate() as f32 / 2.0,
        //     min_y,
        //     max_y,
        //     filename,
        // );
    }

    /// /////////////////

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysSameLevel {
        level: MonoSample,
        is_muted: bool,
    }
    impl TestAudioSourceAlwaysSameLevel {
        pub fn new(level: MonoSample) -> Self {
            Self {
                level,
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysSameLevel {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            self.level
        }
    }
    impl IsMutable for TestAudioSourceAlwaysSameLevel {
        fn is_muted(&self) -> bool {
            self.is_muted
        }

        fn set_muted(&mut self, is_muted: bool) {
            self.is_muted = is_muted;
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysLoud {
        is_muted: bool,
    }
    impl TestAudioSourceAlwaysLoud {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysLoud {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_MAX
        }
    }
    impl IsMutable for TestAudioSourceAlwaysLoud {
        fn is_muted(&self) -> bool {
            self.is_muted
        }

        fn set_muted(&mut self, is_muted: bool) {
            self.is_muted = is_muted;
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysTooLoud {
        is_muted: bool,
    }
    impl TestAudioSourceAlwaysTooLoud {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysTooLoud {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_MAX + 0.1
        }
    }
    impl IsMutable for TestAudioSourceAlwaysTooLoud {
        fn is_muted(&self) -> bool {
            self.is_muted
        }

        fn set_muted(&mut self, is_muted: bool) {
            self.is_muted = is_muted;
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysSilent {
        is_muted: bool,
    }
    impl TestAudioSourceAlwaysSilent {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysSilent {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_SILENCE
        }
    }
    impl IsMutable for TestAudioSourceAlwaysSilent {
        fn is_muted(&self) -> bool {
            self.is_muted
        }

        fn set_muted(&mut self, is_muted: bool) {
            self.is_muted = is_muted;
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysVeryQuiet {
        is_muted: bool,
    }
    impl TestAudioSourceAlwaysVeryQuiet {
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysVeryQuiet {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_MIN
        }
    }
    impl IsMutable for TestAudioSourceAlwaysVeryQuiet {
        fn is_muted(&self) -> bool {
            self.is_muted
        }

        fn set_muted(&mut self, is_muted: bool) {
            self.is_muted = is_muted;
        }
    }

    #[derive(Debug)]
    pub struct TestOrchestrator {
        main_mixer: Box<dyn IsEffect>,
    }

    impl Default for TestOrchestrator {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TestOrchestrator {
        pub fn new() -> Self {
            Self {
                main_mixer: Box::new(Mixer::new()),
            }
        }

        // TODO: I like "new_with"
        #[allow(dead_code)]
        pub fn new_with(main_mixer: Box<dyn IsEffect>) -> Self {
            Self { main_mixer }
        }

        pub fn add_audio_source(&mut self, source: Ww<dyn SourcesAudio>) {
            self.main_mixer.add_audio_source(source);
        }

        pub fn start(&mut self, clock: &mut WatchedClock, samples_out: &mut Vec<MonoSample>) {
            loop {
                if clock.visit_watchers() {
                    break;
                }
                samples_out.push(self.main_mixer.source_audio(clock.inner_clock()));
                clock.tick();
            }
        }
    }

    #[derive(Debug)]
    pub struct TestSynth {
        oscillator: Rc<RefCell<dyn SourcesAudio>>,
        envelope: Rc<RefCell<dyn SourcesAudio>>,
        is_muted: bool,
    }

    impl TestSynth {
        #[deprecated]
        /// You really don't want to call this, because you need a sample rate
        /// for it to do anything meaningful, and it's a bad practice to hardcode
        /// a 44.1KHz rate.
        fn new() -> Self {
            Self::new_with(
                rrc(Oscillator::new()),
                rrc(AdsrEnvelope::new_with(&EnvelopeSettings::default())),
            )
        }
        pub fn new_with(
            oscillator: Rc<RefCell<dyn SourcesAudio>>,
            envelope: Rc<RefCell<dyn SourcesAudio>>,
        ) -> Self {
            Self {
                oscillator,
                envelope,
                is_muted: false,
            }
        }
    }

    impl Default for TestSynth {
        fn default() -> Self {
            #[allow(deprecated)]
            Self::new()
        }
    }

    impl SourcesAudio for TestSynth {
        fn source_audio(&mut self, clock: &Clock) -> MonoSample {
            self.oscillator.borrow_mut().source_audio(clock)
                * self.envelope.borrow_mut().source_audio(clock)
        }
    }
    impl IsMutable for TestSynth {
        fn is_muted(&self) -> bool {
            self.is_muted
        }

        fn set_muted(&mut self, is_muted: bool) {
            self.is_muted = is_muted;
        }
    }

    #[derive(Debug, Default)]
    pub struct TestTimer {
        has_more_work: bool,
        time_to_run_seconds: f32,
    }
    impl TestTimer {
        pub fn new_with(time_to_run_seconds: f32) -> Self {
            Self {
                time_to_run_seconds,
                ..Default::default()
            }
        }
    }
    impl WatchesClock for TestTimer {
        fn tick(&mut self, clock: &Clock) {
            self.has_more_work = clock.seconds() < self.time_to_run_seconds;
        }
    }
    impl Terminates for TestTimer {
        fn is_finished(&self) -> bool {
            !self.has_more_work
        }
    }

    #[derive(Debug, Default)]
    pub struct TestTrigger {
        control_sinks: Vec<Box<dyn SinksControl>>,
        time_to_trigger_seconds: f32,
        value: f32,
        has_triggered: bool,
    }
    impl TestTrigger {
        pub fn new(time_to_trigger_seconds: f32, value: f32) -> Self {
            Self {
                time_to_trigger_seconds,
                value,
                ..Default::default()
            }
        }
    }
    impl WatchesClock for TestTrigger {
        fn tick(&mut self, clock: &Clock) {
            if !self.has_triggered && clock.seconds() >= self.time_to_trigger_seconds {
                self.has_triggered = true;
                let value = self.value;
                self.issue_control(clock, value);
            }
        }
    }
    impl Terminates for TestTrigger {
        fn is_finished(&self) -> bool {
            self.has_triggered
        }
    }
    impl SourcesControl for TestTrigger {
        fn control_sinks(&self) -> &[Box<dyn SinksControl>] {
            &self.control_sinks
        }

        fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
    }

    /// Lets a SourcesAudio act like an IsController
    #[derive(Debug)]
    pub struct TestControlSourceContinuous {
        control_sinks: Vec<Box<dyn SinksControl>>,
        source: Box<dyn SourcesAudio>,
    }
    impl TestControlSourceContinuous {
        pub fn new_with(source: Box<dyn SourcesAudio>) -> Self {
            Self {
                control_sinks: Vec::new(),
                source: source,
            }
        }
    }
    impl SourcesControl for TestControlSourceContinuous {
        fn control_sinks(&self) -> &[Box<dyn SinksControl>] {
            &self.control_sinks
        }

        fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
    }
    impl WatchesClock for TestControlSourceContinuous {
        fn tick(&mut self, clock: &Clock) {
            let value = self.source.source_audio(clock);
            if value < 0.0 {
                let mut _fii = 1.0 + self.source.source_audio(clock);
            }
            self.issue_control(clock, value);
        }
    }
    impl Terminates for TestControlSourceContinuous {
        fn is_finished(&self) -> bool {
            true
        }
    }
    impl IsController for TestControlSourceContinuous {}

    /// Helper for testing SinksMidi
    #[derive(Debug, Default)]
    pub struct TestMidiSink {
        pub(crate) me: Ww<Self>,
        pub is_playing: bool,
        is_muted: bool,
        midi_channel: MidiChannel,
        pub midi_messages_received: usize,
        pub midi_messages_handled: usize,
        pub value: f32,
    }

    impl TestMidiSink {
        pub const TEST_MIDI_CHANNEL: u8 = 42;
        pub const CONTROL_PARAM_DEFAULT: &str = "param";

        pub fn new() -> Self {
            Self {
                me: Weak::new(),
                midi_channel: Self::TEST_MIDI_CHANNEL,
                ..Default::default()
            }
        }
        pub fn new_wrapped() -> Rrc<Self> {
            // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

            let wrapped = Rc::new(RefCell::new(Self::new()));
            wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
            wrapped
        }
        pub fn new_with(midi_channel: MidiChannel) -> Self {
            Self {
                midi_channel,
                ..Default::default()
            }
        }
        #[allow(dead_code)]
        pub fn new_wrapped_with(midi_channel: MidiChannel) -> Rrc<Self> {
            // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

            let wrapped = Rc::new(RefCell::new(Self::new_with(midi_channel)));
            wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
            wrapped
        }
        pub fn set_value(&mut self, value: f32) {
            self.value = value;
        }
    }
    impl SinksMidi for TestMidiSink {
        fn midi_channel(&self) -> MidiChannel {
            self.midi_channel
        }

        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }
        fn handle_midi_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
            self.midi_messages_received += 1;

            match message.status {
                MidiMessageType::NoteOn => {
                    self.is_playing = true;
                    self.midi_messages_handled += 1;
                }
                MidiMessageType::NoteOff => {
                    self.is_playing = false;
                    self.midi_messages_handled += 1;
                }
                MidiMessageType::ProgramChange => {
                    self.midi_messages_handled += 1;
                }
            }
        }
    }
    impl SourcesAudio for TestMidiSink {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            self.value
        }
    }
    impl IsMutable for TestMidiSink {
        fn is_muted(&self) -> bool {
            self.is_muted
        }

        fn set_muted(&mut self, is_muted: bool) {
            self.is_muted = is_muted;
        }
    }
    impl MakesControlSink for TestMidiSink {
        fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
            if self.me.strong_count() != 0 {
                match param_name {
                    Self::CONTROL_PARAM_DEFAULT => Some(Box::new(TestMidiSinkController {
                        target: Weak::clone(&self.me),
                    })),
                    _ => None,
                }
            } else {
                None
            }
        }
    }

    #[derive(Debug)]
    pub struct TestMidiSinkController {
        target: Ww<TestMidiSink>,
    }
    impl SinksControl for TestMidiSinkController {
        fn handle_control(&mut self, _clock: &Clock, param: f32) {
            if let Some(target) = self.target.upgrade() {
                target.borrow_mut().set_value(param);
            }
        }
    }

    /// Keeps asking for time slices until end of specified lifetime.
    #[derive(Debug, Default)]
    pub struct TestClockWatcher {
        has_more_work: bool,
        lifetime_seconds: f32,
    }

    impl WatchesClock for TestClockWatcher {
        fn tick(&mut self, clock: &Clock) {
            self.has_more_work = clock.seconds() < self.lifetime_seconds;
        }
    }

    impl Terminates for TestClockWatcher {
        fn is_finished(&self) -> bool {
            !self.has_more_work
        }
    }

    impl TestClockWatcher {
        pub fn new(lifetime_seconds: f32) -> Self {
            Self {
                lifetime_seconds,
                ..Default::default()
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSource {
        is_muted: bool,
    }

    impl SourcesAudio for TestAudioSource {
        fn source_audio(&mut self, _clock: &Clock) -> crate::common::MonoSample {
            0.
        }
    }
    impl IsMutable for TestAudioSource {
        fn is_muted(&self) -> bool {
            self.is_muted
        }

        fn set_muted(&mut self, is_muted: bool) {
            self.is_muted = is_muted;
        }
    }

    impl TestAudioSource {
        pub fn new() -> Self {
            TestAudioSource {
                ..Default::default()
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSink {
        sources: Vec<Ww<dyn SourcesAudio>>,
    }
    impl SinksAudio for TestAudioSink {
        fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
            &self.sources
        }
        fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
            &mut self.sources
        }
    }
    impl TestAudioSink {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestControlSource {
        control_sinks: Vec<Box<dyn SinksControl>>,
    }

    impl SourcesControl for TestControlSource {
        fn control_sinks(&self) -> &[Box<dyn SinksControl>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
    }

    impl TestControlSource {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        pub fn handle_test_event(&mut self, value: f32) {
            self.issue_control(&Clock::new(), value);
        }
    }

    #[derive(Debug, Default)]
    pub struct TestControllable {
        pub(crate) me: Weak<RefCell<Self>>,
        pub value: f32,
    }
    impl TestControllable {
        pub(crate) const CONTROL_PARAM_DEFAULT: &str = "value";

        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
        pub fn new_wrapped() -> Rrc<Self> {
            // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

            let wrapped = Rc::new(RefCell::new(Self::new()));
            wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
            wrapped
        }
    }
    impl MakesControlSink for TestControllable {
        fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
            if self.me.strong_count() != 0 {
                match param_name {
                    Self::CONTROL_PARAM_DEFAULT => Some(Box::new(TestControllableController {
                        target: Weak::clone(&self.me),
                    })),
                    _ => None,
                }
            } else {
                None
            }
        }
    }

    #[derive(Debug)]
    pub(crate) struct TestControllableController {
        target: Ww<TestControllable>,
    }
    impl SinksControl for TestControllableController {
        fn handle_control(&mut self, _clock: &Clock, param: f32) {
            if let Some(target) = self.target.upgrade() {
                target.borrow_mut().value = param;
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestMidiSource {
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

        fn midi_output_channel(&self) -> MidiChannel {
            MIDI_CHANNEL_RECEIVE_ALL
        }

        fn set_midi_output_channel(&mut self, _midi_channel: MidiChannel) {}
    }

    impl TestMidiSource {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        pub fn source_some_midi(&mut self, clock: &Clock) {
            let message =
                MidiMessage::new_note_on(TestMidiSink::TEST_MIDI_CHANNEL, MidiNote::C4 as u8, 100);
            self.issue_midi(clock, &message);
        }
    }

    // Gets called with native functions telling it about external keyboard events.
    // Translates those into automation events that influence an arpeggiator,
    // which controls a MIDI instrument.
    //
    // This shows how all these traits work together.
    #[derive(Debug, Default)]
    pub struct TestKeyboard {
        control_sinks: Vec<Box<dyn SinksControl>>,
    }

    impl SourcesControl for TestKeyboard {
        fn control_sinks(&self) -> &[Box<dyn SinksControl>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
    }

    impl TestKeyboard {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        pub fn handle_keypress(&mut self, key: u8) {
            match key {
                1 => {
                    self.issue_control(&Clock::new(), 0.5);
                }
                _ => {}
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestArpeggiator {
        me: Weak<RefCell<Self>>,
        midi_channel_out: MidiChannel,
        pub tempo: f32,
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>,
    }
    impl SourcesMidi for TestArpeggiator {
        fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &self.channels_to_sink_vecs
        }
        fn midi_sinks_mut(
            &mut self,
        ) -> &mut HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &mut self.channels_to_sink_vecs
        }

        fn midi_output_channel(&self) -> MidiChannel {
            self.midi_channel_out
        }

        fn set_midi_output_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel_out = midi_channel;
        }
    }
    impl WatchesClock for TestArpeggiator {
        fn tick(&mut self, clock: &Clock) {
            // We don't actually pay any attention to self.tempo, but it's easy
            // enough to see that tempo could have influenced this MIDI message.
            self.issue_midi(
                &clock,
                &MidiMessage::new_note_on(self.midi_channel_out, 60, 100),
            );
        }
    }
    impl Terminates for TestArpeggiator {
        fn is_finished(&self) -> bool {
            true
        }
    }
    impl MakesControlSink for TestArpeggiator {
        fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
            if self.me.strong_count() != 0 {
                match param_name {
                    Self::CONTROL_PARAM_TEMPO => Some(Box::new(TestArpeggiatorTempoController {
                        target: Weak::clone(&self.me),
                    })),
                    _ => None,
                }
            } else {
                None
            }
        }
    }
    impl TestArpeggiator {
        pub(crate) const CONTROL_PARAM_TEMPO: &str = "tempo";

        pub fn new_with(midi_channel_out: MidiChannel) -> Self {
            Self {
                midi_channel_out,
                ..Default::default()
            }
        }
        pub fn new_wrapped_with(midi_channel_out: MidiChannel) -> Rrc<Self> {
            // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

            let wrapped = Rc::new(RefCell::new(Self::new_with(midi_channel_out)));
            wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
            wrapped
        }
    }

    #[derive(Debug)]
    pub struct TestArpeggiatorTempoController {
        target: Ww<TestArpeggiator>,
    }
    impl SinksControl for TestArpeggiatorTempoController {
        fn handle_control(&mut self, _clock: &Clock, param: f32) {
            if let Some(target) = self.target.upgrade() {
                target.borrow_mut().tempo = param;
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
        fn tick(&mut self, clock: &Clock) {
            // We have to check is_empty() twice
            // because we might still get called
            // back if someone else isn't done yet.
            if self.values.is_empty() {
                return;
            }
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
        }
    }

    impl Terminates for TestValueChecker {
        fn is_finished(&self) -> bool {
            self.values.is_empty()
        }
    }
}
