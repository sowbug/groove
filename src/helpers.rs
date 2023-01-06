use crate::{
    common::{MonoSample, MONO_SAMPLE_SILENCE},
    controllers::{sequencers::MidiTickSequencer, Performance},
    entities::BoxedEntity,
    instruments::{drumkit_sampler::DrumkitSampler, welsh::WelshSynth},
    midi::programmers::MidiSmfReader,
    settings::{patches::SynthPatch, songs::SongSettings, ClockSettings},
    GrooveOrchestrator, Orchestrator,
};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Stream, StreamConfig, SupportedStreamConfig,
};
use crossbeam::{
    deque::{Steal, Stealer},
    queue::ArrayQueue,
};
use std::sync::{Arc, Condvar, Mutex};

pub struct AudioOutput {
    sample_rate: usize,
    ring_buffer: Arc<ArrayQueue<f32>>,
    stream: Option<Stream>,
    sync_pair: Arc<(Mutex<bool>, Condvar)>,
}

impl Default for AudioOutput {
    fn default() -> Self {
        Self {
            sample_rate: 0,
            ring_buffer: Arc::new(ArrayQueue::new(4096)),
            stream: None,
            sync_pair: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }
}

// TODO: make this smart and not start playing audio until it has enough samples
// buffered up.
impl AudioOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn buffer_len(&self) -> usize {
        self.ring_buffer.len()
    }

    pub fn buffer_capacity(&self) -> usize {
        self.ring_buffer.capacity()
    }

    pub fn push(&mut self, sample: MonoSample) {
        self.ring_buffer.force_push(sample);
    }

    pub fn start(&mut self) {
        let device = IOHelper::default_output_device();
        let config = IOHelper::default_output_config(&device);
        let sample_format = config.sample_format();
        let config: StreamConfig = config.into();
        self.sample_rate = config.sample_rate.0 as usize;

        let sync_pair_clone = Arc::clone(&self.sync_pair);
        let ring_buffer_clone = Arc::clone(&self.ring_buffer);

        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {err}");
        if let Ok(result) = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<f32>(
                        &ring_buffer_clone,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<i16>(
                        &ring_buffer_clone,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<u16>(
                        &ring_buffer_clone,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
        } {
            self.stream = Some(result);
            if let Some(stream) = &self.stream {
                if stream.play().is_ok() {
                    // hooray
                }
            }
        }
    }

    /// End the audio output and this thread.
    pub fn stop(&mut self) {
        if let Some(stream) = &self.stream {
            if stream.pause().is_ok() {
                // hooray again
            }
        }
        let lock = &self.sync_pair.0;
        let cvar = &self.sync_pair.1;
        let mut finished = lock.lock().unwrap();
        *finished = true;
        cvar.notify_one();
    }

    /// Ask the audio output to stop handling the stream.
    pub fn pause(&mut self) {
        if let Some(stream) = &self.stream {
            let _ = stream.pause();
        }
    }

    /// Ask the audio output to start handling the stream.
    pub fn play(&mut self) {
        if let Some(stream) = &self.stream {
            let _ = stream.play();
        }
    }

    fn sample_from_queue<T: cpal::Sample>(
        queue: &Arc<ArrayQueue<MonoSample>>,
        sync_pair: &Arc<(Mutex<bool>, Condvar)>,
        data: &mut [T],
        _info: &cpal::OutputCallbackInfo,
    ) {
        for next_sample in data.iter_mut() {
            let mut sample = 0.0f32;
            if let Ok(finished) = sync_pair.0.lock() {
                let finished: bool = *finished;
                if !finished {
                    if let Some(popped_sample) = queue.pop() {
                        sample = popped_sample;
                    }
                }
            }
            *next_sample = cpal::Sample::from(&sample);
        }
    }

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }
}

pub struct IOHelper {}

impl IOHelper {
    fn default_output_device() -> cpal::Device {
        if let Some(device) = cpal::default_host().default_output_device() {
            device
        } else {
            panic!("Couldn't get default output device")
        }
    }

    fn default_output_config(device: &cpal::Device) -> SupportedStreamConfig {
        if let Ok(config) = device.default_output_config() {
            config
        } else {
            panic!("Couldn't get default output config")
        }
    }

    pub fn get_output_device_sample_rate() -> usize {
        Self::default_output_config(&Self::default_output_device())
            .sample_rate()
            .0 as usize
    }

    pub fn song_settings_from_yaml_file(filename: &str) -> anyhow::Result<SongSettings> {
        let yaml = std::fs::read_to_string(filename)?;
        let settings = SongSettings::new_from_yaml(yaml.as_str())?;
        Ok(settings)
    }

    pub fn orchestrator_from_midi_file(filename: &str) -> Box<GrooveOrchestrator> {
        let data = std::fs::read(filename).unwrap();
        let mut orchestrator = Box::new(Orchestrator::default());

        let mut sequencer = Box::new(MidiTickSequencer::default());
        MidiSmfReader::program_sequencer(&mut sequencer, &data);
        let sequencer_uid = orchestrator.add(None, BoxedEntity::MidiTickSequencer(sequencer));
        orchestrator.connect_midi_upstream(sequencer_uid);

        // TODO: this is a hack. We need only the number of channels used in the
        // SMF, but a few idle ones won't hurt for now.
        for channel in 0..16 {
            let synth_uid = orchestrator.add(
                None,
                if channel == 9 {
                    BoxedEntity::DrumkitSampler(Box::new(DrumkitSampler::new_from_files()))
                } else {
                    BoxedEntity::WelshSynth(Box::new(WelshSynth::new_with(
                        ClockSettings::default().sample_rate(), // TODO: tie this better to actual reality
                        SynthPatch::by_name("Piano"),
                    )))
                },
            );
            orchestrator.connect_midi_downstream(synth_uid, channel);
            let _ = orchestrator.connect_to_main_mixer(synth_uid);
        }
        orchestrator
    }

    pub fn sample_from_queue<T: cpal::Sample>(
        audio_is_stereo: bool,
        stealer: &Stealer<MonoSample>,
        sync_pair: &Arc<(Mutex<bool>, Condvar)>,
        data: &mut [T],
        _info: &cpal::OutputCallbackInfo,
    ) {
        let lock = &sync_pair.0;
        let cvar = &sync_pair.1;
        let mut finished = lock.lock().unwrap();

        let mut hack_duplicate_mono_for_stereo = false;
        let mut last_sample = MONO_SAMPLE_SILENCE;
        for next_sample in data.iter_mut() {
            let sample = if audio_is_stereo && hack_duplicate_mono_for_stereo {
                last_sample
            } else if let Steal::Success(sample) = stealer.steal() {
                last_sample = sample;
                sample
            } else {
                // TODO(miket): this isn't great, because we don't know whether
                // the steal failure was because of a spurious error (buffer
                // underrun) or complete processing.
                *finished = true;
                cvar.notify_one();
                MONO_SAMPLE_SILENCE
            };
            hack_duplicate_mono_for_stereo = !hack_duplicate_mono_for_stereo;

            // This is where MonoSample becomes an f32.
            #[allow(clippy::unnecessary_cast)]
            let sample_crossover: f32 = sample as f32;
            *next_sample = cpal::Sample::from(&sample_crossover);
        }
    }

    pub fn send_performance_to_output_device(performance: &Performance) -> anyhow::Result<()> {
        let device = Self::default_output_device();
        let config: SupportedStreamConfig = Self::default_output_config(&device);
        let audio_is_stereo = config.channels() == 2;
        let sample_format = config.sample_format();
        let config: StreamConfig = config.into();

        let stealer = performance.worker.stealer();

        let sync_pair = Arc::new((Mutex::new(false), Condvar::new()));
        let sync_pair_clone = Arc::clone(&sync_pair);
        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {err}");
        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<f32>(
                        audio_is_stereo,
                        &stealer,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<i16>(
                        audio_is_stereo,
                        &stealer,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<u16>(
                        audio_is_stereo,
                        &stealer,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
        }
        .unwrap();

        stream.play()?;

        // See https://doc.rust-lang.org/stable/std/sync/struct.Condvar.html for
        // origin of this code.
        let &(ref lock, ref cvar) = &*sync_pair;
        let mut finished = lock.lock().unwrap();
        while !*finished {
            finished = cvar.wait(finished).unwrap();
        }
        Ok(())
    }

    pub fn send_performance_to_file(
        performance: Performance,
        output_filename: &str,
    ) -> anyhow::Result<()> {
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: performance.sample_rate as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(output_filename, spec).unwrap();

        while !performance.worker.is_empty() {
            let sample = performance.worker.pop().unwrap_or_default();
            writer.write_sample((sample * AMPLITUDE) as i16).unwrap();
        }
        Ok(())
    }
}
