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
    time::{Clock, ClockNano, TimeSignature},
    traits::Resets,
    Normal, ParameterType, StereoSample,
};
use groove_orchestration::{
    helpers::IOHelper,
    messages::{ControlLink, GrooveEvent, GrooveInput, Internal, Response},
    Orchestrator,
};
use groove_settings::SongSettings;
use iced::futures::channel::mpsc as iced_mpsc;
use iced_native::subscription::{self, Subscription};
use std::{
    collections::VecDeque,
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
#[derive(Debug)]
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
    Stop,

    /// Reset the cursor to time zero.
    SkipToStart,

    /// Handle this incoming MIDI message from external.
    Midi(MidiChannel, MidiMessage),

    /// Change sample rate.
    SetSampleRate(usize),

    /// Change BPM.
    SetBpm(ParameterType),

    /// Change time signature.
    SetTimeSignature(TimeSignature),

    /// Connect an IsController to a Controllable's control point.
    AddControlLink(ControlLink),

    /// Disconnect an IsController from a Controllable's control point.
    RemoveControlLink(ControlLink),

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

    /// An internal message that is passed directly through.
    GrooveEvent(GrooveEvent),

    /// The sample rate has changed.
    SampleRateChanged(usize),

    /// Sends the engine's current frame. Useful for the GUI to keep the control
    /// bar's clock in sync.
    /// TODO: this MAYBE should be part of GrooveEvent -- TODO yes absolutely
    SetClock(usize),

    /// Sends an updated BPM (beats per minute) whenever it changes.
    /// TODO: this MAYBE should be part of GrooveEvent
    SetBpm(ParameterType),

    /// Sends an updated global time signature whenever it changes. Note that
    /// individual components might have independent time signatures that
    /// operate on their own time.
    /// TODO: this MAYBE should be part of GrooveEvent
    SetTimeSignature(TimeSignature),

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
    // Orchestrator is wrapped in a mutex because we've chosen to give the app
    // direct access to it while building the GUI view.
    orchestrator: Arc<Mutex<Orchestrator>>,
    clock: Clock,
    time_signature: TimeSignature,
    last_clock_update: Instant,
    last_reported_frames: usize,
    is_playing: bool,

    // TODO: I'm surprised that this seems to belong here. This struct does a
    // fair amount of instantiation of key things like Orchestrator, and it owns
    // the audio subsystem, so we need it. But it might turn out that the
    // subscriber is a better owner, and this ends up being a local copy.
    sample_rate: usize,

    // This is true when playback went all the way to the end of the song. The
    // reason it's nice to track this is that after pressing play and listening
    // to the song, the user can press play again without manually resetting the
    // clock to the start. But we don't want to just reset the clock at the end
    // of playback, because that means the clock would read zero at the end of
    // playback, which is undesirable because it's natural to want to know how
    // long the song was after listening, and it's nice to be able to glance at
    // the stopped clock and get that answer.
    reached_end_of_playback: bool,

    events: VecDeque<GrooveEvent>,
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
                        let clock_params = ClockNano {
                            bpm: DEFAULT_BPM,
                            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
                            time_signature: TimeSignature { top: 4, bottom: 4 },
                        };
                        let mut t = Orchestrator::new_with(clock_params.clone());
                        let sample_rate = IOHelper::get_output_device_sample_rate();
                        t.reset(sample_rate);
                        let orchestrator = Arc::new(Mutex::new(t));
                        let orchestrator_for_app = Arc::clone(&orchestrator);
                        let handler = std::thread::spawn(move || {
                            let audio_output = AudioOutput::new_with(input_sender.clone());
                            let mut clock = Clock::new_with(clock_params);
                            clock.reset(sample_rate);
                            let mut subscription = Self::new_with(
                                sample_rate,
                                orchestrator,
                                clock,
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
        sample_rate: usize,
        orchestrator: Arc<Mutex<Orchestrator>>,
        clock: Clock,
        sender: iced_mpsc::Sender<EngineEvent>,
        receiver: mpsc::Receiver<EngineInput>,
        audio_output: AudioOutput,
    ) -> Self {
        Self {
            sample_rate,
            orchestrator,
            clock,
            time_signature: TimeSignature { top: 4, bottom: 4 }, // TODO: what's a good "don't know yet" value?
            last_clock_update: Instant::now(),
            last_reported_frames: usize::MAX,
            is_playing: Default::default(),
            reached_end_of_playback: Default::default(),
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
                self.events.push_back(message);
            }
            Internal::Batch(messages) => {
                self.events.extend(messages);
            }
        }
    }

    fn post_event(&mut self, event: EngineEvent) {
        let _ = self.sender.try_send(event);
    }

    /// Forwards queued-up events to the app.
    fn forward_pending_events(&mut self) {
        while let Some(event) = self.events.pop_front() {
            self.post_event(EngineEvent::GrooveEvent(event));
        }
    }

    fn dispatch_samples(&mut self, samples: &[StereoSample], sample_count: usize) {
        let _ = self.audio_output.push_buffer(&samples[0..sample_count]);
    }

    fn do_loop(&mut self) {
        let mut messages = Vec::new();
        loop {
            if let Ok(input) = self.receiver.recv() {
                self.publish_dashboard_updates();

                match input {
                    EngineInput::GenerateAudio(buffer_count) => self.generate_audio(buffer_count),
                    EngineInput::LoadProject(filename) => {
                        self.stop_playback();
                        let response = self.load_project(&filename);
                        self.push_response(response);
                    }
                    EngineInput::Play => {
                        self.start_or_pause_playback();
                        messages.push(GrooveInput::Play);
                    }
                    EngineInput::SetSampleRate(sample_rate) => {
                        self.update_and_broadcast_sample_rate(sample_rate);
                        messages.push(GrooveInput::SetSampleRate(sample_rate));
                    }
                    EngineInput::Stop => {
                        self.stop_playback();
                        messages.push(GrooveInput::Stop);
                    }
                    EngineInput::SkipToStart => {
                        self.skip_to_start();
                        messages.push(GrooveInput::SkipToStart);
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
                    EngineInput::AddControlLink(link) => {
                        messages.push(GrooveInput::AddControlLink(link));
                    }
                    EngineInput::RemoveControlLink(link) => {
                        messages.push(GrooveInput::RemoveControlLink(link));
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
                self.forward_pending_events();
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
        self.clock.set_sample_rate(sample_rate);
        self.post_event(EngineEvent::SampleRateChanged(sample_rate));
    }

    fn start_or_pause_playback(&mut self) {
        if self.is_playing {
            self.stop_playback();
        } else {
            if self.reached_end_of_playback {
                self.skip_to_start();
                self.reached_end_of_playback = false;
            }
            self.post_event(EngineEvent::GrooveEvent(GrooveEvent::PlaybackStarted));
            self.is_playing = true;
        }
    }

    fn stop_playback(&mut self) {
        self.post_event(EngineEvent::GrooveEvent(GrooveEvent::PlaybackStopped));

        // This logic allows the user to press stop twice as shorthand for going
        // back to the start.
        if self.is_playing {
            self.is_playing = false;
        } else {
            self.skip_to_start();
        }
    }

    fn skip_to_start(&mut self) {
        self.clock.reset(self.clock.sample_rate());
        self.post_event(EngineEvent::SetClock(0));
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
        self.post_event(EngineEvent::SetTimeSignature(
            self.clock.time_signature().clone(),
        ));
    }

    fn start_audio(&mut self) {
        self.audio_output.start();
    }

    fn stop_audio(&mut self) {
        self.audio_output.stop();
    }

    fn load_project(&mut self, filename: &str) -> Response<GrooveEvent> {
        let mut path = Paths::projects_path(PathType::Global);
        path.push(filename);
        if let Ok(settings) = SongSettings::new_from_yaml_file(path.to_str().unwrap()) {
            if let Ok(instance) = settings.instantiate(&Paths::assets_path(PathType::Global), false)
            {
                let title = instance.title();

                let mut v = Vec::default();

                // Tell the app we've loaded the project
                v.push(Response::single(GrooveEvent::ProjectLoaded(
                    filename.to_string(),
                    title,
                )));

                // And that it should clear its local representation of the project
                v.push(Response::single(GrooveEvent::Clear));

                // And that it should add the following new entities/relationships
                v.extend(
                    instance
                        .generate_full_update_messages()
                        .into_iter()
                        .map(|m| Response::single(m))
                        .collect::<Vec<Response<GrooveEvent>>>(),
                );
                if let Ok(mut o) = self.orchestrator.lock() {
                    // I'm amazed this works whenever I see it, but I think it's
                    // just saying that we're replacing what the reference
                    // points to with new content. I don't see how that can
                    // work, but it does work.
                    *o = instance;
                    o.reset(self.sample_rate);
                }
                return Response::batch(v);
            }
        }
        Response::none()
    }

    fn generate_audio(&mut self, buffer_count: u8) {
        let mut samples = [StereoSample::SILENCE; Self::ENGINE_BUFFER_SIZE];
        for i in 0..buffer_count {
            let want_audio_update = i == buffer_count - 1;
            let mut other_response = Response::none();
            let (response, ticks_completed) = if let Ok(mut o) = self.orchestrator.lock() {
                if self.clock.was_reset() {
                    // This could be an expensive operation, since it might
                    // cause a bunch of heap activity. So it's better to do
                    // it as soon as it's needed, rather than waiting for
                    // the time-sensitive generate_audio() method. TODO
                    // move.
                    if let Some(sample_rate) = o.sample_rate() {
                        o.reset(sample_rate);
                    } else {
                        panic!("We're in the middle of generate_audio() but don't have a sample rate. This is bad!");
                    }
                }
                let r = o.tick(&mut samples);
                if want_audio_update {
                    let wad = o.last_audio_wad();
                    other_response = Response::single(GrooveEvent::EntityAudioOutput(wad));
                }
                r
            } else {
                (Response::none(), 0)
            };
            self.push_response(response);
            self.push_response(other_response);
            if ticks_completed < samples.len() {
                self.stop_playback();
                self.reached_end_of_playback = true;
            }
            let ticks_completed = samples.len(); // HACK!

            // This clock is used to tell the app where we are in the song,
            // so even though it looks like it's not helping here in the
            // loop, it's necessary. We have it before the second is_playing
            // test because the tick() that returns false still produced
            // some samples, so we want the clock to reflect that.
            self.clock.tick_batch(ticks_completed);

            self.dispatch_samples(&samples, ticks_completed);
        }
    }
}
