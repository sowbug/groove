use crate::common::{rrc, MonoSample};
use crate::midi::sequencer::MidiSequencer;
use crate::midi::smf_reader::MidiSmfReader;
use crate::orchestrator::{Orchestrator, Performance};
use crate::settings::song::SongSettings;
use crate::settings::ClockSettings;
use crate::synthesizers::drumkit_sampler::Sampler;
use crate::synthesizers::welsh::{PresetName, Synth, SynthPreset};
use crate::traits::IsMidiInstrument;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleRate, StreamConfig};
use crossbeam::deque::Stealer;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};

pub struct IOHelper {}

impl IOHelper {
    pub fn song_settings_from_yaml_file(filename: &str) -> SongSettings {
        if let Ok(yaml) = std::fs::read_to_string(filename) {
            if let Ok(settings) = SongSettings::new_from_yaml(yaml.as_str()) {
                settings
            } else {
                panic!("do something better here");
            }
        } else {
            panic!("couldn't read YAML file")
        }
    }

    pub fn orchestrator_from_midi_file(filename: &str) -> Orchestrator {
        let data = std::fs::read(filename).unwrap();
        let mut orchestrator = Orchestrator::new();

        let midi_sequencer = rrc(MidiSequencer::new());
        MidiSmfReader::load_sequencer(&data, &mut midi_sequencer.borrow_mut());

        let instrument = Rc::clone(&midi_sequencer);
        orchestrator.connect_to_upstream_midi_bus(instrument);
        orchestrator.register_clock_watcher(None, midi_sequencer);

        for channel in 0..MidiSequencer::connected_channel_count() {
            let synth: Rc<RefCell<dyn IsMidiInstrument>> = if channel == 9 {
                Rc::new(RefCell::new(Sampler::new_from_files(channel)))
            } else {
                Rc::new(RefCell::new(Synth::new(
                    channel,
                    ClockSettings::default().sample_rate(), // TODO: tie this better to actual reality
                    SynthPreset::by_name(&PresetName::Piano),
                )))
            };
            let instrument = Rc::clone(&synth);
            orchestrator.register_audio_source(None, instrument);
            let sink = Rc::downgrade(&synth);
            orchestrator.connect_to_downstream_midi_bus(channel, sink);
            let device = Rc::downgrade(&synth);
            orchestrator.add_main_mixer_source(device);
        }
        orchestrator
    }

    pub fn get_sample_from_queue<T: cpal::Sample>(
        stealer: &Stealer<MonoSample>,
        sync_pair: &Arc<(Mutex<bool>, Condvar)>,
        data: &mut [T],
        _info: &cpal::OutputCallbackInfo,
    ) {
        let lock = &sync_pair.0;
        let cvar = &sync_pair.1;
        let mut finished = lock.lock().unwrap();

        for next_sample in data.iter_mut() {
            let sample_option = stealer.steal();
            let sample: MonoSample = if sample_option.is_success() {
                sample_option.success().unwrap_or_default()
            } else {
                // TODO(miket): this isn't great, because we don't know whether
                // the steal failure was because of a spurious error (buffer underrun)
                // or complete processing.
                *finished = true;
                cvar.notify_one();
                0.
            };
            // This is where MonoSample becomes an f32.
            #[allow(clippy::unnecessary_cast)]
            let sample_crossover: f32 = sample as f32;
            *next_sample = cpal::Sample::from(&sample_crossover);
        }
    }

    pub fn send_performance_to_output_device(performance: Performance) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");

        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("no supported config?!")
            .with_sample_rate(SampleRate(performance.sample_rate as u32));

        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        let stealer = performance.worker.stealer();

        let sync_pair = Arc::new((Mutex::new(false), Condvar::new()));
        let sync_pair_clone = Arc::clone(&sync_pair);
        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::get_sample_from_queue::<f32>(
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
                    Self::get_sample_from_queue::<i16>(
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
                    Self::get_sample_from_queue::<u16>(
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

        // See https://doc.rust-lang.org/stable/std/sync/struct.Condvar.html for origin of this
        // code.
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
