use crate::clock::ClockTimeUnit;
use crate::gui::Viewable;
use crate::messages::EntityMessage;
use crate::{clock::Clock, common::MonoSample, messages::MessageBounds};
use crate::{
    common::MONO_SAMPLE_SILENCE,
    instruments::oscillators::Oscillator,
    midi::{MidiChannel, MidiUtils},
    settings::patches::WaveformType,
};
use assert_approx_eq::assert_approx_eq;
use midly::MidiMessage;
use std::collections::VecDeque;
use std::{marker::PhantomData, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

/// An IsController creates events that drive other things in the system.
/// Examples are sequencers and arpeggiators. They get called on each time slice
/// so that they can do work and send any needed messages. An IsController
/// implements Terminates, which indicates that it's done emitting events (and,
/// in the case of timers and sequencers, done waiting for other work in the
/// system to complete).
pub trait IsController:
    Updateable + Terminates + HasUid + Viewable + Send + std::fmt::Debug
{
}

/// An IsEffect transforms audio. It takes audio inputs and produces audio
/// output. It does not get called unless there is audio input to provide to it
/// (which can include silence, e.g., in the case of a muted instrument).
///
/// IsEffects don't terminate in the Terminates sense. This might become an
/// issue in the future, for example if a composition's end is determined by the
/// amount of time it takes for an effect to finish processing inputs (e.g., a
/// delay effect), and it turns out to be inconvenient for an IsController to
/// track the end. In this case, we might add a Terminates bound for IsEffect.
/// But right now I'm not sure that's the right solution.
pub trait IsEffect:
    TransformsAudio + Updateable + HasUid + Viewable + Send + std::fmt::Debug
{
}

/// An IsInstrument produces audio, usually upon request from MIDI or
/// InController input. Like IsEffect, IsInstrument doesn't implement Terminates
/// because it continues to create audio as long as asked.
pub trait IsInstrument:
    SourcesAudio + Updateable + HasUid + Viewable + Send + std::fmt::Debug
{
}

/// A future fourth trait might be named something like IsWidget or
/// IsGuiElement. These exist only to interact with the user of a GUI app, but
/// don't actually create or control audio.

/// BoxedEntity wraps any of the major trait types to allow us to store them in
/// a single big list. This was done for convenience at first, and it's unclear
/// how much benefit it provides over keeping the types in separate lists and
/// trying not to mix them up. In practice, we haven't needed to discover an
/// entity's major type in any way that wouldn't have also been doable with
/// separate lists.
#[derive(Debug)]
pub enum BoxedEntity<M> {
    Controller(Box<dyn IsController<Message = M, ViewMessage = M>>),
    Effect(Box<dyn IsEffect<Message = M, ViewMessage = M>>),
    Instrument(Box<dyn IsInstrument<Message = M, ViewMessage = M>>),
}

/// An Updateable accepts new information through update() (i.e., Messages) or
/// control parameters.
///
/// Methods and messages are isomorphic, and everything could have been done
/// through update(), but sometimes a direct method is the right solution.
pub trait Updateable {
    type Message: MessageBounds;

    #[allow(unused_variables)]
    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        Response::none()
    }
    #[allow(unused_variables)]
    fn param_id_for_name(&self, name: &str) -> usize {
        todo!()
    }
    #[allow(unused_variables)]
    fn set_indexed_param_f32(&mut self, index: usize, value: f32) {
        todo!()
    }
}

/// A HasUid has an ephemeral but globally unique numeric identifier, which is
/// useful for one entity to refer to another without getting into icky Rust
/// ownership questions. It's the foundation of any ECS
/// (entity/component/system) design.
pub trait HasUid {
    fn uid(&self) -> usize;
    fn set_uid(&mut self, uid: usize);
}

/// A SourcesAudio provides audio in the form of digital samples.
pub trait SourcesAudio: std::fmt::Debug + Send {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample;
}

/// A TransformsAudio takes input audio, which is typically produced by
/// SourcesAudio, does something to it, and then outputs it. It's what effects
/// do.
pub trait TransformsAudio: std::fmt::Debug {
    fn transform_audio(&mut self, clock: &Clock, input_sample: MonoSample) -> MonoSample;
}

// A Terminates has a point in time where it would be OK never being called or
// continuing to exist.
//
// If you're required to implement Terminates, but you don't know when you need
// to terminate, then you should always return true. For example, an arpeggiator
// would be happy to keep responding to MIDI input forever. Which is (a little
// strangely) the same as saying it would be happy to quit at any time. Thus it
// should always return true.
//
// The reason to choose true rather than false is that the system uses
// is_finished() to determine whether a song is complete. If a Terminates never
// returns true, the loop will never end. Thus, "is_finished" is more like "is
// unaware of any reason to continue existing" rather than "is certain there is
// no more work to do."
pub trait Terminates: std::fmt::Debug {
    fn is_finished(&self) -> bool;
}

#[derive(Debug)]
pub struct Response<T>(pub Internal<T>);

#[derive(Debug)]
pub enum Internal<T> {
    None,
    Single(T),
    Batch(Vec<T>),
}

impl<T> Response<T> {
    pub const fn none() -> Self {
        Self(Internal::None)
    }

    pub const fn single(action: T) -> Self {
        Self(Internal::Single(action))
    }

    pub fn batch(commands: impl IntoIterator<Item = Response<T>>) -> Self {
        let mut batch = Vec::new();

        for Response(command) in commands {
            match command {
                Internal::None => {}
                Internal::Single(command) => batch.push(command),
                Internal::Batch(commands) => batch.extend(commands),
            }
        }
        if batch.is_empty() {
            Self(Internal::None)
        } else {
            Self(Internal::Batch(batch))
        }
    }
}

// NOTE: The Test... entities are in the non-tests module because they're
// sometimes useful as simple real entities to substitute in for production
// ones, for example if we're trying to determine whether an entity is
// responsible for a performance issue.

pub trait TestsValues {
    fn check_values(&mut self, clock: &Clock) {
        // If we've been asked to assert values at checkpoints, do so.
        if self.has_checkpoint_values()
            && clock.time_for(self.time_unit()) >= self.checkpoint_time()
        {
            const SAD_FLOAT_DIFF: f32 = 1.0e-4;
            if let Some(value) = self.pop_checkpoint_value() {
                assert_approx_eq!(self.value_to_check(), value, SAD_FLOAT_DIFF);
            }
            self.advance_checkpoint_time();
        }
    }

    fn has_checkpoint_values(&self) -> bool;
    fn time_unit(&self) -> &ClockTimeUnit;
    fn checkpoint_time(&self) -> f32;
    fn advance_checkpoint_time(&mut self);
    fn value_to_check(&self) -> f32;
    fn pop_checkpoint_value(&mut self) -> Option<f32>;
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum TestControllerControlParams {
    Tempo,
}

enum TestControllerAction {
    Nothing,
    NoteOn,
    NoteOff,
}

#[derive(Debug, Default)]
pub struct TestController<M: MessageBounds> {
    uid: usize,
    midi_channel_out: MidiChannel,
    pub tempo: f32,
    is_enabled: bool,
    is_playing: bool,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsController for TestController<M> {}
impl<M: MessageBounds> Terminates for TestController<M> {
    fn is_finished(&self) -> bool {
        true
    }
}
impl<M: MessageBounds> Updateable for TestController<M> {
    default type Message = M;

    default fn update(
        &mut self,
        _clock: &Clock,
        _message: Self::Message,
    ) -> Response<Self::Message> {
        Response::none()
    }
}
impl<M: MessageBounds> HasUid for TestController<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid
    }
}
impl<M: MessageBounds> TestController<M> {
    pub fn new_with(midi_channel_out: MidiChannel) -> Self {
        Self {
            midi_channel_out,
            ..Default::default()
        }
    }

    pub fn new_with_test_values(
        midi_channel_out: MidiChannel,
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            midi_channel_out,
            checkpoint_values: VecDeque::from(Vec::from(values)),
            checkpoint,
            checkpoint_delta,
            time_unit,
            ..Default::default()
        }
    }

    fn what_to_do(&self, clock: &Clock) -> TestControllerAction {
        let beat_slice_start = clock.beats();
        let beat_slice_end = clock.next_slice_in_beats();
        let next_exact_beat = beat_slice_start.floor();
        let next_exact_half_beat = next_exact_beat + 0.5;
        if next_exact_beat >= beat_slice_start && next_exact_beat < beat_slice_end {
            return TestControllerAction::NoteOn;
        }
        if next_exact_half_beat >= beat_slice_start && next_exact_half_beat < beat_slice_end {
            return TestControllerAction::NoteOff;
        }
        TestControllerAction::Nothing
    }
}
impl<M: MessageBounds> TestsValues for TestController<M> {
    fn has_checkpoint_values(&self) -> bool {
        !self.checkpoint_values.is_empty()
    }

    fn time_unit(&self) -> &ClockTimeUnit {
        &self.time_unit
    }

    fn checkpoint_time(&self) -> f32 {
        self.checkpoint
    }

    fn advance_checkpoint_time(&mut self) {
        self.checkpoint += self.checkpoint_delta;
    }

    fn value_to_check(&self) -> f32 {
        self.tempo
    }

    fn pop_checkpoint_value(&mut self) -> Option<f32> {
        self.checkpoint_values.pop_front()
    }
}

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum TestEffectControlParams {
    MyValue,
}

#[derive(Debug, Default)]
pub struct TestEffect<M: MessageBounds> {
    uid: usize,
    my_value: f32,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsEffect for TestEffect<M> {}
impl<M: MessageBounds> TransformsAudio for TestEffect<M> {
    fn transform_audio(&mut self, clock: &Clock, input_sample: MonoSample) -> MonoSample {
        self.check_values(clock);
        -input_sample
    }
}
impl<M: MessageBounds> Updateable for TestEffect<M> {
    default type Message = M;

    #[allow(unused_variables)]
    default fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        Response::none()
    }

    #[allow(unused_variables)]
    default fn param_id_for_name(&self, name: &str) -> usize {
        todo!()
    }
}
impl<M: MessageBounds> HasUid for TestEffect<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> TestsValues for TestEffect<M> {
    fn has_checkpoint_values(&self) -> bool {
        !self.checkpoint_values.is_empty()
    }

    fn time_unit(&self) -> &ClockTimeUnit {
        &self.time_unit
    }

    fn checkpoint_time(&self) -> f32 {
        self.checkpoint
    }

    fn advance_checkpoint_time(&mut self) {
        self.checkpoint += self.checkpoint_delta;
    }

    fn value_to_check(&self) -> f32 {
        self.my_value()
    }

    fn pop_checkpoint_value(&mut self) -> Option<f32> {
        self.checkpoint_values.pop_front()
    }
}
impl<M: MessageBounds> TestEffect<M> {
    pub fn new_with_test_values(
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            checkpoint_values: VecDeque::from(Vec::from(values)),
            checkpoint,
            checkpoint_delta,
            time_unit,
            ..Default::default()
        }
    }

    pub fn set_my_value(&mut self, my_value: f32) {
        self.my_value = my_value;
    }

    pub fn my_value(&self) -> f32 {
        self.my_value
    }
}

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum TestInstrumentControlParams {
    // -1.0 is Sawtooth, 1.0 is Square, anything else is Sine.
    Waveform,

    // A fake adjustable number.
    FakeValue,
}

/// A simple implementation of IsInstrument that's useful for testing and
/// debugging. Uses a default Oscillator to produce sound, and its "envelope" is
/// just a boolean that responds to MIDI NoteOn/NoteOff.
///
/// To act as a controller target, it has two parameters: Oscillator waveform
/// and frequency.
#[derive(Debug, Default)]
pub struct TestInstrument<M: MessageBounds> {
    uid: usize,

    oscillator: Oscillator,
    pub is_playing: bool,
    pub received_count: usize,
    pub handled_count: usize,

    pub fake_value: f32,
    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,

    pub debug_messages: Vec<(f32, MidiChannel, MidiMessage)>,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsInstrument for TestInstrument<M> {}
impl<M: MessageBounds> Updateable for TestInstrument<M> {
    default type Message = M;

    default fn update(
        &mut self,
        _clock: &Clock,
        _message: Self::Message,
    ) -> Response<Self::Message> {
        Response::none()
    }

    fn param_id_for_name(&self, param_name: &str) -> usize {
        if let Ok(param) = TestInstrumentControlParams::from_str(param_name) {
            param as usize
        } else {
            usize::MAX
        }
    }
}
impl<M: MessageBounds> HasUid for TestInstrument<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> TestsValues for TestInstrument<M> {
    fn has_checkpoint_values(&self) -> bool {
        !self.checkpoint_values.is_empty()
    }

    fn time_unit(&self) -> &ClockTimeUnit {
        &self.time_unit
    }

    fn checkpoint_time(&self) -> f32 {
        self.checkpoint
    }

    fn advance_checkpoint_time(&mut self) {
        self.checkpoint += self.checkpoint_delta;
    }

    fn value_to_check(&self) -> f32 {
        self.fake_value
    }

    fn pop_checkpoint_value(&mut self) -> Option<f32> {
        self.checkpoint_values.pop_front()
    }
}
impl<M: MessageBounds> TestInstrument<M> {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_with_test_values(
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            checkpoint_values: VecDeque::from(Vec::from(values)),
            checkpoint,
            checkpoint_delta,
            time_unit,
            ..Default::default()
        }
    }

    pub fn handle_midi(&mut self, clock: &Clock, channel: MidiChannel, message: MidiMessage) {
        self.debug_messages.push((clock.beats(), channel, message));
        self.received_count += 1;

        match message {
            MidiMessage::NoteOn { key, vel: _ } => {
                self.is_playing = true;
                self.oscillator
                    .set_frequency(MidiUtils::note_to_frequency(key.as_int()));
            }
            MidiMessage::NoteOff { key: _, vel: _ } => {
                self.is_playing = false;
            }
            _ => {}
        }
    }

    #[allow(dead_code)]
    fn waveform(&self) -> f32 {
        match self.oscillator.waveform() {
            WaveformType::Sawtooth => -1.0,
            WaveformType::Square => 1.0,
            _ => 0.0,
        }
    }

    pub fn set_waveform(&mut self, value: f32) {
        self.oscillator.set_waveform(if value == -1.0 {
            WaveformType::Sawtooth
        } else if value == 1.0 {
            WaveformType::Square
        } else {
            WaveformType::Sine
        });
    }

    pub fn set_fake_value(&mut self, fake_value: f32) {
        self.fake_value = fake_value;
    }

    pub fn fake_value(&self) -> f32 {
        self.fake_value
    }
}

impl<M: MessageBounds> SourcesAudio for TestInstrument<M> {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        // If we've been asked to assert values at checkpoints, do so.
        if !self.checkpoint_values.is_empty() && clock.time_for(&self.time_unit) >= self.checkpoint
        {
            const SAD_FLOAT_DIFF: f32 = 1.0e-2;
            assert_approx_eq!(self.fake_value, self.checkpoint_values[0], SAD_FLOAT_DIFF);
            self.checkpoint += self.checkpoint_delta;
            self.checkpoint_values.pop_front();
        }
        if self.is_playing {
            self.oscillator.source_audio(clock)
        } else {
            MONO_SAMPLE_SILENCE
        }
    }
}

impl Updateable for TestController<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::Tick => {
                self.check_values(clock);
                return match self.what_to_do(clock) {
                    TestControllerAction::Nothing => Response::none(),
                    TestControllerAction::NoteOn => {
                        // This is elegant, I hope. If the arpeggiator is
                        // disabled during play, and we were playing a note,
                        // then we still send the off note,
                        if self.is_enabled {
                            self.is_playing = true;
                            Response::single(Self::Message::Midi(
                                self.midi_channel_out,
                                MidiMessage::NoteOn {
                                    key: 60.into(),
                                    vel: 127.into(),
                                },
                            ))
                        } else {
                            Response::none()
                        }
                    }
                    TestControllerAction::NoteOff => {
                        if self.is_playing {
                            Response::single(Self::Message::Midi(
                                self.midi_channel_out,
                                MidiMessage::NoteOff {
                                    key: 60.into(),
                                    vel: 0.into(),
                                },
                            ))
                        } else {
                            Response::none()
                        }
                    }
                };
            }
            Self::Message::Enable(enabled) => {
                self.is_enabled = enabled;
            }
            #[allow(unused_variables)]
            Self::Message::Midi(channel, message) => {
                //dbg!(&channel, &message);
            }
            _ => todo!(),
        }
        Response::none()
    }
}

impl Updateable for TestEffect<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, _clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::UpdateF32(param_id, value) => {
                if let Some(param) = TestEffectControlParams::from_repr(param_id) {
                    match param {
                        TestEffectControlParams::MyValue => self.set_my_value(value),
                    }
                }
            }
            _ => todo!(),
        }
        Response::none()
    }

    fn param_id_for_name(&self, param_name: &str) -> usize {
        if let Ok(param) = TestEffectControlParams::from_str(param_name) {
            param as usize
        } else {
            usize::MAX
        }
    }
}

impl Updateable for TestInstrument<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::UpdateF32(param_id, value) => {
                if let Some(param) = TestInstrumentControlParams::from_repr(param_id) {
                    match param {
                        TestInstrumentControlParams::Waveform => self.set_waveform(value),
                        TestInstrumentControlParams::FakeValue => self.set_fake_value(value),
                    }
                }
            }
            Self::Message::Midi(channel, message) => {
                self.handle_midi(clock, channel, message);
            }
            _ => todo!(),
        }
        Response::none()
    }
}

#[cfg(test)]
pub mod tests {
    use super::SourcesAudio;
    use super::TestInstrument;
    use crate::clock::Clock;
    use crate::messages::tests::TestMessage;
    use crate::messages::MessageBounds;
    use rand::random;

    // TODO: restore tests that test basic trait behavior, then figure out how
    // to run everyone implementing those traits through that behavior. For now,
    // this one just tests that a generic instrument doesn't panic when accessed
    // for non-consecutive time slices.
    #[test]
    fn test_sources_audio_random_access() {
        let mut instrument = TestInstrument::<TestMessage>::default();
        for _ in 0..100 {
            let mut clock = Clock::default();
            clock.set_samples(random());
            let _ = instrument.source_audio(&clock);
        }
    }

    impl<M: MessageBounds> TestInstrument<M> {
        pub fn dump_messages(&self) {
            dbg!(&self.debug_messages);
        }
    }
}
