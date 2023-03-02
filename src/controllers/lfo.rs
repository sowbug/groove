use crate::{
    instruments::oscillators::Oscillator,
    messages::EntityMessage,
    settings::{patches::WaveformType, ClockSettings},
};
use core::fmt::Debug;
use groove_core::{
    control::F32ControlValue,
    midi::HandlesMidi,
    traits::{Controllable, Generates, HasUid, IsController, Resets, Ticks, TicksWithMessages},
    BipolarNormal, ParameterType,
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

/// Uses an internal LFO as a control source.
#[derive(Control, Debug, Uid)]
pub struct LfoController {
    uid: usize,
    oscillator: Oscillator,
}
impl IsController<EntityMessage> for LfoController {}
impl Resets for LfoController {}
impl TicksWithMessages<EntityMessage> for LfoController {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        self.oscillator.tick(tick_count);
        // TODO: opportunity to use from() to convert properly from 0..1 to -1..0
        (
            Some(vec![EntityMessage::ControlF32(
                BipolarNormal::from(self.oscillator.value()).value() as f32,
            )]),
            0,
        )
    }
}
impl HandlesMidi for LfoController {}
impl LfoController {
    pub fn new_with(
        clock_settings: &ClockSettings,
        waveform: WaveformType,
        frequency_hz: ParameterType,
    ) -> Self {
        Self {
            uid: Default::default(),
            oscillator: Oscillator::new_with_type_and_frequency(
                clock_settings.sample_rate(),
                waveform,
                frequency_hz as f32,
            ),
        }
    }
}
