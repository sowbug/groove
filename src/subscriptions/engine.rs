// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [engine](crate::subscriptions::engine) module contains the
//! [Subscription](iced_native::subscription::Subscription) interface between
//! the [Groove](groove_core::Groove) engine and the app subscribing to it.

use crate::{
    audio::AudioOutput,
    util::{PathType, Paths},
    DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND,
};
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::{Clock, TimeSignature},
    traits::Resets,
    Normal, ParameterType, StereoSample,
};
use groove_orchestration::{
    helpers::IOHelper,
    messages::{GrooveEvent, GrooveInput, Internal, Response},
    Orchestrator,
};
use groove_settings::SongSettings;
use iced::futures::channel::mpsc as iced_mpsc;
use iced_native::subscription::{self, Subscription};
use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::Instant,
};

enum State {
    Start,
    Ready(JoinHandle<()>, iced_mpsc::Receiver<EngineEvent>),
    Ending(JoinHandle<()>),
    Idle,
}

/// The subscriber sends [EngineInput] messages to communicate with the engine.
#[derive(Clone, Debug)]
pub enum EngineInput {
    /// The consumer of the audio engine's output is ready to handle more audio,
    /// and it requests the given number of buffers. (The size of a buffer is
    /// currently known by everyone, TODO to make that more explicit.)
    GenerateAudio(u8),

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

    /// Connect an IsController to a Controllable's control point. First
    /// argument is controller uid, second is controllable uid, third is
    /// controllable's control index.
    ConnectController(usize, usize, usize),

    /// End this thread.
    QuitRequested,
}

/// The engine sends [EngineEvent] messages to subscribers whenever interesting
/// things happen.
#[derive(Clone, Debug)]
pub enum EngineEvent {
    /// This is the first event that the engine sends to subscribers. It gives a
    /// channel to send [EngineInput] back to the engine, and an [Orchestrator]
    /// reference that's necessary for building GUI views.
    Ready(mpsc::Sender<EngineInput>, Arc<Mutex<Orchestrator>>),

    /// Sends the engine's current frame. Useful for the GUI to keep the control
    /// bar's clock in sync.
    SetClock(usize),

    /// Sends an updated BPM (beats per minute) whenever it changes.
    SetBpm(ParameterType),

    /// Sends an updated global time signature whenever it changes. Note that
    /// individual components might have independent time signatures that
    /// operate on their own time.
    SetTimeSignature(TimeSignature),

    /// The engine has generated a MIDI message suitable for forwarding to
    /// external MIDI hardware.
    MidiToExternal(MidiChannel, MidiMessage),

    /// A new project has loaded. For convenience, the filename and optional
    /// project title are included.
    ProjectLoaded(String, Option<String>),

    /// The engine has produced a frame of audio.
    AudioOutput(StereoSample),

    /// The current performance is complete.
    OutputComplete,

    /// How full our audio output buffer is, as a percentage.
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
    // Orchestrator is wrapped in a mutex because we've chosen to give the app
    // direct access to it while building the GUI view.
    orchestrator: Arc<Mutex<Orchestrator>>,
    clock: Clock,
    time_signature: TimeSignature,
    last_clock_update: Instant,
    last_reported_frames: usize,
    is_playing: bool,

    events: Vec<GrooveEvent>,
    sender: iced_mpsc::Sender<EngineEvent>,
    receiver: mpsc::Receiver<EngineInput>,
    audio_output: AudioOutput,

    yes: f64,
    check: f64,
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

                        let input_sender = app_sender.clone();

                        // TODO: deal with output-device and sample-rate
                        // changes. This is a mess.
                        let sample_rate = IOHelper::get_output_device_sample_rate();
                        let t = Orchestrator::new_with(sample_rate, DEFAULT_BPM);
                        let orchestrator = Arc::new(Mutex::new(t));
                        let orchestrator_for_app = Arc::clone(&orchestrator);
                        let handler = std::thread::spawn(move || {
                            let audio_output = AudioOutput::new_with(input_sender.clone());
                            let mut subscription = Self::new_with(
                                orchestrator,
                                Clock::new_with(
                                    sample_rate,
                                    DEFAULT_BPM,
                                    DEFAULT_MIDI_TICKS_PER_SECOND,
                                ),
                                thread_sender,
                                app_receiver,
                                audio_output,
                            );
                            subscription.start_audio();
                            subscription.do_loop();
                            subscription.stop_audio();
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

    fn new_with(
        orchestrator: Arc<Mutex<Orchestrator>>,
        clock: Clock,
        sender: iced_mpsc::Sender<EngineEvent>,
        receiver: mpsc::Receiver<EngineInput>,
        audio_output: AudioOutput,
    ) -> Self {
        Self {
            orchestrator,
            clock,
            time_signature: TimeSignature { top: 4, bottom: 4 }, // TODO: what's a good "don't know yet" value?
            last_clock_update: Instant::now(),
            last_reported_frames: usize::MAX,
            is_playing: Default::default(),
            events: Default::default(),
            sender,
            receiver,
            audio_output,

            yes: 0.0,
            check: 0.0,
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
                GrooveEvent::ProjectLoaded(filename, title) => {
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
        let _ = self.audio_output.push_buffer(&samples[0..sample_count]);
    }

    fn do_loop(&mut self) {
        loop {
            let mut messages = Vec::new();
            if let Ok(input) = self.receiver.recv() {
                self.publish_dashboard_updates();

                match input {
                    EngineInput::GenerateAudio(buffer_count) => self.generate_audio(buffer_count),
                    // TODO: many of these are in the wrong place. This loop
                    // should be tight and dumb.
                    EngineInput::LoadProject(filename) => {
                        self.clock.reset(self.clock.sample_rate());
                        self.is_playing = false;
                        let response = self.load_project(filename);
                        self.push_response(response);
                    }
                    EngineInput::Play => self.is_playing = true,
                    EngineInput::Pause => self.is_playing = false,
                    EngineInput::SkipToStart => {
                        self.clock.reset(self.clock.sample_rate());
                    }
                    EngineInput::Midi(channel, message) => {
                        messages.push(GrooveInput::MidiFromExternal(channel, message))
                    }
                    EngineInput::QuitRequested => {
                        self.post_event(EngineEvent::Quit);
                        break;
                    }
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
                    EngineInput::ConnectController(
                        controllable_id,
                        controller_id,
                        control_index,
                    ) => {
                        messages.push(GrooveInput::ConnectController(
                            controllable_id,
                            controller_id,
                            control_index,
                        ));
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
            } else {
                // In the normal case, we will break when we get the
                // QuitRequested message. This break catches the case where the
                // senders unexpectedly died.
                eprintln!("Unexpected termination of EngineInput senders");
                break;
            }
        }
    }

    /// Periodically sends out events useful for GUI display.
    fn publish_dashboard_updates(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_clock_update).as_millis() > (1000 / 30) {
            let frames = self.clock.frames();
            if frames != self.last_reported_frames {
                self.last_reported_frames = frames;
                self.post_event(EngineEvent::SetClock(frames));
                self.last_clock_update = now;
                self.yes += 1.0;
            }

            // TODO: this is active only while the project is playing. I wanted
            // it whenever the app is open, but it caused 30% CPU usage,
            // probably because of app redraws.
            if self.is_playing {
                self.post_event(EngineEvent::AudioBufferFullness(Normal::from(
                    self.audio_output.buffer_utilization(),
                )));
            }

            #[cfg(disabled)]
            eprintln!(
                "Duty cycle is {:0.2}/{:0.2} {:0.2}%",
                self.yes,
                self.check,
                self.yes / self.check
            );
        }
        self.check += 1.0;
    }

    fn publish_bpm_update(&mut self) {
        self.post_event(EngineEvent::SetBpm(self.clock.bpm()));
    }

    fn publish_time_signature_update(&mut self) {
        self.post_event(EngineEvent::SetTimeSignature(self.time_signature));
    }

    fn start_audio(&mut self) {
        self.audio_output.start();
    }

    fn stop_audio(&mut self) {
        self.audio_output.stop();
    }

    fn load_project(&mut self, filename: String) -> Response<GrooveEvent> {
        let mut path = Paths::projects_path(PathType::Global);
        path.push(filename.clone());
        if let Ok(settings) = SongSettings::new_from_yaml_file(path.to_str().unwrap()) {
            if let Ok(instance) = settings.instantiate(&Paths::assets_path(PathType::Global), false)
            {
                let title = instance.title();
                if let Ok(mut o) = self.orchestrator.lock() {
                    // I'm amazed this works whenever I see it, but I think it's
                    // just saying that we're replacing what the reference
                    // points to with new content. I don't see how that can
                    // work, but it does work.
                    *o = instance;
                }
                return Response::single(GrooveEvent::ProjectLoaded(filename, title));
            }
        }
        Response::none()
    }

    fn generate_audio(&mut self, buffer_count: u8) {
        let mut samples = [StereoSample::SILENCE; Self::ENGINE_BUFFER_SIZE];
        for _ in 0..buffer_count {
            if self.is_playing {
                let (response, ticks_completed) = if let Ok(mut o) = self.orchestrator.lock() {
                    o.tick(&mut samples)
                } else {
                    (Response::none(), 0)
                };
                self.push_response(response);
                if ticks_completed < samples.len() {
                    self.is_playing = false;
                }

                // This clock is used to tell the app where we are in the song,
                // so even though it looks like it's not helping here in the
                // loop, it's necessary. We have it before the second is_playing
                // test because the tick() that returns false still produced
                // some samples, so we want the clock to reflect that.
                self.clock.tick_batch(ticks_completed);

                self.dispatch_samples(&samples, ticks_completed);
            } else {
                self.dispatch_samples(&samples, samples.len());
            }
        }
    }
}
