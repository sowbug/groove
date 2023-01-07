use super::{envelopes::AdsrEnvelope, oscillators::Oscillator};
use crate::{
    common::{F32ControlValue, MonoSample},
    effects::filter::{BiQuadFilter, FilterParams},
    messages::EntityMessage,
    midi::{GeneralMidiProgram, MidiMessage, MidiUtils},
    settings::{
        patches::{LfoRouting, SynthPatch, WaveformType},
        LoadError,
    },
    traits::{
        Controllable, HasUid, IsInstrument, Response, SourcesAudio, TransformsAudio, Updateable,
    },
    utils::Paths,
    Clock,
};
use convert_case::{Boundary, Case, Casing};
use groove_macros::{Control, Uid};
use num_traits::FromPrimitive;
use rustc_hash::FxHashMap;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

// TODO: cache these as they're loaded
impl SynthPatch {
    pub fn patch_name_to_settings_name(name: &str) -> String {
        name.from_case(Case::Camel)
            .without_boundaries(&[Boundary::DigitLower])
            .to_case(Case::Kebab)
    }

    pub fn new_from_yaml(yaml: &str) -> Result<Self, LoadError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            println!("{e}");
            LoadError::FormatError
        })
    }

    pub fn by_name(name: &str) -> Self {
        let mut filename = Paths::asset_path();
        filename.push("patches");
        filename.push("welsh");
        filename.push(format!(
            "{}.yaml",
            Self::patch_name_to_settings_name(name.to_string().as_str())
        ));
        if let Ok(contents) = std::fs::read_to_string(&filename) {
            match Self::new_from_yaml(&contents) {
                Ok(patch) => patch,
                Err(err) => {
                    // TODO: this should return a failsafe patch, maybe a boring
                    // square wave
                    panic!("couldn't parse patch file: {err:?}");
                }
            }
        } else {
            panic!("couldn't read patch file named {:?}", &filename);
        }
    }
}

impl WelshSynth {
    pub fn general_midi_preset(program: &GeneralMidiProgram) -> anyhow::Result<SynthPatch> {
        let mut delegated = false;
        let preset = match program {
            GeneralMidiProgram::AcousticGrand => "Piano",
            GeneralMidiProgram::BrightAcoustic => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::ElectricGrand => "ElectricPiano",
            GeneralMidiProgram::HonkyTonk => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::ElectricPiano1 => "ElectricPiano",
            GeneralMidiProgram::ElectricPiano2 => "ElectricPiano",
            GeneralMidiProgram::Harpsichord => "Harpsichord",
            GeneralMidiProgram::Clav => "Clavichord",
            GeneralMidiProgram::Celesta => "Celeste",
            GeneralMidiProgram::Glockenspiel => "Glockenspiel",
            GeneralMidiProgram::MusicBox => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Vibraphone => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Marimba => "Marimba",
            GeneralMidiProgram::Xylophone => "Xylophone",
            GeneralMidiProgram::TubularBells => "Bell",
            GeneralMidiProgram::Dulcimer => "Dulcimer",
            GeneralMidiProgram::DrawbarOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::PercussiveOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::RockOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::ChurchOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::ReedOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::Accordion => "Accordion",
            GeneralMidiProgram::Harmonica => "Harmonica",
            GeneralMidiProgram::TangoAccordion => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::AcousticGuitarNylon => "GuitarAcoustic",
            GeneralMidiProgram::AcousticGuitarSteel => {
                "GuitarAcoustic" // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarJazz => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarClean => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarMuted => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::OverdrivenGuitar => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::DistortionGuitar => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::GuitarHarmonics => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::AcousticBass => "DoubleBass",
            GeneralMidiProgram::ElectricBassFinger => "StandupBass",
            GeneralMidiProgram::ElectricBassPick => "AcidBass",
            GeneralMidiProgram::FretlessBass => {
                "DetroitBass" // TODO same?
            }
            GeneralMidiProgram::SlapBass1 => "FunkBass",
            GeneralMidiProgram::SlapBass2 => "FunkBass",
            GeneralMidiProgram::SynthBass1 => "DigitalBass",
            GeneralMidiProgram::SynthBass2 => "DigitalBass",
            GeneralMidiProgram::Violin => "Violin",
            GeneralMidiProgram::Viola => "Viola",
            GeneralMidiProgram::Cello => "Cello",
            GeneralMidiProgram::Contrabass => "Contrabassoon",
            GeneralMidiProgram::TremoloStrings => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::PizzicatoStrings => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::OrchestralHarp => "Harp",
            GeneralMidiProgram::Timpani => "Timpani",
            GeneralMidiProgram::StringEnsemble1 => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::StringEnsemble2 => {
                "StringsPwm" // TODO same?
            }
            GeneralMidiProgram::Synthstrings1 => "StringsPwm", // TODO same?

            GeneralMidiProgram::Synthstrings2 => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::ChoirAahs => "Angels",

            GeneralMidiProgram::VoiceOohs => "Choir",
            GeneralMidiProgram::SynthVoice => "VocalFemale",

            GeneralMidiProgram::OrchestraHit => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Trumpet => "Trumpet",
            GeneralMidiProgram::Trombone => "Trombone",
            GeneralMidiProgram::Tuba => "Tuba",
            GeneralMidiProgram::MutedTrumpet => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::FrenchHorn => "FrenchHorn",

            GeneralMidiProgram::BrassSection => "BrassSection",

            GeneralMidiProgram::Synthbrass1 => {
                "BrassSection" // TODO dup
            }
            GeneralMidiProgram::Synthbrass2 => {
                "BrassSection" // TODO dup
            }
            GeneralMidiProgram::SopranoSax => {
                "Saxophone" // TODO dup
            }
            GeneralMidiProgram::AltoSax => "Saxophone",
            GeneralMidiProgram::TenorSax => {
                "Saxophone" // TODO dup
            }
            GeneralMidiProgram::BaritoneSax => {
                "Saxophone" // TODO dup
            }
            GeneralMidiProgram::Oboe => "Oboe",
            GeneralMidiProgram::EnglishHorn => "EnglishHorn",
            GeneralMidiProgram::Bassoon => "Bassoon",
            GeneralMidiProgram::Clarinet => "Clarinet",
            GeneralMidiProgram::Piccolo => "Piccolo",
            GeneralMidiProgram::Flute => "Flute",
            GeneralMidiProgram::Recorder => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::PanFlute => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::BlownBottle => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Shakuhachi => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Whistle => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Ocarina => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead1Square => {
                "MonoSolo" // TODO: same?
            }
            GeneralMidiProgram::Lead2Sawtooth => {
                "Trance5th" // TODO: same?
            }
            GeneralMidiProgram::Lead3Calliope => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead4Chiff => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead5Charang => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead6Voice => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead7Fifths => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead8BassLead => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad1NewAge => {
                "NewAgeLead" // TODO pad or lead?
            }
            GeneralMidiProgram::Pad2Warm => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad3Polysynth => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad4Choir => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad5Bowed => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad6Metallic => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad7Halo => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad8Sweep => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx1Rain => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx2Soundtrack => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx3Crystal => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx4Atmosphere => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx5Brightness => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx6Goblins => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx7Echoes => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx8SciFi => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Sitar => "Sitar",
            GeneralMidiProgram::Banjo => "Banjo",
            GeneralMidiProgram::Shamisen => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Koto => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Kalimba => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Bagpipe => "Bagpipes",
            GeneralMidiProgram::Fiddle => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Shanai => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::TinkleBell => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Agogo => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::SteelDrums => {
                "WheelsOfSteel" // TODO same?
            }
            GeneralMidiProgram::Woodblock => "SideStick",
            GeneralMidiProgram::TaikoDrum => {
                // XXXXXXXXXXXXX TMP
                "Cello" // TODO substitute.....
            }
            GeneralMidiProgram::MelodicTom => "Bongos",
            GeneralMidiProgram::SynthDrum => "SnareDrum",
            GeneralMidiProgram::ReverseCymbal => "Cymbal",
            GeneralMidiProgram::GuitarFretNoise => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::BreathNoise => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Seashore => "OceanWavesWithFoghorn",
            GeneralMidiProgram::BirdTweet => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::TelephoneRing => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Helicopter => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Applause => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Gunshot => {
                delegated = true;
                "Piano"
            }
        };
        if delegated {
            eprintln!("Delegated {program} to {preset}");
        }
        Ok(SynthPatch::by_name(preset))
    }
}

#[derive(Clone, Debug, Default)]
pub struct WelshVoice {
    oscillators: Vec<Oscillator>,
    oscillator_2_sync: bool,
    amp_envelope: AdsrEnvelope,

    lfo: Oscillator,
    lfo_routing: LfoRouting,
    lfo_depth: f32,

    filter: BiQuadFilter<EntityMessage>,
    filter_cutoff_start: f32,
    filter_cutoff_end: f32,
    filter_envelope: AdsrEnvelope,
}

impl WelshVoice {
    pub fn new_with(sample_rate: usize, preset: &SynthPatch) -> Self {
        let mut r = Self {
            amp_envelope: AdsrEnvelope::new_with(&preset.amp_envelope),

            lfo: Oscillator::new_lfo(&preset.lfo),
            lfo_routing: preset.lfo.routing,
            lfo_depth: preset.lfo.depth.into(),

            filter: BiQuadFilter::new_with(
                &FilterParams::LowPass24db {
                    cutoff: preset.filter_type_24db.cutoff_hz,
                    passband_ripple: BiQuadFilter::<EntityMessage>::denormalize_q(
                        preset.filter_resonance,
                    ),
                },
                sample_rate,
            ),
            filter_cutoff_start: BiQuadFilter::<EntityMessage>::frequency_to_percent(
                preset.filter_type_24db.cutoff_hz,
            ),
            filter_cutoff_end: preset.filter_envelope_weight,
            filter_envelope: AdsrEnvelope::new_with(&preset.filter_envelope),
            ..Default::default()
        };
        if !matches!(preset.oscillator_1.waveform, WaveformType::None) {
            r.oscillators
                .push(Oscillator::new_from_preset(&preset.oscillator_1));
        }
        if !matches!(preset.oscillator_2.waveform, WaveformType::None) {
            let mut o = Oscillator::new_from_preset(&preset.oscillator_2);
            if !preset.oscillator_2_track {
                if let crate::settings::patches::OscillatorTune::Note(note) =
                    preset.oscillator_2.tune
                {
                    o.set_fixed_frequency(MidiUtils::note_to_frequency(note));
                } else {
                    panic!("Patch configured without oscillator 2 tracking, but tune is not a note specification");
                }
            }
            r.oscillator_2_sync = preset.oscillator_2_sync;
            r.oscillators.push(o);
        }
        if preset.noise > 0.0 {
            r.oscillators
                .push(Oscillator::new_with(WaveformType::Noise));
        }
        r
    }

    pub(crate) fn is_playing(&self, clock: &Clock) -> bool {
        !self.amp_envelope.is_idle(clock)
    }
}
impl Updateable for WelshVoice {
    // TODO I really wanted this to be MidiMessage, but for now I'm borrowing
    // midly::MidiMessage, and it's missing at least one requirement of
    // MessageBounds' trait bounds.
    //
    // type Message = MidiMessage;
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        #[allow(unused_variables)]
        if let Self::Message::Midi(channel, message) = message {
            match message {
                MidiMessage::NoteOff { key, vel } => {
                    self.amp_envelope.handle_note_event(clock, false);
                    self.filter_envelope.handle_note_event(clock, false);
                }
                MidiMessage::NoteOn { key, vel } => {
                    let frequency = MidiUtils::message_to_frequency(&message);
                    for o in self.oscillators.iter_mut() {
                        o.set_frequency(frequency);
                    }
                    self.amp_envelope.handle_note_event(clock, true);
                    self.filter_envelope.handle_note_event(clock, true);
                }
                _ => {}
            }
        }

        Response::none()
    }
}

impl SourcesAudio for WelshVoice {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        // LFO
        let lfo = self.lfo.source_audio(clock);
        if matches!(self.lfo_routing, LfoRouting::Pitch) {
            let lfo_for_pitch = lfo * self.lfo_depth;
            for o in self.oscillators.iter_mut() {
                o.set_frequency_modulation(lfo_for_pitch);
            }
        }

        // Oscillators
        let len = self.oscillators.len();
        let osc_sum = match len {
            0 => 0.0,
            1 => self.oscillators[0].source_audio(clock),
            2 => {
                let osc_1_val = self.oscillators[0].source_audio(clock);
                if self.oscillator_2_sync && self.oscillators[0].has_period_restarted() {
                    self.oscillators[1].sync(clock);
                }
                (osc_1_val + self.oscillators[1].source_audio(clock)) / 2.0 as MonoSample
            }
            _ => todo!(),
        };

        // Filters
        //
        // https://aempass.blogspot.com/2014/09/analog-and-welshs-synthesizer-cookbook.html
        // I am not sure this is right.
        if self.filter_cutoff_end != 0.0 {
            let new_cutoff_percentage = self.filter_cutoff_start
                + (1.0 - self.filter_cutoff_start)
                    * self.filter_cutoff_end
                    * self.filter_envelope.source_audio(clock);
            self.filter.set_cutoff_pct(new_cutoff_percentage);
        } else if matches!(self.lfo_routing, LfoRouting::FilterCutoff) {
            let lfo_for_cutoff = lfo * self.lfo_depth;
            self.filter
                .set_cutoff_pct(self.filter_cutoff_start * (1.0 + lfo_for_cutoff));
        }
        let filtered_mix = self.filter.transform_audio(clock, osc_sum);

        // LFO amplitude modulation
        let lfo_for_amplitude = if matches!(self.lfo_routing, LfoRouting::Amplitude) {
            // LFO ranges from [-1, 1], so convert to something that can silence or double the volume.
            lfo * self.lfo_depth + 1.0
        } else {
            1.0
        };

        // Final
        filtered_mix * self.amp_envelope.source_audio(clock) * lfo_for_amplitude
    }
}

#[derive(Clone, Control, Debug, Uid)]
pub struct WelshSynth {
    uid: usize,
    sample_rate: usize,
    pub(crate) preset: SynthPatch,
    voices: Vec<WelshVoice>,
    notes_to_voice_indexes: FxHashMap<u8, usize>,

    debug_last_seconds: f32,
}
impl IsInstrument for WelshSynth {}
impl SourcesAudio for WelshSynth {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        if clock.seconds() == self.debug_last_seconds {
            panic!("We were called twice with the same time slice. Should this be OK?");
        } else {
            self.debug_last_seconds = clock.seconds();
        }

        // We previously scaled the sum to account for either all voices or all
        // voices that were playing. This led to icky discontinuities as that
        // number changed. As it is now, if you play a bunch of notes at once,
        // it's going to be very loud.
        self.voices
            .iter_mut()
            .filter(|v| v.is_playing(clock))
            .map(|v| v.source_audio(clock))
            .sum()
    }
}
impl Updateable for WelshSynth {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        #[allow(unused_variables)]
        match message {
            Self::Message::Midi(channel, midi_message) => match midi_message {
                MidiMessage::NoteOn { key, vel } => {
                    let voice = self.voice_for_note(clock, u8::from(key));
                    voice.update(clock, message);
                }
                MidiMessage::NoteOff { key, vel } => {
                    let voice = self.voice_for_note(clock, u8::from(key));
                    voice.update(clock, message);
                }
                MidiMessage::ProgramChange { program } => {
                    if let Some(program) = GeneralMidiProgram::from_u8(u8::from(program)) {
                        if let Ok(preset) = WelshSynth::general_midi_preset(&program) {
                            self.preset = preset;
                        } else {
                            println!("unrecognized patch from MIDI program change: {}", &program);
                        }
                    }
                }
                _ => {
                    println!("FYI - ignoring MIDI command {midi_message:?}");
                }
            },
            _ => todo!(),
        }
        Response::none()
    }
}

impl Default for WelshSynth {
    fn default() -> Self {
        Self {
            uid: Default::default(),
            sample_rate: usize::default(),
            preset: SynthPatch::default(),
            voices: Default::default(),
            notes_to_voice_indexes: Default::default(),
            debug_last_seconds: -1.0,
        }
    }
}
impl WelshSynth {
    pub(crate) fn new_with(sample_rate: usize, preset: SynthPatch) -> Self {
        Self {
            sample_rate,
            preset,
            ..Default::default()
        }
    }

    pub fn preset_name(&self) -> &str {
        self.preset.name.as_str()
    }

    // // TODO: this has unlimited-voice polyphony. Should we limit to a fixed number?
    // fn voice_for_note_old(&mut self, clock: &Clock, note: u8) -> &mut WelshVoice {
    //     // If we already have a voice for this note, return it.
    //     if let Some(&index) = self.note_to_voice_index.get(&note) {
    //         &mut self.voices[index]
    //     } else {
    //         // If there's an empty slot (a voice that's done playing), return that.
    //         if let Some(index) = self.voices.iter().position(|v| !v.is_playing(clock)) {
    //             self.note_to_voice_index.insert(note, index);
    //             &mut self.voices[index]
    //         } else {
    //             // All existing voices are playing. Make a new one.
    //             self.voices
    //                 .push(WelshVoice::new(self.sample_rate, &self.preset));
    //             let index = self.voices.len() - 1;
    //             self.note_to_voice_index.insert(note, index);
    //             &mut self.voices[index]
    //         }
    //     }
    // }

    // TODO: this has unlimited-voice polyphony. Should we limit to a fixed number?
    fn voice_for_note(&mut self, clock: &Clock, note: u8) -> &mut WelshVoice {
        if let Some(&index) = self.notes_to_voice_indexes.get(&note) {
            return &mut self.voices[index];
        }
        for (index, voice) in self.voices.iter().enumerate() {
            if !voice.is_playing(clock) {
                self.notes_to_voice_indexes.insert(note, index);
                return &mut self.voices[index];
            }
        }
        self.voices
            .push(WelshVoice::new_with(self.sample_rate, &self.preset));
        let index = self.voices.len() - 1;
        self.notes_to_voice_indexes.insert(note, index);
        &mut self.voices[index]
    }
}

#[cfg(test)]
mod tests {
    use super::WelshVoice;
    use super::*;
    use crate::{
        clock::Clock,
        instruments::welsh::WaveformType,
        midi::{MidiMessage, MidiUtils},
        settings::patches::{
            EnvelopeSettings, FilterPreset, LfoPreset, LfoRouting, OscillatorSettings,
            PolyphonySettings,
        },
        utils::tests::canonicalize_filename,
    };

    // TODO: refactor out to common test utilities
    #[allow(dead_code)]
    fn write_voice(voice: &mut WelshVoice, duration: f32, basename: &str) {
        let mut clock = Clock::default();

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

        let midi_on = MidiUtils::note_on_c4();
        let midi_off = MidiUtils::note_off_c4();

        let mut last_recognized_time_point = -1.;
        let time_note_off = duration / 2.0;
        while clock.seconds() < duration {
            if clock.seconds() >= 0.0 && last_recognized_time_point < 0.0 {
                last_recognized_time_point = clock.seconds();
                voice.update(&clock, EntityMessage::Midi(0, midi_on));
            } else if clock.seconds() >= time_note_off && last_recognized_time_point < time_note_off
            {
                last_recognized_time_point = clock.seconds();
                voice.update(&clock, EntityMessage::Midi(0, midi_off));
            }

            let sample = voice.source_audio(&clock);
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    // use std::panic;
    // use strum::IntoEnumIterator;
    // #[test]
    // #[should_panic]
    // fn test_presets() {
    //     let clock = Clock::new(&ClockSettings::new_defaults());
    //     for preset in PresetName::iter() {
    //         let result = panic::catch_unwind(|| {
    //             Voice::new(
    //                 MIDI_CHANNEL_RECEIVE_ALL,
    //                 clock.sample_rate(),
    //                 &super::SynthPreset::by_name(&preset),
    //             )
    //         });
    //         if result.is_ok() {
    //             let mut voice = result.unwrap();
    //             let preset_name = preset.to_string();
    //             write_voice(&mut voice, 2.0, &format!("voice_{}", preset_name));
    //         }
    //     }
    // }

    // This code was used to convert Rust representation of 26 Welsh patches to serde YAML.
    // #[derive(Serialize)]
    // struct Foo {
    //     x: Vec<SynthPreset>,
    // }

    // #[test]
    // #[should_panic]
    // fn test_presets() {
    //     for preset in PresetName::iter() {
    //         if let Ok(result) = panic::catch_unwind(|| super::SynthPreset::by_name(&preset)) {
    //             if let Ok(s) = serde_yaml::to_string(&result) {
    //                 if let Ok(_) = std::fs::write(format!("{}.yaml", result.name), s) {
    //                     // great
    //                 }
    //             }
    //         }
    //     }
    // }

    // TODO: get rid of this
    fn write_sound(
        source: &mut WelshVoice,
        clock: &mut Clock,
        duration: f32,
        message: &MidiMessage,
        when: f32,
        basename: &str,
    ) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

        let mut is_message_sent = false;
        while clock.seconds() < duration {
            if when <= clock.seconds() && !is_message_sent {
                is_message_sent = true;
                source.update(clock, EntityMessage::Midi(0, *message));
            }
            let sample = source.source_audio(clock);
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    fn cello_patch() -> SynthPatch {
        SynthPatch {
            name: SynthPatch::patch_name_to_settings_name("Cello"),
            oscillator_1: OscillatorSettings {
                waveform: WaveformType::PulseWidth(0.1),
                ..Default::default()
            },
            oscillator_2: OscillatorSettings {
                waveform: WaveformType::Square,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo: LfoPreset {
                routing: LfoRouting::Amplitude,
                waveform: WaveformType::Sine,
                frequency: 7.5,
                depth: crate::settings::patches::LfoDepth::Pct(5.0),
            },
            glide: 0.0,
            unison: false,
            polyphony: PolyphonySettings::Multi,
            filter_type_24db: FilterPreset {
                cutoff_hz: 40.0,
                cutoff_pct: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff_hz: 40.0,
                cutoff_pct: 0.1,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 0.9,
            filter_envelope: EnvelopeSettings {
                attack: 0.0,
                decay: 3.29,
                sustain: 0.78,
                release: EnvelopeSettings::MAX,
            },
            amp_envelope: EnvelopeSettings {
                attack: 0.06,
                decay: EnvelopeSettings::MAX,
                sustain: 1.0,
                release: 0.3,
            },
        }
    }

    fn test_patch() -> SynthPatch {
        SynthPatch {
            name: SynthPatch::patch_name_to_settings_name("Test"),
            oscillator_1: OscillatorSettings {
                waveform: WaveformType::Sawtooth,
                ..Default::default()
            },
            oscillator_2: OscillatorSettings {
                waveform: WaveformType::None,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo: LfoPreset {
                routing: LfoRouting::None,
                ..Default::default()
            },
            glide: 0.0,
            unison: false,
            polyphony: PolyphonySettings::Multi,
            filter_type_24db: FilterPreset {
                cutoff_hz: 40.0,
                cutoff_pct: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff_hz: 20.0,
                cutoff_pct: 0.05,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 1.0,
            filter_envelope: EnvelopeSettings {
                attack: 5.0,
                decay: EnvelopeSettings::MAX,
                sustain: 1.0,
                release: EnvelopeSettings::MAX,
            },
            amp_envelope: EnvelopeSettings {
                attack: 0.5,
                decay: EnvelopeSettings::MAX,
                sustain: 1.0,
                release: EnvelopeSettings::MAX,
            },
        }
    }

    #[test]
    fn test_basic_synth_patch() {
        let message_on = MidiUtils::note_on_c4();
        let message_off = MidiUtils::note_off_c4();

        let mut clock = Clock::default();
        let mut voice = WelshVoice::new_with(clock.sample_rate(), &test_patch());
        voice.update(&clock, EntityMessage::Midi(0, message_on));
        write_sound(
            &mut voice,
            &mut clock,
            5.0,
            &message_off,
            5.0,
            "voice_basic_test_c4",
        );
    }

    #[test]
    fn test_basic_cello_patch() {
        let message_on = MidiUtils::note_on_c4();
        let message_off = MidiUtils::note_off_c4();

        let mut clock = Clock::default();
        let mut voice = WelshVoice::new_with(clock.sample_rate(), &cello_patch());
        voice.update(&clock, EntityMessage::Midi(0, message_on));
        write_sound(
            &mut voice,
            &mut clock,
            5.0,
            &message_off,
            1.0,
            "voice_cello_c4",
        );
    }
}
