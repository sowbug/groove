use crate::common::{MidiMessage, MidiMessageType};
use crate::preset::welsh::WelshSynthPreset;
use crate::preset::{EnvelopePreset, LfoPreset, LfoRouting, OscillatorPreset};
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
    pub filter_24db_weight: f32,
    pub filter_12db_weight: f32,
    pub filter_envelope_preset: EnvelopePreset,
    pub filter_envelope_weight: f32,
}

#[derive(Default)]
pub struct SuperVoice {
    osc_1: MiniOscillator,
    osc_2: MiniOscillator,
    osc_1_mix: f32,
    osc_2_mix: f32,
    amp_envelope: MiniEnvelope,

    lfo: MiniOscillator,
    lfo_routing: LfoRouting,
    lfo_depth: f32,

    filter: MiniFilter2,
    filter_weight: f32,
    filter_envelope: MiniEnvelope,
    filter_envelope_weight: f32,
}

impl SuperVoice {
    pub fn new(sample_rate: u32, preset: &WelshSynthPreset) -> Self {
        Self {
            osc_1: MiniOscillator::new_from_preset(&preset.oscillator_1_preset),
            osc_2: MiniOscillator::new_from_preset(&preset.oscillator_2_preset),
            osc_1_mix: preset.oscillator_1_preset.mix,
            osc_2_mix: preset.oscillator_2_preset.mix,
            amp_envelope: MiniEnvelope::new(sample_rate, &preset.amp_envelope_preset),

            lfo: MiniOscillator::new_lfo(&preset.lfo_preset),
            lfo_routing: preset.lfo_preset.routing,
            lfo_depth: preset.lfo_preset.depth,

            filter: MiniFilter2::new(MiniFilter2Type::LowPass(
                sample_rate,
                preset.filter_type_12db.cutoff,
                1.0 / 2.0f32.sqrt(),
            )),
            filter_weight: preset.filter_type_12db.weight,
            filter_envelope: MiniEnvelope::new(sample_rate, &preset.filter_envelope_preset),
            filter_envelope_weight: preset.filter_envelope_weight,

            ..Default::default()
        }
    }

    pub(crate) fn process(&mut self, time_seconds: f32) -> f32 {
        self.amp_envelope.tick(time_seconds);
        self.filter_envelope.tick(time_seconds);

        let lfo = self.lfo.process(time_seconds) * self.lfo_depth;
        if matches!(self.lfo_routing, LfoRouting::Pitch) {
            // Frequency assumes LFO [-1, 1]
            self.osc_1.set_frequency_modulation(lfo);
            self.osc_2.set_frequency_modulation(lfo);
        }

        let osc_1 = self.osc_1.process(time_seconds);
        let osc_2 = self.osc_2.process(time_seconds);
        let mut osc_denom = self.osc_1_mix + self.osc_2_mix;
        if osc_denom == 0.0 {
            osc_denom = 1.0;
        }
        let osc_mix = (osc_1 * self.osc_1_mix + osc_2 * self.osc_2_mix) / osc_denom;

        {
            let filter = self.filter.filter(osc_mix);
            let filter_shaped =
                filter * (1.0 + self.filter_envelope.value() * self.filter_envelope_weight);
            let dry_wet_mix =
                filter_shaped * self.filter_weight + osc_mix * (1.0 - self.filter_weight);

            let lfo_amplitude_modulation = if matches!(self.lfo_routing, LfoRouting::Amplitude) {
                // Amplitude assumes LFO [0, 1]
                lfo / 2.0 + 0.5
            } else {
                1.0
            };
            self.amp_envelope.value() * lfo_amplitude_modulation * dry_wet_mix
        }
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
                self.osc_1.set_frequency(frequency);
                self.osc_2.set_frequency(frequency);
            }
            MidiMessageType::NoteOff => {}
            MidiMessageType::ProgramChange => {}
        }
    }
}
