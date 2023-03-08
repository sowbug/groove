// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [engine] module contains the main `Subscription` interface between the
//! engine and the app that is embedding it.

use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::{Clock, TimeSignature},
    traits::Resets,
    util::Paths,
    ParameterType, StereoSample,
};
use groove_orchestration::{
    helpers::{AudioOutput, IOHelper},
    messages::{GrooveEvent, GrooveInput, Internal, Response},
    Orchestrator,
};
use groove_settings::SongSettings;
use iced::futures::channel::mpsc;
use iced_native::subscription::{self, Subscription};
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND};

enum State {
    Start,
    Ready(JoinHandle<()>, mpsc::Receiver<EngineEvent>),
    Ending(JoinHandle<()>),
    Idle,
}

/// A GrooveInput is a kind of message that acts as input from the subscriber
/// (the app) to the subscription publisher (the Groove engine).
#[derive(Clone, Debug)]
pub enum EngineInput {
    /// Load the project at the given file path.
    LoadProject(String),

    /// Start playing at current cursor.
    Play,

    /// Stop playing.
    Pause,

    /// Reset the cursor to time zero.
    SkipToStart,

    /// Handle this incoming MIDI message from external.
    Midi(MidiChannel, MidiMessage),

    /// Change BPM.
    SetBpm(ParameterType),

    /// Change time signature.
    SetTimeSignature(TimeSignature),

    /// End this thread.
    QuitRequested,
}

/// A GrooveEvent is a kind of message that lets the subscriber (the app) know
/// something happened with the subscription publisher (the Groove engine). We
/// could have also called it a GrooveOutput, but other examples of Iced
/// subscriptions use the term "event," so we're going with that.
#[derive(Clone, Debug)]
pub enum EngineEvent {
    Ready(mpsc::Sender<EngineInput>, Arc<Mutex<Orchestrator>>),
    SetClock(usize),
    SetBpm(ParameterType),
    SetTimeSignature(TimeSignature),
    MidiToExternal(MidiChannel, MidiMessage),
    ProjectLoaded(String, Option<String>),
    AudioOutput(StereoSample),
    OutputComplete,
    Quit,
}

/// Runner is the glue between Groove (the audio engine) and the Iced
/// Subscription interface. It takes input/output going over the MPSC channels
/// and converts them to work with Groove. It's also the thing that knows that
/// Groove is running in a separate thread, so it manages the Arc<Mutex<>> that
/// lets app messages arrive asynchronously.
///
/// Runner also spins up AudioOutput, which is another thread. This might make
/// more sense as its own subscription, so that the app can arrange for
/// GrooveSubscription audio output to be routed to the audio system. For now
/// it's not causing any trouble.
struct Runner {
    orchestrator: Arc<Mutex<Orchestrator>>,
    clock: Clock,
    time_signature: TimeSignature,
    last_clock_update: Instant,

    events: Vec<GrooveEvent>,
    sender: mpsc::Sender<EngineEvent>,
    receiver: mpsc::Receiver<EngineInput>,
    audio_output: Option<AudioOutput>,

    buffer_target: usize,
}
impl Runner {
    pub fn new_with(
        orchestrator: Arc<Mutex<Orchestrator>>,
        clock: Clock,
        sender: mpsc::Sender<EngineEvent>,
        receiver: mpsc::Receiver<EngineInput>,
    ) -> Self {
        Self {
            orchestrator,
            clock,
            time_signature: TimeSignature { top: 4, bottom: 4 }, // TODO: what's a good "don't know yet" value?
            last_clock_update: Instant::now(),
            events: Default::default(),
            sender,
            receiver,
            audio_output: None,

            buffer_target: 2048,
        }
    }

    fn push_response(&mut self, response: Response<GrooveEvent>) {
        match response.0 {
            Internal::None => {}
            Internal::Single(message) => {
                self.events.push(message);
            }
            Internal::Batch(messages) => {
                self.events.extend(messages);
            }
        }
    }

    fn post_event(&mut self, event: EngineEvent) {
        let _ = self.sender.try_send(event);
    }

    /// Processes any queued-up messages that we can handle, and sends what's
    /// left to the app.
    ///
    /// Returns an audio sample if found, and returns true if the orchestrator
    /// has indicated that it's done with its work.
    fn handle_pending_messages(&mut self) -> (StereoSample, bool) {
        let mut sample = StereoSample::default();
        let mut done = false;
        while let Some(event) = self.events.pop() {
            match event {
                GrooveEvent::AudioOutput(output_sample) => sample = output_sample,
                GrooveEvent::OutputComplete => {
                    done = true;
                    self.post_event(EngineEvent::OutputComplete);
                }
                GrooveEvent::MidiToExternal(channel, message) => {
                    self.post_event(EngineEvent::MidiToExternal(channel, message))
                }
                GrooveEvent::LoadedProject(filename, title) => {
                    self.post_event(EngineEvent::ProjectLoaded(filename, title))
                }
                GrooveEvent::EntityMessage(_, _) => {
                    panic!("this should have been handled by now")
                }
            }
        }
        (sample, done)
    }

    fn dispatch_samples(&mut self, samples: &[StereoSample], sample_count: usize) {
        if let Some(output) = self.audio_output.as_mut() {
            for (i, sample) in samples.iter().enumerate() {
                if i < sample_count {
                    let _ = output.push(*sample);
                } else {
                    break;
                }
            }
        }
    }

    pub fn do_loop(&mut self) {
        let mut samples = [StereoSample::SILENCE; 64];
        let mut is_playing = false;
        loop {
            self.publish_clock_update();

            // Handle any received messages before asking Orchestrator to handle
            // Tick.
            let mut messages = Vec::new();
            while let Ok(Some(input)) = self.receiver.try_next() {
                match input {
                    // TODO: many of these are in the wrong place. This loop
                    // should be tight and dumb.
                    EngineInput::LoadProject(filename) => {
                        self.clock.reset(self.clock.sample_rate());
                        is_playing = false;
                        self.load_project(filename);
                    }
                    EngineInput::Play => is_playing = true,
                    EngineInput::Pause => is_playing = false,
                    EngineInput::SkipToStart => {
                        self.clock.reset(self.clock.sample_rate());
                    }
                    EngineInput::Midi(channel, message) => {
                        messages.push(GrooveInput::MidiFromExternal(channel, message))
                    }
                    EngineInput::QuitRequested => break,
                    EngineInput::SetBpm(bpm) => {
                        if bpm != self.clock.bpm() {
                            self.clock.set_bpm(bpm);
                            self.publish_bpm_update();
                        }
                    }
                    EngineInput::SetTimeSignature(time_signature) => {
                        if time_signature != self.time_signature {
                            self.time_signature = time_signature;
                            self.publish_time_signature_update();
                        }
                    }
                }
            }

            // Forward any messages that were meant for Orchestrator. Any
            // responses we get at this point are to messages that aren't Tick,
            // so we can ignore the return values from send_pending_messages().
            while let Some(message) = messages.pop() {
                let response = if let Ok(mut o) = self.orchestrator.lock() {
                    o.update(message)
                } else {
                    Response::none()
                };
                self.push_response(response);
            }
            let (_, _) = self.handle_pending_messages();

            if is_playing {
                let ticks_completed = if let Ok(mut o) = self.orchestrator.lock() {
                    o.tick(&mut samples)
                } else {
                    0
                };
                if ticks_completed < samples.len() {
                    is_playing = false;
                }

                // This clock is used to tell the app where we are in the song,
                // so even though it looks like it's not helping here in the
                // loop, it's necessary. We have it before the second is_playing
                // test because the tick() that returns false still produced
                // some samples, so we want the clock to reflect that.
                self.clock.tick_batch(ticks_completed);

                // TODO: this might cut off the end of the buffer if it doesn't
                // end on a 64-sample boundary. Would make sense to change
                // tick() to return the number of samples it was able to fill,
                // and then propagate that number through to dispatch_samples()
                // et seq.
                if is_playing {
                    self.dispatch_samples(&samples, ticks_completed);
                    self.wait_for_audio_buffer();
                }
            }
        }
    }

    /// Periodically sends out an event telling the app what time we think it
    /// is.
    fn publish_clock_update(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_clock_update).as_millis() > 15 {
            self.post_event(EngineEvent::SetClock(self.clock.frames()));
            self.last_clock_update = now;
        }
    }

    fn publish_bpm_update(&mut self) {
        self.post_event(EngineEvent::SetBpm(self.clock.bpm()));
    }

    fn publish_time_signature_update(&mut self) {
        self.post_event(EngineEvent::SetTimeSignature(self.time_signature));
    }

    pub fn start_audio(&mut self) {
        let mut audio_output = AudioOutput::default();
        audio_output.start();
        self.audio_output = Some(audio_output);
    }

    pub fn stop_audio(&mut self) {
        if let Some(audio_output) = self.audio_output.as_mut() {
            audio_output.stop();
        }
        self.audio_output = None;
    }

    // TODO: visualize buffer
    fn wait_for_audio_buffer(&mut self) {
        if let Some(output) = self.audio_output.as_ref() {
            let buffer_len = output.buffer_len();
            if buffer_len < self.buffer_target / 4 {
                self.buffer_target *= 2;
                if self.buffer_target > 4096 {
                    self.buffer_target = 4096;
                }
            } else if buffer_len >= self.buffer_target * 2 {
                self.buffer_target *= 8;
                self.buffer_target /= 10;
            } else if buffer_len >= self.buffer_target {
                let mut time_to_sleep = 2;
                while output.buffer_len() >= self.buffer_target {
                    std::thread::sleep(Duration::from_micros(time_to_sleep));
                    time_to_sleep *= 2;
                }
            }
        }
    }

    fn load_project(&mut self, filename: String) -> Response<GrooveEvent> {
        let mut path = Paths::project_path();
        path.push(filename.clone());
        if let Ok(settings) = SongSettings::new_from_yaml_file(path.to_str().unwrap()) {
            if let Ok(instance) = settings.instantiate(false) {
                let title = instance.title();
                self.orchestrator = Arc::new(Mutex::new(instance)); // TODO: this can't be right, because the app doesn't know about the new one
                return Response::single(GrooveEvent::LoadedProject(filename, title));
            }
        }
        return Response::none();
    }
}

/// GrooveSubscription is the Iced Subscription for the Groove engine. It
/// creates the MPSC channels and spawns the Orchestrator/Runner in a thread. It
/// also knows how to signal the thread to quit when it's time.
pub struct EngineSubscription {}
impl EngineSubscription {
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
                        let (app_sender, app_receiver) = mpsc::channel::<EngineInput>(1024);

                        // This channel surfaces event messages from
                        // Runner/Orchestrator as subscription events.
                        let (thread_sender, thread_receiver) = mpsc::channel::<EngineEvent>(1024);

                        // TODO: deal with output-device and sample-rate
                        // changes. This is a mess.
                        let sample_rate = IOHelper::get_output_device_sample_rate();
                        let t = Orchestrator::new_with(sample_rate, DEFAULT_BPM);
                        let orchestrator = Arc::new(Mutex::new(t));
                        let orchestrator_for_app = Arc::clone(&orchestrator);
                        let handler = std::thread::spawn(move || {
                            let mut runner = Runner::new_with(
                                orchestrator,
                                Clock::new_with(
                                    sample_rate,
                                    DEFAULT_BPM,
                                    DEFAULT_MIDI_TICKS_PER_SECOND,
                                ),
                                thread_sender,
                                app_receiver,
                            );
                            runner.start_audio();
                            runner.do_loop();
                            runner.stop_audio();
                        });

                        (
                            Some(EngineEvent::Ready(app_sender, orchestrator_for_app)),
                            State::Ready(handler, thread_receiver),
                        )
                    }
                    State::Ready(handler, mut receiver) => {
                        use iced_native::futures::StreamExt;

                        let groove_event = receiver.select_next_some().await;
                        if let EngineEvent::Quit = groove_event {
                            (Some(EngineEvent::Quit), State::Ending(handler))
                        } else {
                            (Some(groove_event), State::Ready(handler, receiver))
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
}
