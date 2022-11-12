#[cfg(test)]
pub mod tests {
    use crate::{
        clock::{Clock, ClockTimeUnit, WatchedClock},
        common::{
            rrc, rrc_clone, rrc_downgrade, MonoSample, Rrc, Ww, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN,
            MONO_SAMPLE_SILENCE,
        },
        control::{BigMessage, SmallMessage, SmallMessageGenerator},
        effects::mixer::Mixer,
        envelopes::AdsrEnvelope,
        midi::{MidiChannel, MidiMessage, MidiNote, MidiUtils, MIDI_CHANNEL_RECEIVE_ALL},
        oscillators::Oscillator,
        settings::patches::EnvelopeSettings,
        settings::patches::WaveformType,
        settings::ClockSettings,
        traits::{
            HasOverhead, IsEffect, Overhead, SinksAudio, SinksMidi, SinksUpdates, SourcesAudio,
            SourcesMidi, SourcesUpdates, Terminates, WatchesClock,
        },
    };
    use assert_approx_eq::assert_approx_eq;
    use convert_case::{Case, Casing};
    // use plotters::prelude::*;
    use spectrum_analyzer::{
        samples_fft_to_spectrum, scaling::divide_by_N, windows::hann_window, FrequencyLimit,
    };
    use std::collections::{HashMap, VecDeque};
    use std::fs;
    use strum_macros::{Display, EnumString};

    pub fn canonicalize_filename(filename: &str) -> String {
        const OUT_DIR: &str = "out";
        let result = fs::create_dir_all(OUT_DIR);
        if result.is_err() {
            panic!();
        }
        let snake_filename = filename.to_case(Case::Snake);
        format!("{OUT_DIR}/{snake_filename}.wav")
    }

    pub fn canonicalize_fft_filename(filename: &str) -> String {
        const OUT_DIR: &str = "out";
        let snake_filename = filename.to_case(Case::Snake);
        format!("{OUT_DIR}/{snake_filename}-spectrum")
    }

    fn write_samples_to_wav_file(basename: &str, sample_rate: usize, samples: &[MonoSample]) {
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
        let sample_rate = c.inner_clock().sample_rate();
        let mut o = TestOrchestrator::new();
        let osc = Oscillator::new_wrapped_with(waveform_type);
        if let Some(effect) = effect_opt {
            effect
                .borrow_mut()
                .add_audio_source(rrc_downgrade::<Oscillator>(&osc));
            o.add_audio_source(rrc_downgrade::<dyn IsEffect>(&effect));
        }
        c.add_watcher(rrc(TestTimer::new_with(2.0)));
        if let Some(control) = control_opt {
            c.add_watcher(rrc_clone::<dyn WatchesClock>(&control));
        }
        let mut samples_out = Vec::<MonoSample>::new();
        o.start(&mut c, &mut samples_out);
        write_samples_to_wav_file(basename, sample_rate, &samples_out);
    }

    ///////////////////// DEDUPLICATE vvvvv
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
            sample_rate: clock.sample_rate() as u32,
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
            sample_rate: clock.inner_clock().sample_rate() as u32,
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

    //     let mut chart = ChartBuilder::on(&root) .margin(0)
    //         .x_label_area_size(20) .y_label_area_size(0) .build_cartesian_2d(
    //         IntoLogRange::log_scale(min_domain..max_domain),
    //         IntoLogRange::log_scale(min_range..max_range), )?;
    //         chart.configure_mesh().disable_mesh().draw()?;
    //             chart.draw_series(LineSeries::new(data.iter().map(|t| (t.0,
    //             t.1)), &BLUE))?;

    //     root.present()?;

    //     Ok(())
    // }

    pub(crate) fn generate_fft_for_samples(
        clock_settings: &ClockSettings,
        samples: &Vec<f32>,
        _filename: &str,
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
        overhead: Overhead,

        level: MonoSample,
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
    impl HasOverhead for TestAudioSourceAlwaysSameLevel {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysLoud {
        overhead: Overhead,
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
    impl HasOverhead for TestAudioSourceAlwaysLoud {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysTooLoud {
        overhead: Overhead,
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
    impl HasOverhead for TestAudioSourceAlwaysTooLoud {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysSilent {
        overhead: Overhead,
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
    impl HasOverhead for TestAudioSourceAlwaysSilent {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysVeryQuiet {
        overhead: Overhead,
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
    impl HasOverhead for TestAudioSourceAlwaysVeryQuiet {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
        }
    }

    #[derive(Debug)]
    pub struct TestOrchestrator {
        pub main_mixer: Box<dyn IsEffect>,
        pub updateables: HashMap<usize, Ww<dyn SinksUpdates>>,

        // The final clock watcher gets to run and test the state resulting from
        // all the prior clock watchers' ticks.
        pub final_clock_watcher: Option<Ww<dyn WatchesClock>>,
    }

    impl Default for TestOrchestrator {
        fn default() -> Self {
            Self {
                main_mixer: Box::new(Mixer::new()),
                updateables: Default::default(),
                final_clock_watcher: None,
            }
        }
    }

    impl TestOrchestrator {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        #[allow(dead_code)]
        pub fn new_with(main_mixer: Box<dyn IsEffect>) -> Self {
            Self {
                main_mixer,
                ..Default::default()
            }
        }

        pub fn add_audio_source(&mut self, source: Ww<dyn SourcesAudio>) {
            self.main_mixer.add_audio_source(source);
        }

        pub fn add_final_watcher(&mut self, watcher: Rrc<dyn WatchesClock>) {
            self.final_clock_watcher = Some(rrc_downgrade(&watcher));
        }

        pub fn start(&mut self, clock: &mut WatchedClock, samples_out: &mut Vec<MonoSample>) {
            loop {
                let (mut done, messages) = clock.visit_watchers();
                if let Some(watcher) = &self.final_clock_watcher {
                    if let Some(watcher) = watcher.upgrade() {
                        watcher.borrow_mut().tick(&clock.inner_clock());
                        done = done && watcher.borrow().is_finished();
                    }
                }
                if done {
                    break;
                }
                for message in messages {
                    match message {
                        crate::control::BigMessage::SmallMessage(uid, message) => {
                            if let Some(target) = self.updateables.get_mut(&uid) {
                                if let Some(target) = target.upgrade() {
                                    target.borrow_mut().update(clock.inner_clock(), message);
                                }
                            }
                        }
                    }
                }
                samples_out.push(self.main_mixer.source_audio(clock.inner_clock()));
                clock.tick();
            }
        }
    }

    #[derive(Debug)]
    pub struct TestSynth {
        overhead: Overhead,

        oscillator: Rrc<dyn SourcesAudio>,
        envelope: Rrc<dyn SourcesAudio>,
    }

    impl TestSynth {
        /// You really don't want to call this, because you need a sample rate
        /// for it to do anything meaningful, and it's a bad practice to
        /// hardcode a 44.1KHz rate.
        #[deprecated]
        #[allow(dead_code)]
        fn new() -> Self {
            Self::new_with(
                rrc(Oscillator::new()),
                rrc(AdsrEnvelope::new_with(&EnvelopeSettings::default())),
            )
        }
        pub fn new_with(
            oscillator: Rrc<dyn SourcesAudio>,
            envelope: Rrc<dyn SourcesAudio>,
        ) -> Self {
            Self {
                oscillator,
                envelope,
                ..Default::default()
            }
        }
    }
    impl Default for TestSynth {
        fn default() -> Self {
            Self {
                overhead: Overhead::default(),
                oscillator: rrc(Oscillator::new()),
                envelope: rrc(AdsrEnvelope::new_with(&EnvelopeSettings::default())),
            }
        }
    }
    impl SourcesAudio for TestSynth {
        fn source_audio(&mut self, clock: &Clock) -> MonoSample {
            self.oscillator.borrow_mut().source_audio(clock)
                * self.envelope.borrow_mut().source_audio(clock)
        }
    }
    impl HasOverhead for TestSynth {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
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
        fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
            self.has_more_work = clock.seconds() < self.time_to_run_seconds;
            Vec::new()
        }
    }
    impl Terminates for TestTimer {
        fn is_finished(&self) -> bool {
            !self.has_more_work
        }
    }

    #[derive(Debug, Default)]
    pub struct TestTrigger {
        target_uids: Vec<usize>,
        target_messages: Vec<SmallMessageGenerator>,

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
        fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
            if !self.has_triggered && clock.seconds() >= self.time_to_trigger_seconds {
                self.has_triggered = true;
                let value = self.value;
                self.post_message(value)
            } else {
                Vec::default()
            }
        }
    }
    impl SourcesUpdates for TestTrigger {
        fn target_uids(&self) -> &[usize] {
            &self.target_uids
        }

        fn target_uids_mut(&mut self) -> &mut Vec<usize> {
            &mut self.target_uids
        }

        fn target_messages(&self) -> &[SmallMessageGenerator] {
            &self.target_messages
        }

        fn target_messages_mut(&mut self) -> &mut Vec<SmallMessageGenerator> {
            &mut self.target_messages
        }
    }
    impl Terminates for TestTrigger {
        fn is_finished(&self) -> bool {
            self.has_triggered
        }
    }

    /// Lets a SourcesAudio act like an IsController
    #[derive(Debug)]
    pub struct TestControlSourceContinuous {
        source: Box<dyn SourcesAudio>,
        target_uids: Vec<usize>,
        target_messages: Vec<SmallMessageGenerator>,
    }
    impl TestControlSourceContinuous {
        pub fn new_with(source: Box<dyn SourcesAudio>) -> Self {
            Self {
                source,
                target_uids: Vec::new(),
                target_messages: Vec::new(),
            }
        }
    }
    impl WatchesClock for TestControlSourceContinuous {
        fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
            let value = self.source.source_audio(clock).abs();
            self.post_message(value)
        }
    }
    impl SourcesUpdates for TestControlSourceContinuous {
        fn target_uids(&self) -> &[usize] {
            &self.target_uids
        }

        fn target_uids_mut(&mut self) -> &mut Vec<usize> {
            &mut self.target_uids
        }

        fn target_messages(&self) -> &[SmallMessageGenerator] {
            &self.target_messages
        }

        fn target_messages_mut(&mut self) -> &mut Vec<SmallMessageGenerator> {
            &mut self.target_messages
        }
    }
    impl Terminates for TestControlSourceContinuous {
        fn is_finished(&self) -> bool {
            true
        }
    }

    #[derive(Display, Debug, EnumString)]
    #[strum(serialize_all = "kebab_case")]
    pub(crate) enum TestMidiSinkControlParams {
        Param,
    }

    /// Helper for testing SinksMidi
    #[derive(Debug, Default)]
    pub struct TestMidiSink {
        pub(crate) me: Ww<Self>,
        overhead: Overhead,

        pub is_playing: bool,
        midi_channel: MidiChannel,
        pub received_count: usize,
        pub handled_count: usize,
        pub value: f32,

        pub messages: Vec<(f32, MidiChannel, MidiMessage)>,
    }

    impl TestMidiSink {
        pub const TEST_MIDI_CHANNEL: u8 = 42;

        pub fn new() -> Self {
            Self {
                midi_channel: Self::TEST_MIDI_CHANNEL,
                ..Default::default()
            }
        }
        pub fn new_wrapped() -> Rrc<Self> {
            let wrapped = rrc(Self::new());
            wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
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
            let wrapped = rrc(Self::new_with(midi_channel));
            wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
            wrapped
        }
        #[allow(dead_code)]
        pub fn set_value(&mut self, value: f32) {
            self.value = value;
        }

        #[allow(dead_code)]
        pub fn dump_messages(&self) {
            dbg!(&self.messages);
        }
    }

    impl SinksMidi for TestMidiSink {
        fn midi_channel(&self) -> MidiChannel {
            self.midi_channel
        }

        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }
        fn handle_midi_for_channel(
            &mut self,
            clock: &Clock,
            channel: &MidiChannel,
            message: &MidiMessage,
        ) {
            assert_eq!(self.midi_channel, *channel);
            self.messages.push((clock.beats(), *channel, *message));
            self.received_count += 1;

            #[allow(unused_variables)]
            match message {
                MidiMessage::NoteOff { key, vel } => {
                    self.is_playing = false;
                    self.handled_count += 1;
                }
                MidiMessage::NoteOn { key, vel } => {
                    self.is_playing = true;
                    self.handled_count += 1;
                }
                MidiMessage::Aftertouch { key, vel } => todo!(),
                MidiMessage::Controller { controller, value } => todo!(),
                MidiMessage::ProgramChange { program } => {
                    self.handled_count += 1;
                }
                MidiMessage::ChannelAftertouch { vel } => todo!(),
                MidiMessage::PitchBend { bend } => todo!(),
            }
        }
    }
    impl SourcesAudio for TestMidiSink {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            self.value
        }
    }
    impl HasOverhead for TestMidiSink {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
        }
    }
    impl SinksUpdates for TestMidiSink {
        fn update(&mut self, _clock: &Clock, message: SmallMessage) {
            match message {
                SmallMessage::ValueChanged(value) => {
                    self.value = value;
                }
                _ => {
                    dbg!(&message);
                }
            }
        }

        fn message_for(&self, _param: &str) -> SmallMessageGenerator {
            todo!()
        }
    }

    /// Keeps asking for time slices until end of specified lifetime.
    #[derive(Debug, Default)]
    pub struct TestClockWatcher {
        has_more_work: bool,
        lifetime_seconds: f32,
    }

    impl WatchesClock for TestClockWatcher {
        fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
            self.has_more_work = clock.seconds() < self.lifetime_seconds;
            Vec::new()
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
        overhead: Overhead,
    }

    impl SourcesAudio for TestAudioSource {
        fn source_audio(&mut self, _clock: &Clock) -> crate::common::MonoSample {
            0.
        }
    }
    impl HasOverhead for TestAudioSource {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
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
        overhead: Overhead,
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
    impl HasOverhead for TestAudioSink {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
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
    pub struct TestMidiSource {
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
    }

    impl SourcesMidi for TestMidiSource {
        fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
            &self.channels_to_sink_vecs
        }
        fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
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
            let message = MidiUtils::new_note_on(MidiNote::C4 as u8, 100);
            self.issue_midi(clock, &TestMidiSink::TEST_MIDI_CHANNEL, &message);
        }
    }

    // Gets called with native functions telling it about external keyboard
    // events. Translates those into update messages that influence an
    // arpeggiator, which controls a MIDI instrument.
    //
    // This shows how all these traits work together.
    #[derive(Debug, Default)]
    pub struct TestKeyboard {
        target_uids: Vec<usize>,
        target_messages: Vec<SmallMessageGenerator>,
    }
    impl SourcesUpdates for TestKeyboard {
        fn target_uids(&self) -> &[usize] {
            &self.target_uids
        }

        fn target_uids_mut(&mut self) -> &mut Vec<usize> {
            &mut self.target_uids
        }

        fn target_messages(&self) -> &[SmallMessageGenerator] {
            &self.target_messages
        }

        fn target_messages_mut(&mut self) -> &mut Vec<SmallMessageGenerator> {
            &mut self.target_messages
        }
    }

    impl TestKeyboard {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        pub fn handle_keypress(&mut self, key: u8) -> Vec<BigMessage> {
            match key {
                1 => self.post_message(0.5),
                _ => {
                    vec![]
                }
            }
        }
    }

    #[derive(Display, Debug, EnumString)]
    #[strum(serialize_all = "kebab_case")]
    pub(crate) enum TestArpeggiatorControlParams {
        Tempo,
    }

    #[derive(Debug, Default)]
    pub struct TestArpeggiator {
        me: Ww<Self>,
        midi_channel_out: MidiChannel,
        pub tempo: f32,
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
    }
    impl SourcesMidi for TestArpeggiator {
        fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
            &self.channels_to_sink_vecs
        }
        fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
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
        fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
            // We don't actually pay any attention to self.tempo, but it's easy
            // enough to see that tempo could have influenced this MIDI message.
            self.issue_midi(
                clock,
                &self.midi_channel_out,
                &MidiUtils::new_note_on(60, 100),
            );
            Vec::new()
        }
    }
    impl Terminates for TestArpeggiator {
        fn is_finished(&self) -> bool {
            true
        }
    }
    impl SinksUpdates for TestArpeggiator {
        fn message_for(&self, param: &str) -> SmallMessageGenerator {
            assert_eq!(
                TestArpeggiatorControlParams::Tempo.to_string().as_str(),
                param
            );
            Box::new(SmallMessage::ValueChanged)
        }

        fn update(&mut self, _clock: &Clock, message: SmallMessage) {
            match message {
                SmallMessage::ValueChanged(value) => self.tempo = value,
                _ => todo!(),
            }
        }
    }
    impl TestArpeggiator {
        pub fn new_with(midi_channel_out: MidiChannel) -> Self {
            Self {
                midi_channel_out,
                ..Default::default()
            }
        }
        pub fn new_wrapped_with(midi_channel_out: MidiChannel) -> Rrc<Self> {
            let wrapped = rrc(Self::new_with(midi_channel_out));
            wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
            wrapped
        }
    }

    #[derive(Debug)]
    pub struct TestValueChecker {
        pub values: VecDeque<f32>,
        pub target: Rrc<dyn SourcesAudio>,
        pub checkpoint: f32,
        pub checkpoint_delta: f32,
        pub time_unit: ClockTimeUnit,
    }

    impl WatchesClock for TestValueChecker {
        fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
            if !self.values.is_empty() {
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
            Vec::new()
        }
    }

    impl Terminates for TestValueChecker {
        fn is_finished(&self) -> bool {
            self.values.is_empty()
        }
    }
}
