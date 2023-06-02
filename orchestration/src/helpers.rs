// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [helpers](crate::helpers) module contains structs and methods that make
//! it easier to use the Groove engine.

use crate::{orchestrator::Performance, Orchestrator};
use anyhow::anyhow;
use cpal::{
    traits::{DeviceTrait, HostTrait},
    SupportedStreamConfig,
};
use groove_core::{time::SampleRate, SampleType};
use std::path::PathBuf;

pub struct IOHelper {}
impl IOHelper {
    pub fn default_output_device() -> cpal::Device {
        if let Some(device) = cpal::default_host().default_output_device() {
            device
        } else {
            panic!("Couldn't get default output device")
        }
    }

    pub fn default_output_config(device: &cpal::Device) -> SupportedStreamConfig {
        if let Ok(config) = device.default_output_config() {
            config
        } else {
            panic!("Couldn't get default output config")
        }
    }

    pub fn get_output_device_sample_rate() -> SampleRate {
        SampleRate::new(
            Self::default_output_config(&Self::default_output_device())
                .sample_rate()
                .0 as usize,
        )
    }

    #[allow(unused_variables)]
    pub fn orchestrator_from_midi_file(filename: &str) -> Orchestrator {
        // // TODO: where do BPM, time signature, etc. come from?
        // let mut orchestrator = Orchestrator::new_with(DEFAULT_BPM);

        // let data = std::fs::read(filename).unwrap();
        // let mut sequencer = Box::new(MidiTickSequencer::new_with(
        //     DEFAULT_SAMPLE_RATE,
        //     DEFAULT_MIDI_TICKS_PER_SECOND,
        // ));
        // MidiSmfReader::program_sequencer(&mut sequencer, &data);
        // let sequencer_uid = orchestrator.add(None, Entity::MidiTickSequencer(sequencer));

        // // TODO: this is a hack. We need only the number of channels used in the
        // // SMF, but a few idle ones won't hurt for now.
        // for channel in 0..16 {
        //     let synth_uid = orchestrator.add(
        //         None,
        //         if channel == 9 {
        //             Entity::Drumkit(Box::new(Drumkit::new_from_files(
        //                 orchestrator.sample_rate(),
        //             )))
        //         } else {
        //             Entity::WelshSynth(Box::new(
        //                 WelshPatchSettings::by_name("Piano")
        //                     .into_welsh_synth(orchestrator.sample_rate()),
        //             ))
        //         },
        //     );
        //     orchestrator.connect_midi_downstream(synth_uid, channel);
        //     let _ = orchestrator.connect_to_main_mixer(synth_uid);
        // }
        // orchestrator
        panic!()
    }

    pub fn send_performance_to_file(
        performance: &Performance,
        output_path: &PathBuf,
    ) -> anyhow::Result<()> {
        const AMPLITUDE: SampleType = i16::MAX as SampleType;
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: performance.sample_rate.value() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        if let Some(path) = output_path.to_str() {
            let mut writer = hound::WavWriter::create(path, spec).unwrap();

            while !performance.worker.is_empty() {
                let sample = performance.worker.pop().unwrap_or_default();
                let _ = writer.write_sample((sample.0 .0 * AMPLITUDE) as i16);
                let _ = writer.write_sample((sample.1 .0 * AMPLITUDE) as i16);
            }
            Ok(())
        } else {
            Err(anyhow!("Couldn't create path from {:?}", output_path))
        }
    }
}
