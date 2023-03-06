use groove_core::ParameterType;
use groove_entities::instruments::{FmVoice, VoiceStore};

pub(crate) struct FmSynthesizerPreset {
    modulator_frequency_hz: ParameterType,
}

impl FmSynthesizerPreset {
    pub fn into_voice_store(&self, sample_rate: usize) -> VoiceStore<FmVoice> {
        VoiceStore::<FmVoice>::new_with_voice(sample_rate, 8, || self.into_voice(sample_rate))
    }

    pub fn into_voice(&self, sample_rate: usize) -> FmVoice {
        FmVoice::new_with_modulator_frequency(sample_rate, self.modulator_frequency_hz)
    }

    pub fn from_name(_name: &str) -> FmSynthesizerPreset {
        FmSynthesizerPreset {
            modulator_frequency_hz: 388.0,
        }
    }
}
