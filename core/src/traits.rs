// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use crate::midi::HandlesMidi;

use crate::{
    midi::u7,
    time::{MusicalTime, PerfectTimeUnit},
    Normal, Sample, StereoSample,
};
use std::ops::Range;

pub trait MessageBounds: std::fmt::Debug + Send {}

/// An [IsController] controls things in the system that implement
/// [Controllable]. Examples are sequencers, arpeggiators, and discrete LFOs (as
/// contrasted with LFOs that are integrated into other instruments).
///
/// [IsController] emits messages, either control messages that the system
/// routes to [Controllable]s, or MIDI messages that go over the MIDI bus.
///
/// An [IsController] is the only kind of entity that can "finish." An
/// [IsEffect] or [IsInstrument] can't finish; they wait forever for audio to
/// process, or MIDI commands to handle. A performance ends once all
/// [IsController] entities indicate that they've finished.
pub trait IsController:
    Controls + HandlesMidi + Performs + HasUid + Send + std::fmt::Debug
{
}

/// An [IsEffect] transforms audio. It takes audio inputs and produces audio
/// output. It does not get called unless there is audio input to provide to it
/// (which can include silence, e.g., in the case of a muted instrument).
pub trait IsEffect:
    TransformsAudio + Controllable + Resets + HasUid + Send + std::fmt::Debug
{
}

/// An [IsInstrument] produces audio, usually upon request from MIDI or
/// [IsController] input.
pub trait IsInstrument:
    Generates<StereoSample> + Ticks + HandlesMidi + Controllable + HasUid + Send + std::fmt::Debug
{
}

/// Something that [Generates] creates the given type as its work product over
/// time. Examples are envelopes, which produce a [Normal] signal, and
/// oscillators, which produce a [BipolarNormal] signal.
pub trait Generates<V>: Send + std::fmt::Debug + Ticks {
    /// The value for the current frame. Advance the frame by calling
    /// Ticks::tick().
    fn value(&self) -> V;

    /// The batch version of value(). To deliver each value, this method will
    /// typically call tick() internally. If you don't want this, then call
    /// value() on your own.
    fn batch_values(&mut self, values: &mut [V]);
}

/// Something that is [Controllable] exposes a set of attributes, each with a text
/// name, that an [IsController] can change. If you're familiar with DAWs, this is
/// typically called automation.
///
/// The [Controllable] trait is more powerful than ordinary getters/setters
/// because it allows runtime binding of an [IsController] to a [Controllable].
#[allow(unused_variables)]
pub trait Controllable {
    // See https://stackoverflow.com/a/71988904/344467 to show that we could
    // have made these functions rather than methods (no self). But then we'd
    // lose the ability to query an object without knowing its struct, which is
    // important for the loose binding that the automation system provides.
    fn control_index_count(&self) -> usize {
        unimplemented!()
    }
    fn control_index_for_name(&self, name: &str) -> Option<usize> {
        unimplemented!("Controllable trait methods are implemented by a macro")
    }
    fn control_name_for_index(&self, index: usize) -> Option<String> {
        unimplemented!()
    }
    fn control_set_param_by_name(&mut self, name: &str, value: crate::control::F32ControlValue) {
        unimplemented!()
    }
    fn control_set_param_by_index(&mut self, index: usize, value: crate::control::F32ControlValue) {
        unimplemented!()
    }
}

/// A HasUid has an ephemeral but globally unique numeric identifier, which is
/// useful for one entity to refer to another without getting into icky Rust
/// ownership questions. It's the foundation of any ECS
/// (entity/component/system) design. We're not using any ECS, but our uids work
/// similarly to how they do in an ECS.
pub trait HasUid {
    fn uid(&self) -> usize;
    fn set_uid(&mut self, uid: usize);
    fn name(&self) -> &'static str;
}

/// Something that Resets also either [Ticks] or [TicksWithMessages]. Since the
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

pub trait Controls: Resets + Send + std::fmt::Debug {
    type Message;

    #[allow(unused_variables)]
    fn update_time(&mut self, range: &Range<MusicalTime>);

    /// The entity should perform work for the time range specified in the
    /// previous update_time().
    ///
    /// Returns zero or more messages.
    ///
    /// Returns the number of requested ticks handled before terminating.
    fn work(&mut self) -> Option<Vec<Self::Message>>;

    /// Returns true if the entity is done with all its scheduled work. An
    /// entity that performs work only on command should always return true, as
    /// the framework ends the piece being performed only when all entities
    /// implementing [Controls] indicate that they're finished.
    fn is_finished(&self) -> bool;
}

/// A [TransformsAudio] takes input audio, which is typically produced by
/// [SourcesAudio], does something to it, and then outputs it. It's what effects
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

/// Describes the public interface of an envelope generator, which provides a
/// normalized amplitude (0.0..=1.0) that changes over time according to its
/// internal parameters, external triggers, and the progression of time.
pub trait GeneratesEnvelope: Generates<Normal> + Send + std::fmt::Debug + Ticks {
    /// Triggers the envelope's active stage.
    fn trigger_attack(&mut self);

    /// Triggers the end of the envelope's active stage.
    fn trigger_release(&mut self);

    /// Requests a fast decrease to zero amplitude. Upon reaching zero, switches
    /// to idle. If the EG is already idle, then does nothing. For normal EGs,
    /// the EG's settings (ADSR, etc.) don't affect the rate of shutdown decay.
    ///
    /// See DSSPC, 4.5 Voice Stealing, for an understanding of how the shutdown
    /// state helps. TL;DR: if we have to steal one voice to play a different
    /// note, it sounds better if the voice very briefly stops and restarts.
    fn trigger_shutdown(&mut self);

    /// Whether the envelope generator is in the idle state, which usually means
    /// quiescent and zero amplitude.
    fn is_idle(&self) -> bool;
}

/// A [PlaysNotes] turns note events into sound. It seems to overlap with
/// [HandlesMidi]; the reason it exists is to allow the two interfaces to evolve
/// independently, because MIDI is unlikely to be perfect for all our needs.
pub trait PlaysNotes {
    /// Whether the entity is currently making sound.
    fn is_playing(&self) -> bool;

    /// Initiates a note-on event. Depending on implementation, might initiate a
    /// steal (tell envelope to go to shutdown state, then do note-on when
    /// that's done).
    fn note_on(&mut self, key: u8, velocity: u8);

    /// Initiates an aftertouch event.
    fn aftertouch(&mut self, velocity: u8);

    /// Initiates a note-off event, which can take a long time to complete,
    /// depending on how long the envelope's release is.
    fn note_off(&mut self, velocity: u8);
}

/// A [StoresVoices] provides access to a collection of voices for a polyphonic
/// synthesizer. Different implementers provide different policies for how to
/// handle voice-stealing.
pub trait StoresVoices: Generates<StereoSample> + Send + std::fmt::Debug {
    type Voice;

    /// Generally, this value won't change after initialization, because we try
    /// not to dynamically allocate new voices.
    fn voice_count(&self) -> usize;

    /// The number of voices reporting is_playing() true.
    fn active_voice_count(&self) -> usize;

    /// Fails if we run out of idle voices and can't steal any active ones.
    fn get_voice(&mut self, key: &u7) -> anyhow::Result<&mut Box<Self::Voice>>;

    /// All the voices.
    // Thanks to https://stackoverflow.com/a/58612273/344467 for the lifetime magic
    fn voices<'a>(&'a self) -> Box<dyn Iterator<Item = &Box<Self::Voice>> + 'a>;

    /// All the voices as a mutable iterator.
    fn voices_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut Box<Self::Voice>> + 'a>;
}

/// A device that [Performs] has a concept of a performance that has a beginning
/// and an end, and it knows how to respond to requests to start, stop, restart,
/// and seek within the performance.
pub trait Performs {
    /// Tells the device to play its performance from the current location.
    fn play(&mut self);

    /// Tells the device to stop playing its performance. It shouldn't change
    /// its cursor location, so that a play() after a stop() acts like a resume.
    fn stop(&mut self);

    /// Resets cursors to the beginning. This is set_cursor Lite (TODO).
    fn skip_to_start(&mut self);

    /// Sets the loop range. Parents should propagate to children. We provide a
    /// default implementation for this set of methods because looping doesn't
    /// apply to many devices.
    fn set_loop(&mut self, range: &Range<PerfectTimeUnit>) {}

    /// Clears the loop range, restoring normal cursor behavior.
    fn clear_loop(&mut self) {}

    /// Enables or disables loop behavior. When looping is enabled, if the
    /// cursor is outside the range on the right side (i.e., after), it sets
    /// itself to the start point.
    fn set_loop_enabled(&mut self, is_enabled: bool) {}

    /// Whether the device is currently playing. This is part of the trait so
    /// that implementers don't have to leak their internal state to unit test
    /// code.
    fn is_performing(&self) -> bool;
}

/// A synthesizer is composed of Voices. Ideally, a synth will know how to
/// construct Voices, and then handle all the MIDI events properly for them.
pub trait IsVoice<V>: Generates<V> + PlaysNotes + Send {}
pub trait IsStereoSampleVoice: IsVoice<StereoSample> {}

#[cfg(feature = "egui-framework")]
pub mod gui {
    use eframe::egui::Ui;

    /// Implements egui content inside a Window or SidePanel.
    pub trait Shows {
        fn show(&mut self, ui: &mut Ui);
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::Ticks;

    pub trait DebugTicks: Ticks {
        fn debug_tick_until(&mut self, tick_number: usize);
    }
}
