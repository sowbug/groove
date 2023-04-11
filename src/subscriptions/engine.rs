// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [engine](crate::subscriptions::engine) module contains the
//! [Subscription](iced_native::subscription::Subscription) interface between
//! the [Groove](groove_core::Groove) engine and the app subscribing to it.

use crate::audio::AudioOutput;
use crossbeam::queue::ArrayQueue;
use groove_core::{Normal, StereoSample};
use groove_orchestration::{helpers::IOHelper, messages::GrooveInput};
use iced::futures::channel::mpsc as iced_mpsc;
use iced_native::subscription::{self, Subscription};
use std::{
    sync::{mpsc, Arc},
    thread::JoinHandle,
    time::Instant,
};

// TODO: this is waaaaaaaay more complicated than it needs to be. I'm evolving
// it from a separate thread that was running the entire engine to a simple
// interface around AudioOutput. Simplify!

enum State {
    Start,
    Ready(JoinHandle<()>, iced_mpsc::Receiver<EngineEvent>),
    Ending(JoinHandle<()>),
    Idle,
}

/// The subscriber sends [EngineInput] messages to communicate with the engine.
#[derive(Debug)]
pub enum EngineInput {
    /// Change sample rate.
    SetSampleRate(usize),

    /// Start the audio interface. After this point, it's important to respond
    /// quickly to GenerateAudio events, because the audio interface will be
    /// consuming audio samples from the ring buffer.
    ///
    /// Note that this service starts in the StartAudio state, so there's no
    /// need to send it unless you've paused audio and want to resume.
    StartAudio,

    /// Pause the audio interface. Send this when you know you won't have any
    /// audio to send for a while. Resume with StartAudio.
    PauseAudio,

    /// End this thread.
    QuitRequested,
}

/// The engine sends [EngineEvent] messages to subscribers whenever interesting
/// things happen.
#[derive(Clone, Debug)]
pub enum EngineEvent {
    /// This is the first event that the engine sends to subscribers. It gives a
    /// channel to send [EngineInput] back to the engine, and an ArrayQueue that
    /// the subscriber can use to push audio to the audio engine.
    Ready(mpsc::Sender<EngineInput>, Arc<ArrayQueue<StereoSample>>),

    /// The audio interface needs one or more buffers of audio.
    GenerateAudio(u8),

    /// The sample rate has changed.
    SampleRateChanged(usize),

    /// How full our audio output buffer is, as a percentage.
    /// TODO: this MAYBE should be part of GrooveEvent
    AudioBufferFullness(Normal),

    /// The engine has received an [EngineInput::QuitRequested] message, has
    /// successfully processed it, and is now ready for its subscription to end.
    Quit,
}

/// [EngineSubscription] is the glue between the audio engine and the Iced
/// [Subscription] interface.
///
/// [EngineSubscription] also spins up [AudioOutput] in its own thread. This
/// might make more sense as its own subscription, so that the app can arrange
/// for [EngineSubscription] audio output to be routed to the audio system. But
/// for now, it's not causing any trouble, so we're keeping it where it is.
pub struct EngineSubscription {
    last_clock_update: Instant,
    sample_rate: usize,
    has_broadcast_sample_rate: bool,

    sender: iced_mpsc::Sender<EngineEvent>,
    receiver: mpsc::Receiver<EngineInput>,
    audio_output: AudioOutput,
}
impl EngineSubscription {
    /// The size of a single unit of engine work. At 44.1KHz, this corresponds
    /// to 1.45 milliseconds of audio. Higher means less overhead per sample,
    /// and lower means two things: first, less latency because buffers aren't
    /// sent to the output device until they're complete; and second, more
    /// precise automation because we aggregate all MIDI and control events for
    /// a single buffer, applying them all at the start of the buffer rather
    /// than the exact time slice when they're scheduled.
    ///
    /// This should eventually be adjustable. For a live performance, being a
    /// millisecond or two off isn't a big deal. But for a final rendering of an
    /// audio track, where it's OK if the PC can't keep up real-time, and very
    /// precise event timing is desirable, it makes more sense to choose a
    /// buffer size of 1 sample.
    pub const ENGINE_BUFFER_SIZE: usize = 64;

    /// Starts the subscription. The first message sent with the subscription
    /// will be [EngineEvent::Ready].
    pub fn subscription() -> Subscription<EngineEvent> {
        subscription::unfold(
            std::any::TypeId::of::<EngineSubscription>(),
            State::Start,
            |state| async move {
                match state {
                    State::Start => {
                        // This channel lets the app send us messages.
                        //
                        // TODO: what's the right number for the buffer size?
                        let (app_sender, app_receiver) = mpsc::channel::<EngineInput>();

                        // This channel surfaces engine event messages as
                        // subscription events.
                        let (thread_sender, thread_receiver) =
                            iced_mpsc::channel::<EngineEvent>(1024);

                        let event_sender = thread_sender.clone();

                        // TODO: deal with output-device and sample-rate
                        // changes. This is a mess.
                        let ring_buffer = AudioOutput::create_ring_buffer();
                        let rb2 = Arc::clone(&ring_buffer);
                        let sample_rate = IOHelper::get_output_device_sample_rate();
                        let handler = std::thread::spawn(move || {
                            let audio_output =
                                AudioOutput::new_with(ring_buffer, event_sender.clone());
                            let mut subscription = Self::new_with(
                                sample_rate,
                                thread_sender,
                                app_receiver,
                                audio_output,
                            );
                            subscription.update_and_broadcast_sample_rate(sample_rate);
                            subscription.start_audio();
                            subscription.do_loop();
                            subscription.stop_audio();
                        });

                        (
                            Some(EngineEvent::Ready(app_sender, rb2)),
                            State::Ready(handler, thread_receiver),
                        )
                    }
                    State::Ready(handler, mut receiver) => {
                        use iced_native::futures::StreamExt;

                        let engine_event = receiver.select_next_some().await;
                        if let EngineEvent::Quit = engine_event {
                            (Some(EngineEvent::Quit), State::Ending(handler))
                        } else {
                            (Some(engine_event), State::Ready(handler, receiver))
                        }
                    }
                    State::Ending(handler) => {
                        let _ = handler.join();
                        // See https://github.com/iced-rs/iced/issues/1348
                        (None, State::Idle)
                    }
                    State::Idle => {
                        // I took this line from
                        // https://github.com/iced-rs/iced/issues/336, but I
                        // don't understand why it helps. I think it's necessary
                        // for the system to get a chance to process all the
                        // subscription results.
                        let _: () = iced::futures::future::pending().await;
                        (None, State::Idle)
                    }
                }
            },
        )
    }

    fn new_with(
        sample_rate: usize,
        sender: iced_mpsc::Sender<EngineEvent>,
        receiver: mpsc::Receiver<EngineInput>,
        audio_output: AudioOutput,
    ) -> Self {
        Self {
            sample_rate,
            has_broadcast_sample_rate: false,
            last_clock_update: Instant::now(),
            sender,
            receiver,
            audio_output,
        }
    }

    fn post_event(&mut self, event: EngineEvent) {
        let _ = self.sender.try_send(event);
    }

    fn do_loop(&mut self) {
        let mut messages = Vec::new();
        loop {
            if let Ok(input) = self.receiver.recv() {
                self.publish_dashboard_updates();
                if !self.has_broadcast_sample_rate {
                    self.has_broadcast_sample_rate = true;
                    self.post_event(EngineEvent::SampleRateChanged(self.sample_rate));
                }

                match input {
                    EngineInput::SetSampleRate(sample_rate) => {
                        self.update_and_broadcast_sample_rate(sample_rate);
                        messages.push(GrooveInput::SetSampleRate(sample_rate));
                    }
                    EngineInput::QuitRequested => {
                        self.post_event(EngineEvent::Quit);
                        break;
                    }
                    EngineInput::StartAudio => self.start_audio(),
                    EngineInput::PauseAudio => self.pause_audio(),
                }
            } else {
                // In the normal case, we will break when we get the
                // QuitRequested message. This break catches the case where the
                // senders unexpectedly died.
                eprintln!("Unexpected termination of EngineInput senders");
                break;
            }
        }
    }

    fn update_and_broadcast_sample_rate(&mut self, sample_rate: usize) {
        // TODO: ask audio subsystem to change. Raises question who owns sample
        // rate -- user? Audio? -- and whether it's OK to have two different
        // rates, such as one for audio output, and one for rendering to WAV

        // Decide whether this is a UI request, or something else.

        self.sample_rate = sample_rate;
        self.post_event(EngineEvent::SampleRateChanged(sample_rate));
    }

    /// Periodically sends out events useful for GUI display.
    fn publish_dashboard_updates(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_clock_update).as_millis() > (1000 / 30) {
            // TODO: this is active only while the project is playing. I wanted
            // it whenever the app is open, but it caused 30% CPU usage,
            // probably because of app redraws.

            // TODO: this is actually always active.... fix
            self.post_event(EngineEvent::AudioBufferFullness(Normal::from(
                self.audio_output.buffer_utilization(),
            )));
        }
    }

    fn start_audio(&mut self) {
        self.audio_output.start();
    }

    fn pause_audio(&mut self) {
        self.audio_output.pause();
    }

    fn stop_audio(&mut self) {
        self.audio_output.stop();
    }
}
