use crate::common::{MidiMessage, MidiMessageType};
use crate::preset::welsh::WelshSynthPreset;
use crate::preset::{EnvelopePreset, FilterPreset, LfoPreset, LfoRouting, OscillatorPreset};
use crate::primitives::clock::Clock;
use crate::primitives::envelopes::MiniEnvelope;
use crate::primitives::filter::{MiniFilter2, MiniFilter2Type};
use crate::primitives::oscillators::{MiniOscillator, Waveform};

use super::traits::DeviceTrait;

#[derive(Default, Debug, Clone, Copy)]
pub struct SuperSynthPreset {
    pub oscillator_1_preset: OscillatorPreset,
    pub oscillator_2_preset: OscillatorPreset,
    // TODO: osc 2 track/sync
    pub amp_envelope_preset: EnvelopePreset,

    pub lfo_preset: LfoPreset,

    // TODO: glide, time, unison, voices

    // There is meant to be only one filter, but the Welsh book
    // provides alternate settings depending on the kind of filter
    // your synthesizer has.
    pub filter_24db_type: MiniFilter2Type,
    pub filter_12db_type: MiniFilter2Type,
    pub filter_envelope_preset: EnvelopePreset,
    pub filter_envelope_weight: f32,
}

#[derive(Default)]
pub struct SuperVoice {
    oscillators: Vec<MiniOscillator>,
    osc_mix: Vec<f32>,
    amp_envelope: MiniEnvelope,

    lfo: MiniOscillator,
    lfo_routing: LfoRouting,
    lfo_depth: f32,

    filter: MiniFilter2,
    filter_cutoff_start: f32,
    filter_cutoff_end: f32,
    filter_envelope: MiniEnvelope,
}

impl SuperVoice {
    pub fn new(sample_rate: u32, preset: &WelshSynthPreset) -> Self {
        let mut r = Self {
            oscillators: Vec::new(),
            osc_mix: Vec::new(),
            amp_envelope: MiniEnvelope::new(sample_rate, &preset.amp_envelope_preset),

            lfo: MiniOscillator::new_lfo(&preset.lfo_preset),
            lfo_routing: preset.lfo_preset.routing,
            lfo_depth: preset.lfo_preset.depth,

            filter: MiniFilter2::new(MiniFilter2Type::LowPass(
                sample_rate,
                preset.filter_type_12db.cutoff,
                1.0 / 2.0f32.sqrt(), // TODO: resonance
            )),
            filter_cutoff_start: MiniFilter2::frequency_to_percent(preset.filter_type_12db.cutoff),
            filter_cutoff_end: preset.filter_envelope_weight,
            filter_envelope: MiniEnvelope::new(sample_rate, &preset.filter_envelope_preset),
        };
        if !matches!(preset.oscillator_1_preset.waveform, Waveform::None) {
            r.oscillators
                .push(MiniOscillator::new_from_preset(&preset.oscillator_1_preset));
            r.osc_mix.push(preset.oscillator_1_preset.mix);
        }
        if !matches!(preset.oscillator_2_preset.waveform, Waveform::None) {
            r.oscillators
                .push(MiniOscillator::new_from_preset(&preset.oscillator_2_preset));
            r.osc_mix.push(preset.oscillator_2_preset.mix);
        }
        if preset.noise > 0.0 {
            r.oscillators.push(MiniOscillator::new(Waveform::Noise));
            r.osc_mix.push(preset.noise);
        }
        r
    }

    pub(crate) fn process(&mut self, time_seconds: f32) -> f32 {
        // TODO: divide by 10,000 until we figure out how pitch depth is supposed to go
        let lfo = self.lfo.process(time_seconds) * self.lfo_depth / 10000.0;
        if matches!(self.lfo_routing, LfoRouting::Pitch) {
            // TODO: this could leave a side effect if we reuse voices and forget to clean up.
            for o in self.oscillators.iter_mut() {
                o.set_frequency_modulation(lfo);
            }
        }

        let mut osc_sum = 0.0f32;
        if self.oscillators.len() > 0 {
            for o in self.oscillators.iter_mut() {
                osc_sum += o.process(time_seconds);
            }
            osc_sum /= self.oscillators.len() as f32;
        }
        self.filter_envelope.tick(time_seconds);
        let new_cutoff_percentage = (self.filter_cutoff_start
            + (self.filter_cutoff_end - self.filter_cutoff_start) * self.filter_envelope.value());
        let new_cutoff = MiniFilter2::percent_to_frequency(new_cutoff_percentage);
        self.filter.set_cutoff(new_cutoff);
        let filtered_mix = self.filter.filter(osc_sum);

        let lfo_amplitude_modulation = if matches!(self.lfo_routing, LfoRouting::Amplitude) {
            // LFO ranges from [-1, 1], so convert to something that can silence or double the volume.
            lfo + 1.0
        } else {
            1.0
        };
        self.amp_envelope.tick(time_seconds);
        self.amp_envelope.value() * lfo_amplitude_modulation * filtered_mix
    }

    pub(crate) fn is_playing(&self) -> bool {
        !self.amp_envelope.is_idle()
    }
}

impl DeviceTrait for SuperVoice {
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        self.amp_envelope
            .handle_midi_message(message, clock.seconds);
        self.filter_envelope
            .handle_midi_message(message, clock.seconds);
        match message.status {
            MidiMessageType::NoteOn => {
                let frequency = message.to_frequency();
                for o in self.oscillators.iter_mut() {
                    if !matches!(o.waveform, Waveform::Noise) {
                        o.set_frequency(frequency);
                    }
                }
            }
            MidiMessageType::NoteOff => {}
            MidiMessageType::ProgramChange => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::MidiMessage,
        devices::traits::DeviceTrait,
        preset::{
            welsh::{GlidePreset, PolyphonyPreset, WelshSynthPreset},
            EnvelopePreset, FilterPreset, LfoPreset, LfoRouting, OscillatorPreset,
        },
        primitives::{clock::Clock, oscillators::Waveform},
    };

    use super::SuperVoice;

    const SAMPLE_RATE: u32 = 44100;

    fn write_sound(
        source: &mut SuperVoice,
        clock: &mut Clock,
        duration: f32,
        message: &MidiMessage,
        when: f32,
        filename: &str,
    ) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: f32 = i16::MAX as f32;
        let mut writer = hound::WavWriter::create(filename, spec).unwrap();

        let mut is_message_sent = false;
        while clock.seconds < duration {
            if (when <= clock.seconds && !is_message_sent) {
                is_message_sent = true;
                source.handle_midi_message(message, clock);
            }
            let sample = source.process(clock.seconds);
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    fn angels_patch() -> WelshSynthPreset {
        WelshSynthPreset {
            oscillator_1_preset: OscillatorPreset {
                waveform: Waveform::Sawtooth,
                ..Default::default()
            },
            oscillator_2_preset: OscillatorPreset {
                ..Default::default()
            },
            oscillator_2_track: false,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo_preset: LfoPreset {
                routing: LfoRouting::Pitch,
                waveform: Waveform::Triangle,
                frequency: 2.4,
                depth: LfoPreset::semis_and_cents(0.0, 20.0),
            },
            glide: GlidePreset::Off,
            has_unison: false,
            polyphony: PolyphonyPreset::Multi,
            filter_type_24db: FilterPreset {
                cutoff: 900.0,
                weight: 0.55,
            },
            filter_type_12db: FilterPreset {
                cutoff: 900.0,
                weight: 0.55,
            },
            filter_resonance: 0.7,
            filter_envelope_weight: 0.0,
            filter_envelope_preset: EnvelopePreset {
                attack_seconds: 0.0,
                decay_seconds: 0.0,
                sustain_percentage: 0.0,
                release_seconds: 0.0,
            },
            amp_envelope_preset: EnvelopePreset {
                attack_seconds: 0.32,
                decay_seconds: EnvelopePreset::MAX,
                sustain_percentage: 1.0,
                release_seconds: 0.93,
            },
        }
    }

    fn cello_patch() -> WelshSynthPreset {
        WelshSynthPreset {
            oscillator_1_preset: OscillatorPreset {
                waveform: Waveform::PulseWidth(0.1),
                ..Default::default()
            },
            oscillator_2_preset: OscillatorPreset {
                waveform: Waveform::Square,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo_preset: LfoPreset {
                routing: LfoRouting::Amplitude,
                waveform: Waveform::Sine,
                frequency: 7.5,
                depth: LfoPreset::percent(5.0),
            },
            glide: GlidePreset::Off,
            has_unison: false,
            polyphony: PolyphonyPreset::Multi,
            filter_type_24db: FilterPreset {
                cutoff: 40.0,
                weight: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff: 40.0,
                weight: 0.1,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 0.9,
            filter_envelope_preset: EnvelopePreset {
                attack_seconds: 0.0,
                decay_seconds: 3.29,
                sustain_percentage: 0.78,
                release_seconds: EnvelopePreset::MAX,
            },
            amp_envelope_preset: EnvelopePreset {
                attack_seconds: 0.06,
                decay_seconds: EnvelopePreset::MAX,
                sustain_percentage: 1.0,
                release_seconds: 0.3,
            },
        }
    }

    fn test_patch() -> WelshSynthPreset {
        WelshSynthPreset {
            oscillator_1_preset: OscillatorPreset {
                waveform: Waveform::Sawtooth,
                ..Default::default()
            },
            oscillator_2_preset: OscillatorPreset {
                waveform: Waveform::None,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo_preset: LfoPreset {
                routing: LfoRouting::None,
                ..Default::default()
            },
            glide: GlidePreset::Off,
            has_unison: false,
            polyphony: PolyphonyPreset::Multi,
            filter_type_24db: FilterPreset {
                cutoff: 40.0,
                weight: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff: 20.0,
                weight: 0.1,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 1.0,
            filter_envelope_preset: EnvelopePreset {
                attack_seconds: 5.0,
                decay_seconds: EnvelopePreset::MAX,
                sustain_percentage: 1.0,
                release_seconds: EnvelopePreset::MAX,
            },
            amp_envelope_preset: EnvelopePreset {
                attack_seconds: 0.5,
                decay_seconds: EnvelopePreset::MAX,
                sustain_percentage: 1.0,
                release_seconds: EnvelopePreset::MAX,
            },
        }
    }

    #[test]
    fn test_basic_synth_patch() {
        let message_on = MidiMessage {
            status: crate::common::MidiMessageType::NoteOn,
            channel: 0,
            data1: 60,
            data2: 0,
        };

        let message_off = MidiMessage {
            status: crate::common::MidiMessageType::NoteOff,
            channel: 0,
            data1: 60,
            data2: 0,
        };

        let mut clock = Clock::new(SAMPLE_RATE, 4, 4, 128.);
        let mut voice = SuperVoice::new(SAMPLE_RATE, &test_patch());
        voice.handle_midi_message(&message_on, &clock);
        write_sound(
            &mut voice,
            &mut clock,
            5.0,
            &message_off,
            5.0,
            "voice_test_c3.wav",
        );
    }

    #[test]
    fn test_basic_cello_patch() {
        let message_on = MidiMessage {
            status: crate::common::MidiMessageType::NoteOn,
            channel: 0,
            data1: 60,
            data2: 0,
        };

        let message_off = MidiMessage {
            status: crate::common::MidiMessageType::NoteOff,
            channel: 0,
            data1: 60,
            data2: 0,
        };

        let mut clock = Clock::new(SAMPLE_RATE, 4, 4, 128.);
        let mut voice = SuperVoice::new(SAMPLE_RATE, &cello_patch());
        voice.handle_midi_message(&message_on, &clock);
        write_sound(
            &mut voice,
            &mut clock,
            5.0,
            &message_off,
            1.0,
            "voice_cello_c3.wav",
        );
    }
}
