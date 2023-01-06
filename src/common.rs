pub type MonoSample = f32;
// impl Default for MonoSample {
//     fn default() -> Self {
//         MONO_SAMPLE_SILENCE
//     }
// }
#[allow(dead_code)]
pub type StereoSample = (MonoSample, MonoSample);
pub const MONO_SAMPLE_SILENCE: MonoSample = 0.0;
pub const MONO_SAMPLE_MAX: MonoSample = 1.0;
pub const MONO_SAMPLE_MIN: MonoSample = -1.0;

pub type DeviceId = String;

pub struct F32ControlValue(pub f32);
