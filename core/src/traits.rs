// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{control::F32ControlValue, midi::HandlesMidi, Sample, StereoSample};

/// An IsController controls things in the system that implement Controllable.
/// Examples are sequencers, arpeggiators, and discrete LFOs (as contrasted with
/// LFOs that are integrated into other instruments).
///
/// An IsController implements Terminates, which indicates that it's done
/// emitting events (and, in the case of timers and sequencers, done waiting for
/// other work in the system to complete).
///
/// An IsController necessarily implements TicksWithMessages, rather than just
/// Ticks, because messages are how controllers control other things in the
/// system.
pub trait IsController<V>:
    TicksWithMessages<V> + HandlesMidi + HasUid + Send + std::fmt::Debug
{
}

/// An IsEffect transforms audio. It takes audio inputs and produces audio
/// output. It does not get called unless there is audio input to provide to it
/// (which can include silence, e.g., in the case of a muted instrument).
///
/// IsEffects don't implement Terminates. They process audio indefinitely, and
/// don't have a sense of the length of the performance.
pub trait IsEffect: TransformsAudio + Controllable + HasUid + Send + std::fmt::Debug {}

/// An IsInstrument produces audio, usually upon request from MIDI or
/// IsController input. Like IsEffect, IsInstrument doesn't implement Terminates
/// because it continues to create audio as long as asked.
pub trait IsInstrument:
    Generates<StereoSample> + Ticks + HandlesMidi + Controllable + HasUid + Send + std::fmt::Debug
{
}

pub trait Generates<V>: Send + std::fmt::Debug + Ticks {
    /// The value for the current frame. Advance the frame by calling
    /// Ticks::tick().
    fn value(&self) -> V;

    /// The batch version of value(). To deliver each value, this method will
    /// typically call tick() internally. If you don't want this, then call
    /// value() on your own.
    fn batch_values(&mut self, values: &mut [V]);
}

/// Something that is Controllable exposes a set of attributes, each with a text
/// name, that IsControllers can change. If you're familiar with DAWs, this is
/// typically called "automation."
///
/// The Controllable trait is more powerful than ordinary getters/setters
/// because it allows runtime binding of an IsController to a Controllable.
pub trait Controllable {
    #[allow(unused_variables)]
    fn control_index_for_name(&self, name: &str) -> usize {
        unimplemented!()
    }
    #[allow(unused_variables)]
    fn set_by_control_index(&mut self, index: usize, value: F32ControlValue) {
        unimplemented!()
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

/// Something that Resets also either Ticks or TicksWithMessages. Since the
/// Ticks family of traits don't get access to a global clock, they have to
/// maintain internal clocks and trust that they'll be asked to tick exactly the
/// same number of times as everyone else in the system. Resets::reset() ensures
/// that everyone starts from the beginning at the same time, and that everyone
/// agrees how long a tick lasts.
///
/// Sometimes we'll refer to a tick's "time slice" or "frame." These all mean
/// the same thing.
pub trait Resets {
    /// The entity should reset its internal state.
    ///
    /// The system will call reset() when the global sample rate changes, and
    /// whenever the global clock is reset. Since most entities that care about
    /// sample rate need to know it during construction, the system *won't* call
    /// reset() on entity construction; entities can require the sample rate as
    /// part of their new() functions, and if desired call reset() within that
    /// function.
    #[allow(unused_variables)]
    fn reset(&mut self, sample_rate: usize) {}
}

pub trait Ticks: Resets + Send + std::fmt::Debug {
    /// The entity should perform work for the current frame or frames. Under
    /// normal circumstances, successive tick()s represent successive frames.
    /// Exceptions include, for example, restarting a performance, which would
    /// reset the global clock, which the entity learns about via reset().
    ///
    /// Entities are responsible for tracking their own notion of time, which
    /// they should update during tick().
    ///
    /// tick() guarantees that any state for the current frame is valid *after*
    /// tick() has been called for the current frame. This means that Ticks
    /// implementers must treat the first frame as special. Normally, entity
    /// state is correct for the first frame after entity construction, so
    /// tick() must be careful not to update state on the first frame, because
    /// that would cause the state to represent the second frame, not the first.
    fn tick(&mut self, tick_count: usize);
}

pub trait TicksWithMessages<V>: Resets + Send + std::fmt::Debug {
    type Message;

    /// Similar to Ticks::tick().
    ///
    /// Returns zero or more EntityMessages.
    ///
    /// Returns the number of requested ticks handled before terminating.
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<V>>, usize);
}

/// A TransformsAudio takes input audio, which is typically produced by
/// SourcesAudio, does something to it, and then outputs it. It's what effects
/// do.
pub trait TransformsAudio: std::fmt::Debug {
    fn transform_audio(&mut self, input_sample: StereoSample) -> StereoSample {
        // Beware: converting from mono to stereo isn't just doing the work
        // twice! You'll also have to double whatever state you maintain from
        // tick to tick that has to do with a single channel's audio data.
        StereoSample(
            self.transform_channel(0, input_sample.0),
            self.transform_channel(1, input_sample.1),
        )
    }

    /// channel: 0 is left, 1 is right. Use the value as an index into arrays.
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample;
}
