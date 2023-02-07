use crate::{
    midi::MidiChannel, traits::Response, AudioOutput, Clock, GrooveMessage, IOHelper, Orchestrator,
    StereoSample, TimeSignature,
};
use iced::futures::channel::mpsc;
use iced_native::subscription::{self, Subscription};
use midly::MidiMessage;
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

enum State {
    Start,
    Ready(JoinHandle<()>, mpsc::Receiver<GrooveEvent>),
    Ending(JoinHandle<()>),
    Idle,
}

#[derive(Clone, Debug)]
pub enum GrooveInput {
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
    SetBpm(f32),

    /// Change time signature.
    SetTimeSignature(TimeSignature),

    /// End this thread.
    QuitRequested,
}

#[derive(Clone, Debug)]
pub enum GrooveEvent {
    Ready(mpsc::Sender<GrooveInput>, Arc<Mutex<Orchestrator>>),
    SetClock(usize),
    SetBpm(f32),
    SetTimeSignature(TimeSignature),
    MidiToExternal(MidiChannel, MidiMessage),
    ProjectLoaded(String, Option<String>),
    AudioOutput(StereoSample),
    OutputComplete,
    Quit,
}

struct Runner {
    orchestrator: Arc<Mutex<Orchestrator>>,
    clock: Clock,
    last_clock_update: Instant,

    messages: Vec<GrooveMessage>,
    sender: mpsc::Sender<GrooveEvent>,
    receiver: mpsc::Receiver<GrooveInput>,
    audio_output: Option<AudioOutput>,

    buffer_target: usize,
}
impl Runner {
    pub fn new_with(
        orchestrator: Arc<Mutex<Orchestrator>>,
        sender: mpsc::Sender<GrooveEvent>,
        receiver: mpsc::Receiver<GrooveInput>,
    ) -> Self {
        Self {
            orchestrator,
            clock: Default::default(),
            last_clock_update: Instant::now(),
            messages: Default::default(),
            sender,
            receiver,
            audio_output: None,

            buffer_target: 2048,
        }
    }

    fn push_response(&mut self, response: Response<GrooveMessage>) {
        match response.0 {
            crate::traits::Internal::None => {}
            crate::traits::Internal::Single(message) => {
                self.messages.push(message);
            }
            crate::traits::Internal::Batch(messages) => {
                self.messages.extend(messages);
            }
        }
    }

    fn post_event(&mut self, event: GrooveEvent) {
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
        while let Some(message) = self.messages.pop() {
            match message {
                GrooveMessage::AudioOutput(output_sample) => sample = output_sample,
                GrooveMessage::OutputComplete => {
                    done = true;
                    self.post_event(GrooveEvent::OutputComplete);
                }
                GrooveMessage::MidiToExternal(channel, message) => {
                    self.post_event(GrooveEvent::MidiToExternal(channel, message))
                }
                GrooveMessage::LoadedProject(filename, title) => {
                    self.post_event(GrooveEvent::ProjectLoaded(filename, title))
                }
                _ => todo!(),
            }
        }
        (sample, done)
    }

    fn dispatch_sample(&mut self, sample: StereoSample) {
        if let Some(output) = self.audio_output.as_mut() {
            let _ = output.push(sample);
        }
    }

    pub fn do_loop(&mut self) {
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
                    GrooveInput::LoadProject(filename) => {
                        self.clock.reset();
                        is_playing = false;
                        messages.push(GrooveMessage::LoadProject(filename));
                    }
                    GrooveInput::Play => is_playing = true,
                    GrooveInput::Pause => is_playing = false,
                    GrooveInput::SkipToStart => {
                        self.clock.reset();
                    }
                    GrooveInput::Midi(channel, message) => {
                        messages.push(GrooveMessage::MidiFromExternal(channel, message))
                    }
                    GrooveInput::QuitRequested => break,
                    GrooveInput::SetBpm(bpm) => {
                        if bpm != self.clock.bpm() {
                            self.clock.set_bpm(bpm);
                            self.publish_bpm_update();
                        }
                    }
                    GrooveInput::SetTimeSignature(time_signature) => {
                        if time_signature != self.clock.settings().time_signature() {
                            self.clock.set_time_signature(time_signature);
                            self.publish_time_signature_update();
                        }
                    }
                }
            }

            // Forward any messages that were meant for Orchestrator.
            // Any responses we get at this point are to messages that aren't
            // Tick, so we can ignore the return values from
            // send_pending_messages().
            while let Some(message) = messages.pop() {
                let response = if let Ok(mut o) = self.orchestrator.lock() {
                    o.update(&self.clock, message)
                } else {
                    Response::none()
                };
                self.push_response(response);
            }
            let (_, _) = self.handle_pending_messages();

            if is_playing {
                // Send Tick to Orchestrator so it can do the bulk of its work for
                // the loop.
                let response = if let Ok(mut o) = self.orchestrator.lock() {
                    o.update(&self.clock, GrooveMessage::Tick)
                } else {
                    Response::none()
                };
                self.push_response(response);

                // Since this is a response to a Tick, we know that we got an
                // AudioOutput and maybe an OutputComplete. Thus the return values
                // we get here are meaningful.
                let (sample, done) = self.handle_pending_messages();
                if done {
                    // TODO: I think we need to identify the edge between not done
                    // and done, and advance the clock one more time. Or maybe what
                    // we really need is to have two clocks, one driving the
                    // automated note events, and the other driving the audio
                    // processing.
                    is_playing = false;
                }

                if is_playing {
                    self.clock.tick();
                    self.dispatch_sample(sample);

                    self.wait_for_audio_buffer();
                }
            }
        }
    }

    /// Periodically sends out an event telling the app what time we think it is.
    fn publish_clock_update(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_clock_update).as_millis() > 15 {
            self.post_event(GrooveEvent::SetClock(self.clock.samples()));
            self.last_clock_update = now;
        }
    }

    fn publish_bpm_update(&mut self) {
        self.post_event(GrooveEvent::SetBpm(self.clock.bpm()));
    }

    fn publish_time_signature_update(&mut self) {
        self.post_event(GrooveEvent::SetTimeSignature(
            self.clock.settings().time_signature(),
        ));
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
}

pub struct GrooveSubscription {}
impl GrooveSubscription {
    pub fn subscription() -> Subscription<GrooveEvent> {
        subscription::unfold(
            std::any::TypeId::of::<GrooveSubscription>(),
            State::Start,
            |state| async move {
                match state {
                    State::Start => {
                        // This channel lets the app send us messages.
                        //
                        // TODO: what's the right number for the buffer size?
                        let (app_sender, app_receiver) = mpsc::channel::<GrooveInput>(1024);

                        // This channel surfaces event messages from
                        // Runner/Orchestrator as subscription events.
                        let (thread_sender, thread_receiver) = mpsc::channel::<GrooveEvent>(1024);

                        // TODO: deal with output-device and sample-rate changes.
                        let mut t = Orchestrator::default();
                        t.set_sample_rate(IOHelper::get_output_device_sample_rate());
                        let orchestrator = Arc::new(Mutex::new(t));
                        let orchestrator_for_app = Arc::clone(&orchestrator);
                        let handler = std::thread::spawn(move || {
                            let mut runner =
                                Runner::new_with(orchestrator, thread_sender, app_receiver);
                            runner.start_audio();
                            runner.do_loop();
                            runner.stop_audio();
                        });

                        (
                            Some(GrooveEvent::Ready(app_sender, orchestrator_for_app)),
                            State::Ready(handler, thread_receiver),
                        )
                    }
                    State::Ready(handler, mut receiver) => {
                        use iced_native::futures::StreamExt;

                        let groove_event = receiver.select_next_some().await;
                        if let GrooveEvent::Quit = groove_event {
                            (Some(GrooveEvent::Quit), State::Ending(handler))
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
