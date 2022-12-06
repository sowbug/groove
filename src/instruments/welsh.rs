use crate::{
    common::MonoSample,
    effects::filter::{BiQuadFilter, FilterParams},
    messages::EntityMessage,
    midi::{GeneralMidiProgram, MidiMessage, MidiUtils},
    settings::{
        patches::{LfoRouting, SynthPatch, WaveformType},
        LoadError,
    },
    traits::{HasUid, IsInstrument, Response, SourcesAudio, TransformsAudio, Updateable},
    Clock,
};
use convert_case::{Case, Casing};
use num_traits::FromPrimitive;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::f32::consts::FRAC_1_SQRT_2;
use strum_macros::{Display, EnumIter};

use super::{envelopes::AdsrEnvelope, oscillators::Oscillator};

#[derive(Clone, Debug, Deserialize, Display, EnumIter, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PatchName {
    // -------------------- Strings
    Banjo,
    Cello,
    DoubleBass,
    Dulcimer,
    GuitarAcoustic,
    GuitarElectric,
    Harp,
    HurdyGurdy,
    Kora,
    Lute,
    Mandocello,
    Mandolin,
    Riti,
    Sitar,
    StandupBass,
    Viola,
    Violin,
    // -------------------- Woodwinds
    Bagpipes,
    BassClarinet,
    Bassoon,
    Clarinet,
    ConchShell,
    Contrabassoon,
    Digeridoo,
    EnglishHorn,
    Flute,
    Oboe,
    Piccolo,
    // -------------------- Brass
    FrenchHorn,
    Harmonica,
    PennyWhistle,
    Saxophone,
    Trombone,
    Trumpet,
    Tuba,
    // -------------------- Keyboards
    Accordion,
    Celeste,
    Clavichord,
    ElectricPiano,
    Harpsichord,
    Organ,
    Piano,
    // -------------------- Vocals
    Angels,
    Choir,
    VocalFemale,
    VocalMale,
    Whistling,
    // -------------------- Tuned Percussion
    Bell,
    Bongos,
    Conga,
    Glockenspiel,
    Marimba,
    Timpani,
    Xylophone,
    // -------------------- Untuned Percussion
    BassDrum,
    Castanets,
    Clap,
    Claves,
    Cowbell,
    CowbellAnalog,
    Cymbal,
    SideStick,
    SnareDrum,
    Tambourine,
    WheelsOfSteel,
    // -------------------- Leads
    BrassSection,
    Mellow70sLead,
    MonoSolo,
    NewAgeLead,
    RAndBSlide,
    ScreamingSync,
    StringsPwm,
    Trance5th,
    // -------------------- Bass
    AcidBass,
    BassOfTheTimeLords,
    DetroitBass,
    DeutscheBass,
    DigitalBass,
    FunkBass,
    GrowlingBass,
    RezBass,
    // -------------------- Pads
    AndroidDreams,
    CelestialWash,
    DarkCity,
    Aurora,
    GalacticCathedral,
    GalacticChapel,
    Portus,
    PostApocalypticSyncSweep,
    TerraEnceladus,
    // -------------------- Sound Effects
    Cat,
    DigitalAlarmClock,
    JourneyToTheCore,
    Kazoo,
    Laser,
    Motor,
    NerdOTron2000,
    OceanWavesWithFoghorn,
    PositronicRhythm,
    SpaceAttack,
    Toad,
    Wind,
}

// TODO: cache these as they're loaded
impl SynthPatch {
    pub fn patch_name_to_settings_name(name: &str) -> String {
        name.to_case(Case::Kebab)
    }

    pub fn new_from_yaml(yaml: &str) -> Result<Self, LoadError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            println!("{e}");
            LoadError::FormatError
        })
    }

    pub fn by_name(name: &PatchName) -> Self {
        let filename = format!(
            "assets/patches/welsh/{}.yaml",
            Self::patch_name_to_settings_name(name.to_string().as_str())
        );
        if let Ok(contents) = std::fs::read_to_string(filename) {
            match Self::new_from_yaml(&contents) {
                Ok(patch) => patch,
                Err(err) => {
                    // TODO: this should return a failsafe patch, maybe a boring
                    // square wave
                    panic!("couldn't parse patch file: {:?}", err);
                }
            }
        } else {
            panic!("couldn't read patch file");
        }
    }
}

impl WelshSynth {
    pub fn general_midi_preset(program: &GeneralMidiProgram) -> anyhow::Result<SynthPatch> {
        let mut delegated = false;
        let preset = match program {
            GeneralMidiProgram::AcousticGrand => PatchName::Piano,
            GeneralMidiProgram::BrightAcoustic => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::ElectricGrand => PatchName::ElectricPiano,
            GeneralMidiProgram::HonkyTonk => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::ElectricPiano1 => PatchName::ElectricPiano,
            GeneralMidiProgram::ElectricPiano2 => PatchName::ElectricPiano,
            GeneralMidiProgram::Harpsichord => PatchName::Harpsichord,
            GeneralMidiProgram::Clav => PatchName::Clavichord,
            GeneralMidiProgram::Celesta => PatchName::Celeste,
            GeneralMidiProgram::Glockenspiel => PatchName::Glockenspiel,
            GeneralMidiProgram::MusicBox => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Vibraphone => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Marimba => PatchName::Marimba,
            GeneralMidiProgram::Xylophone => PatchName::Xylophone,
            GeneralMidiProgram::TubularBells => PatchName::Bell,
            GeneralMidiProgram::Dulcimer => PatchName::Dulcimer,
            GeneralMidiProgram::DrawbarOrgan => {
                PatchName::Organ // TODO dup
            }
            GeneralMidiProgram::PercussiveOrgan => {
                PatchName::Organ // TODO dup
            }
            GeneralMidiProgram::RockOrgan => {
                PatchName::Organ // TODO dup
            }
            GeneralMidiProgram::ChurchOrgan => {
                PatchName::Organ // TODO dup
            }
            GeneralMidiProgram::ReedOrgan => {
                PatchName::Organ // TODO dup
            }
            GeneralMidiProgram::Accordion => PatchName::Accordion,
            GeneralMidiProgram::Harmonica => PatchName::Harmonica,
            GeneralMidiProgram::TangoAccordion => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::AcousticGuitarNylon => PatchName::GuitarAcoustic,
            GeneralMidiProgram::AcousticGuitarSteel => {
                PatchName::GuitarAcoustic // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarJazz => {
                PatchName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarClean => {
                PatchName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarMuted => {
                PatchName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::OverdrivenGuitar => {
                PatchName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::DistortionGuitar => {
                PatchName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::GuitarHarmonics => {
                PatchName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::AcousticBass => PatchName::DoubleBass,
            GeneralMidiProgram::ElectricBassFinger => PatchName::StandupBass,
            GeneralMidiProgram::ElectricBassPick => PatchName::AcidBass,
            GeneralMidiProgram::FretlessBass => {
                PatchName::DetroitBass // TODO same?
            }
            GeneralMidiProgram::SlapBass1 => PatchName::FunkBass,
            GeneralMidiProgram::SlapBass2 => PatchName::FunkBass,
            GeneralMidiProgram::SynthBass1 => PatchName::DigitalBass,
            GeneralMidiProgram::SynthBass2 => PatchName::DigitalBass,
            GeneralMidiProgram::Violin => PatchName::Violin,
            GeneralMidiProgram::Viola => PatchName::Viola,
            GeneralMidiProgram::Cello => PatchName::Cello,
            GeneralMidiProgram::Contrabass => PatchName::Contrabassoon,
            GeneralMidiProgram::TremoloStrings => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::PizzicatoStrings => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::OrchestralHarp => PatchName::Harp,
            GeneralMidiProgram::Timpani => PatchName::Timpani,
            GeneralMidiProgram::StringEnsemble1 => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::StringEnsemble2 => {
                PatchName::StringsPwm // TODO same?
            }
            GeneralMidiProgram::Synthstrings1 => PatchName::StringsPwm, // TODO same?

            GeneralMidiProgram::Synthstrings2 => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::ChoirAahs => PatchName::Angels,

            GeneralMidiProgram::VoiceOohs => PatchName::Choir,
            GeneralMidiProgram::SynthVoice => PatchName::VocalFemale,

            GeneralMidiProgram::OrchestraHit => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Trumpet => PatchName::Trumpet,
            GeneralMidiProgram::Trombone => PatchName::Trombone,
            GeneralMidiProgram::Tuba => PatchName::Tuba,
            GeneralMidiProgram::MutedTrumpet => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::FrenchHorn => PatchName::FrenchHorn,

            GeneralMidiProgram::BrassSection => PatchName::BrassSection,

            GeneralMidiProgram::Synthbrass1 => {
                PatchName::BrassSection // TODO dup
            }
            GeneralMidiProgram::Synthbrass2 => {
                PatchName::BrassSection // TODO dup
            }
            GeneralMidiProgram::SopranoSax => {
                PatchName::Saxophone // TODO dup
            }
            GeneralMidiProgram::AltoSax => PatchName::Saxophone,
            GeneralMidiProgram::TenorSax => {
                PatchName::Saxophone // TODO dup
            }
            GeneralMidiProgram::BaritoneSax => {
                PatchName::Saxophone // TODO dup
            }
            GeneralMidiProgram::Oboe => PatchName::Oboe,
            GeneralMidiProgram::EnglishHorn => PatchName::EnglishHorn,
            GeneralMidiProgram::Bassoon => PatchName::Bassoon,
            GeneralMidiProgram::Clarinet => PatchName::Clarinet,
            GeneralMidiProgram::Piccolo => PatchName::Piccolo,
            GeneralMidiProgram::Flute => PatchName::Flute,
            GeneralMidiProgram::Recorder => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::PanFlute => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::BlownBottle => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Shakuhachi => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Whistle => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Ocarina => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Lead1Square => {
                PatchName::MonoSolo // TODO: same?
            }
            GeneralMidiProgram::Lead2Sawtooth => {
                PatchName::Trance5th // TODO: same?
            }
            GeneralMidiProgram::Lead3Calliope => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Lead4Chiff => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Lead5Charang => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Lead6Voice => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Lead7Fifths => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Lead8BassLead => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Pad1NewAge => {
                PatchName::NewAgeLead // TODO pad or lead?
            }
            GeneralMidiProgram::Pad2Warm => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Pad3Polysynth => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Pad4Choir => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Pad5Bowed => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Pad6Metallic => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Pad7Halo => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Pad8Sweep => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Fx1Rain => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Fx2Soundtrack => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Fx3Crystal => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Fx4Atmosphere => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Fx5Brightness => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Fx6Goblins => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Fx7Echoes => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Fx8SciFi => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Sitar => PatchName::Sitar,
            GeneralMidiProgram::Banjo => PatchName::Banjo,
            GeneralMidiProgram::Shamisen => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Koto => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Kalimba => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Bagpipe => PatchName::Bagpipes,
            GeneralMidiProgram::Fiddle => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Shanai => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::TinkleBell => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Agogo => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::SteelDrums => {
                PatchName::WheelsOfSteel // TODO same?
            }
            GeneralMidiProgram::Woodblock => PatchName::SideStick,
            GeneralMidiProgram::TaikoDrum => {
                // XXXXXXXXXXXXX TMP
                PatchName::Cello // TODO substitute.....
            }
            GeneralMidiProgram::MelodicTom => PatchName::Bongos,
            GeneralMidiProgram::SynthDrum => PatchName::SnareDrum,
            GeneralMidiProgram::ReverseCymbal => PatchName::Cymbal,
            GeneralMidiProgram::GuitarFretNoise => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::BreathNoise => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Seashore => PatchName::OceanWavesWithFoghorn,
            GeneralMidiProgram::BirdTweet => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::TelephoneRing => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Helicopter => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Applause => {
                delegated = true;
                PatchName::Piano
            }
            GeneralMidiProgram::Gunshot => {
                delegated = true;
                PatchName::Piano
            }
        };
        if delegated {
            println!("Delegated {program} to {preset}");
        }
        Ok(SynthPatch::by_name(&preset))
    }
}

#[derive(Clone, Debug, Default)]
pub struct WelshVoice {
    oscillators: Vec<Oscillator>,
    osc_mix: Vec<f32>,
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
            lfo_depth: preset.lfo.depth,

            filter: BiQuadFilter::new_with(
                &FilterParams::LowPass {
                    cutoff: preset.filter_type_12db.cutoff,
                    q: FRAC_1_SQRT_2, // TODO: resonance
                },
                sample_rate,
            ),
            filter_cutoff_start: BiQuadFilter::<EntityMessage>::frequency_to_percent(
                preset.filter_type_12db.cutoff,
            ),
            filter_cutoff_end: preset.filter_envelope_weight,
            filter_envelope: AdsrEnvelope::new_with(&preset.filter_envelope),
            ..Default::default()
        };
        if !matches!(preset.oscillator_1.waveform, WaveformType::None) {
            r.oscillators
                .push(Oscillator::new_from_preset(&preset.oscillator_1));
            r.osc_mix.push(preset.oscillator_1.mix);
        }
        if !matches!(preset.oscillator_2.waveform, WaveformType::None) {
            let mut o = Oscillator::new_from_preset(&preset.oscillator_2);
            if !preset.oscillator_2_track {
                o.set_fixed_frequency(MidiUtils::note_to_frequency(preset.oscillator_2.tune as u8));
            }
            r.oscillators.push(o);
            r.osc_mix.push(preset.oscillator_2.mix);
        }
        if preset.noise > 0.0 {
            r.oscillators
                .push(Oscillator::new_with(WaveformType::Noise));
            r.osc_mix.push(preset.noise);
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
        match message {
            Self::Message::Midi(channel, message) => match message {
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
            },
            _ => {}
        }

        Response::none()
    }
}

impl SourcesAudio for WelshVoice {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        // LFO
        let lfo = self.lfo.source_audio(clock) * self.lfo_depth as MonoSample;
        if matches!(self.lfo_routing, LfoRouting::Pitch) {
            let lfo_for_pitch = lfo / 10000.0;
            // TODO: divide by 10,000 until we figure out how pitch depth is supposed to go
            // TODO: this could leave a side effect if we reuse voices and forget to clean up.
            for o in self.oscillators.iter_mut() {
                o.set_frequency_modulation(lfo_for_pitch);
            }
        }

        // Oscillators
        let osc_sum = if self.oscillators.is_empty() {
            0.0
        } else {
            let t: MonoSample = self
                .oscillators
                .iter_mut()
                .map(|o| o.source_audio(clock))
                .sum();
            t / self.oscillators.len() as MonoSample
        };

        // Filters
        let new_cutoff_percentage = self.filter_cutoff_start
            + (self.filter_cutoff_end - self.filter_cutoff_start)
                * self.filter_envelope.source_audio(clock);
        self.filter.set_cutoff_pct(new_cutoff_percentage);
        let filtered_mix = self.filter.transform_audio(clock, osc_sum);

        // LFO amplitude modulation
        let lfo_amplitude_modulation = if matches!(self.lfo_routing, LfoRouting::Amplitude) {
            // LFO ranges from [-1, 1], so convert to something that can silence or double the volume.
            lfo + 1.0
        } else {
            1.0
        };

        // Final
        filtered_mix * self.amp_envelope.source_audio(clock) * lfo_amplitude_modulation
    }
}

#[derive(Clone, Debug)]
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
                    println!("FYI - ignoring MIDI command {:?}", midi_message);
                }
            },
            _ => todo!(),
        }
        Response::none()
    }
}
impl HasUid for WelshSynth {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
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
        let mut r = Self::default();
        r.sample_rate = sample_rate;
        r.preset = preset;
        r
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
        return &mut self.voices[index];
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
            EnvelopeSettings, FilterPreset, GlideSettings, LfoPreset, LfoRouting,
            OscillatorSettings, PolyphonySettings,
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
                depth: LfoPreset::percent(5.0),
            },
            glide: GlideSettings::Off,
            has_unison: false,
            polyphony: PolyphonySettings::Multi,
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
            glide: GlideSettings::Off,
            has_unison: false,
            polyphony: PolyphonySettings::Multi,
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
