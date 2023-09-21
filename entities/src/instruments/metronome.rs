// Copyright (c) 2023 Mike Tsao. All rights reserved.

use ensnare::prelude::*;
use groove_core::{
    generators::{Oscillator, OscillatorParams, Waveform},
    midi::HandlesMidi,
    traits::{Configurable, Generates, Serializable, Ticks},
};
use groove_proc_macros::{Control, IsInstrument, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[cfg(obsolete)]
mod obsolete {

    #[derive(Debug, Control, IsInstrument, Params, Uid)]
    #[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
    pub struct Metronome {
        #[control]
        #[params]
        #[allow(dead_code)]
        bpm: ParameterType,

        clock: MusicalTime,

        uid: Uid,
        oscillator: Oscillator,

        is_playing: bool,
        when_to_stop_playing: f64,
        current_measure: usize,
        current_beat: usize,
    }
    impl Generates<StereoSample> for Metronome {
        fn value(&self) -> StereoSample {
            if self.is_playing {
                self.oscillator.value().into()
            } else {
                StereoSample::SILENCE
            }
        }

        fn generate_batch_values(&mut self, _values: &mut [StereoSample]) {
            todo!("write a way to batch BipolarNormal to StereoSample")
        }
    }
    impl Serializable for Metronome {}
    impl Configurable for Metronome {
        fn update_sample_rate(&mut self, sample_rate: SampleRate) {
            self.clock.update_sample_rate(sample_rate);
            self.oscillator.update_sample_rate(sample_rate);
        }
    }
    impl Ticks for Metronome {
        fn tick(&mut self, tick_count: usize) {
            if self.current_beat != self.clock.beats() as usize {
                self.current_beat = self.clock.beats() as usize;
                self.is_playing = true;
                self.oscillator.set_frequency(440.0.into());
                if self.current_measure != self.clock.measures() {
                    self.current_measure = self.clock.measures();
                    self.oscillator.set_frequency(880.0.into());
                }
                self.when_to_stop_playing = self.clock.seconds() + 0.01;
            }
            if self.is_playing && self.clock.seconds() >= self.when_to_stop_playing {
                self.is_playing = false;
            }
            self.clock.tick(tick_count);
            self.oscillator.tick(tick_count);
        }
    }
    impl HandlesMidi for Metronome {}
    impl Metronome {
        pub fn new_with(params: &MetronomeParams) -> Self {
            let mut oscillator_params = OscillatorParams::default();
            oscillator_params.waveform = Waveform::Square;
            let mut clock_params = ClockParams::default();
            clock_params.set_bpm(params.bpm());
            Self {
                bpm: params.bpm(),
                clock: Clock::new_with(&clock_params),
                uid: Default::default(),
                oscillator: Oscillator::new_with(&oscillator_params),
                is_playing: false,
                when_to_stop_playing: Default::default(),
                current_measure: usize::MAX,
                current_beat: usize::MAX,
            }
        }

        pub fn clock(&self) -> &Clock {
            &self.clock
        }

        pub fn is_playing(&self) -> bool {
            self.is_playing
        }

        pub fn bpm(&self) -> f64 {
            self.clock.bpm()
        }

        pub fn set_bpm(&mut self, bpm: ParameterType) {
            self.clock.set_bpm(bpm);
        }
    }

    #[cfg(feature = "egui-framework")]
    mod gui {
        use super::Metronome;
        use eframe::egui::Ui;
        use groove_core::traits::gui::Displays;

        impl Displays for Metronome {
            fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
                ui.label(format!("BPM: {:0.1}", self.bpm()));
                ui.label(format!(
                    "Time Signature: {}/{}",
                    self.clock().time_signature().top,
                    self.clock().time_signature().bottom
                ));
                ui.label(if self.is_playing() { "X" } else { " " })
            }
        }
    }
}
