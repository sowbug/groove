use crate::{
    controllers::{sequencers::MidiTickSequencer, Performance},
    entities::Entity,
    instruments::{drumkit::Drumkit, welsh::WelshSynth},
    midi::programmers::MidiSmfReader,
    settings::{patches::SynthPatch, songs::SongSettings, ClockSettings},
    Orchestrator,
};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample as CpalSample, Stream, StreamConfig, SupportedStreamConfig,
};
use crossbeam::{deque::Steal, queue::ArrayQueue};
use groove_core::{SampleType, StereoSample};
use std::sync::{Arc, Condvar, Mutex};

pub struct AudioOutput {
    sample_rate: usize,
    ring_buffer: Arc<ArrayQueue<StereoSample>>,
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

    pub fn force_push(&mut self, sample: StereoSample) {
        self.ring_buffer.force_push(sample);
    }

    pub fn push(&mut self, sample: StereoSample) -> Result<(), StereoSample> {
        self.ring_buffer.push(sample)
    }

    pub fn push_buffer(&mut self, samples: &[StereoSample]) -> Result<(), StereoSample> {
        for sample in samples {
            if let Err(e) = self.ring_buffer.push(*sample) {
                return Result::Err(e);
            }
        }
        Ok(())
    }

    pub fn start(&mut self) {
        let device = IOHelper::default_output_device();
        let config = IOHelper::default_output_config(&device);
        let sample_format = config.sample_format();
        let channels: usize = config.channels() as usize;
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
                        channels,
                        &ring_buffer_clone,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<i16>(
                        channels,
                        &ring_buffer_clone,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<u16>(
                        channels,
                        &ring_buffer_clone,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I8 => todo!(),
            cpal::SampleFormat::I32 => todo!(),
            cpal::SampleFormat::I64 => todo!(),
            cpal::SampleFormat::U8 => todo!(),
            cpal::SampleFormat::U32 => todo!(),
            cpal::SampleFormat::U64 => todo!(),
            cpal::SampleFormat::F64 => todo!(),
            _ => todo!(),
        } {
            self.stream = Some(result);
            self.play();
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
        channels: usize,
        queue: &Arc<ArrayQueue<StereoSample>>,
        sync_pair: &Arc<(Mutex<bool>, Condvar)>,
        data: &mut [T],
        _info: &cpal::OutputCallbackInfo,
    ) where
        T: CpalSample + FromSample<f32>,
    {
        for frame in data.chunks_mut(channels) {
            let mut sample = StereoSample::default();
            if let Ok(finished) = sync_pair.0.lock() {
                let finished: bool = *finished;
                if !finished {
                    if let Some(popped_sample) = queue.pop() {
                        sample = popped_sample;
                    }
                }
            }
            // I haven't really looked at what a frame is, so although this
            // works, I'm not sure it's right. TODO: spend more than 10 minutes
            // looking at cpal to understand how to use it robustly.
            let left_value = T::from_sample(sample.0 .0 as f32);
            let right_value = T::from_sample(sample.1 .0 as f32);
            frame[0] = left_value;
            if channels > 1 {
                frame[1] = right_value;
            }
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

    pub fn orchestrator_from_midi_file(filename: &str) -> Box<Orchestrator> {
        // TODO: where do BPM, time signature, etc. come from?
        let clock_settings = ClockSettings::default();
        let mut orchestrator = Box::new(Orchestrator::new_with_clock_settings(&clock_settings));

        let data = std::fs::read(filename).unwrap();
        let mut sequencer = Box::new(MidiTickSequencer::new_with(
            clock_settings.sample_rate(),
            clock_settings.midi_ticks_per_second(),
        ));
        MidiSmfReader::program_sequencer(&mut sequencer, &data);
        let sequencer_uid = orchestrator.add(None, Entity::MidiTickSequencer(sequencer));
        orchestrator.connect_midi_upstream(sequencer_uid);

        // TODO: this is a hack. We need only the number of channels used in the
        // SMF, but a few idle ones won't hurt for now.
        for channel in 0..16 {
            let synth_uid = orchestrator.add(
                None,
                if channel == 9 {
                    Entity::Drumkit(Box::new(Drumkit::new_from_files(
                        orchestrator.clock_settings().sample_rate(),
                    )))
                } else {
                    Entity::WelshSynth(Box::new(WelshSynth::new_with(
                        orchestrator.clock_settings().sample_rate(), // TODO: tie this better to actual reality
                        SynthPatch::by_name("Piano"),
                    )))
                },
            );
            orchestrator.connect_midi_downstream(synth_uid, channel);
            let _ = orchestrator.connect_to_main_mixer(synth_uid);
        }
        orchestrator
    }

    /// This utility function assumes the caller is cool with blocking.
    pub fn send_performance_to_output_device(performance: &Performance) -> anyhow::Result<()> {
        let mut audio_output = AudioOutput::default();
        let stealer = performance.worker.stealer();
        audio_output.start();
        while let Steal::Success(sample) = stealer.steal() {
            loop {
                if audio_output.push(sample).is_ok() {
                    break;
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
        Ok(())
    }

    pub fn send_performance_to_file(
        performance: &Performance,
        output_filename: &str,
    ) -> anyhow::Result<()> {
        const AMPLITUDE: SampleType = i16::MAX as SampleType;
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: performance.sample_rate as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(output_filename, spec).unwrap();

        while !performance.worker.is_empty() {
            let sample = performance.worker.pop().unwrap_or_default();
            let _ = writer.write_sample((sample.0 .0 * AMPLITUDE) as i16);
            let _ = writer.write_sample((sample.1 .0 * AMPLITUDE) as i16);
        }
        Ok(())
    }
}
