use crate::{
    common::{MonoSample, MONO_SAMPLE_SILENCE},
    instruments::oscillators::Oscillator,
    messages::{EntityMessage, MessageBounds},
    midi::MidiChannel,
    traits::{Response, TestController, TestEffect, TestInstrument},
    utils::{AudioSource, TestLfo, TestSynth, Timer, Trigger},
    AudioOutput, Clock, GrooveMessage, GrooveOrchestrator, Orchestrator, TimeSignature,
};
use iced::{
    alignment::{Horizontal, Vertical},
    futures::channel::mpsc,
    theme,
    widget::row,
    Color, Element, Font,
};
use iced::{
    widget::{column, container, text},
    Theme,
};
use iced_native::subscription::{self, Subscription};
use midly::MidiMessage;
use std::{marker::PhantomData, time::Instant};
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

pub const SMALL_FONT_SIZE: u16 = 16;
pub const SMALL_FONT: Font = Font::External {
    name: "Small Font",
    bytes: include_bytes!("../res/fonts/SourceSansPro-Regular.ttf"),
};

pub const LARGE_FONT_SIZE: u16 = 20;
pub const LARGE_FONT: Font = Font::External {
    name: "Large Font",
    bytes: include_bytes!("../res/fonts/SourceSansPro-Regular.ttf"),
};

pub const NUMBERS_FONT_SIZE: u16 = 32;
pub const NUMBERS_FONT: Font = Font::External {
    name: "Numbers Font",
    bytes: include_bytes!("../res/fonts/NotoSansMono-Regular.ttf"),
};

pub trait Viewable {
    type ViewMessage;

    fn view(&self) -> Element<'_, Self::ViewMessage, iced::Renderer> {
        GuiStuff::titled_container(
            "Untitled",
            text("under construction")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center)
                .into(),
        )
    }
}

struct TitledContainerTitleStyle {
    theme: iced::Theme,
}

impl container::StyleSheet for TitledContainerTitleStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let palette = self.theme.extended_palette();
        container::Appearance {
            text_color: Some(palette.background.strong.text),
            background: Some(palette.background.strong.color.into()),
            ..Default::default()
        }
    }
}

struct NumberContainerStyle {
    _theme: iced::Theme,
}

impl container::StyleSheet for NumberContainerStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb8(255, 255, 0)),
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct GuiStuff<'a, Message> {
    phantom: PhantomData<&'a Message>,
}

impl<'a, Message: 'a> GuiStuff<'a, Message> {
    pub fn titled_container(title: &str, contents: Element<'a, Message>) -> Element<'a, Message> {
        container(column![
            Self::titled_container_title(title),
            container(contents).padding(2)
        ])
        .padding(0)
        .style(theme::Container::Box)
        .into()
    }

    #[allow(unused_variables)]
    pub fn titled_container_title(title: &str) -> Element<'a, Message> {
        // let checkboxes = container(if let Some(device) = device { row![
        //     checkbox( "Enabled".to_string(), device.borrow().is_enabled(),
        //         ViewableMessage::EnablePressed ), checkbox(
        //             "Muted".to_string(), device.borrow().is_muted(),
        //             ViewableMessage::MutePressed ) ] } else {
        //             row![text("".to_string())] });
        container(row![
            text(title.to_string())
                .font(SMALL_FONT)
                .size(SMALL_FONT_SIZE)
                .horizontal_alignment(iced::alignment::Horizontal::Left)
                .vertical_alignment(Vertical::Center),
            // checkboxes
        ])
        .width(iced::Length::Fill)
        .padding(1)
        .style(theme::Container::Custom(
            Self::titled_container_title_style(&Theme::Dark),
        ))
        .into()
    }

    pub fn container_text(label: &str) -> Element<'a, Message> {
        text(label.to_string())
            .font(LARGE_FONT)
            .size(LARGE_FONT_SIZE)
            .horizontal_alignment(iced::alignment::Horizontal::Left)
            .vertical_alignment(Vertical::Center)
            .into()
    }

    fn titled_container_title_style(
        theme: &iced::Theme,
    ) -> Box<(dyn iced::widget::container::StyleSheet<Style = Theme>)> {
        Box::new(TitledContainerTitleStyle {
            theme: theme.clone(),
        })
    }

    pub fn number_box_style(
        theme: &iced::Theme,
    ) -> Box<(dyn iced::widget::container::StyleSheet<Style = Theme>)> {
        Box::new(NumberContainerStyle {
            _theme: theme.clone(),
        })
    }
}

impl Viewable for Oscillator {
    type ViewMessage = EntityMessage;
}
impl<M: MessageBounds> Viewable for TestController<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for TestEffect<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for TestInstrument<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for TestLfo<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for TestSynth<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for Timer<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for Trigger<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for AudioSource<M> {
    type ViewMessage = M;
}

enum State {
    Start,
    Ready(JoinHandle<()>, mpsc::Receiver<GrooveEvent>),
    Ending(JoinHandle<()>),
    Idle,
}

#[derive(Clone, Debug)]
pub enum GrooveInput {
    LoadProject(String),
    Play,
    Pause,
    Restart,
    Midi(MidiChannel, MidiMessage),
    SetBpm(f32),
    SetTimeSignature(TimeSignature),
    QuitRequested,
}

#[derive(Clone, Debug)]
pub enum GrooveEvent {
    Ready(mpsc::Sender<GrooveInput>, Arc<Mutex<GrooveOrchestrator>>),
    SetClock(usize),
    SetBpm(f32),
    SetTimeSignature(TimeSignature),
    MidiToExternal(MidiChannel, MidiMessage),
    ProjectLoaded(String),
    AudioOutput(MonoSample),
    OutputComplete,
    Quit,
}

struct Runner {
    orchestrator: Arc<Mutex<GrooveOrchestrator>>,
    clock: Clock,
    last_clock_update: Instant,

    messages: Vec<GrooveMessage>,
    sender: mpsc::Sender<GrooveEvent>,
    receiver: mpsc::Receiver<GrooveInput>,
    audio_output: Option<AudioOutput>,
}
impl Runner {
    pub fn new_with(
        orchestrator: Arc<Mutex<GrooveOrchestrator>>,
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
    fn handle_pending_messages(&mut self) -> (MonoSample, bool) {
        let mut sample: MonoSample = MONO_SAMPLE_SILENCE;
        let mut done = false;
        while let Some(message) = self.messages.pop() {
            match message {
                GrooveMessage::AudioOutput(output_sample) => sample = output_sample,
                GrooveMessage::OutputComplete => done = true,
                GrooveMessage::MidiToExternal(channel, message) => {
                    self.post_event(GrooveEvent::MidiToExternal(channel, message))
                }
                GrooveMessage::LoadedProject(filename) => {
                    self.post_event(GrooveEvent::ProjectLoaded(filename))
                }
                _ => todo!(),
            }
        }
        (sample, done)
    }

    fn dispatch_sample(&mut self, sample: f32) {
        if let Some(output) = self.audio_output.as_mut() {
            output.worker_mut().push(sample);
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
                    GrooveInput::Restart => {
                        self.clock.reset();
                        is_playing = true;
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
                    o.update(&mut self.clock, message)
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
                    o.update(&mut self.clock, GrooveMessage::Tick)
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
        let mut audio_output = AudioOutput::default();
        audio_output.stop();
        self.audio_output = None;
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

                        let orchestrator = Arc::new(Mutex::new(Orchestrator::default()));
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

                        if let GrooveEvent::Quit = receiver.select_next_some().await {
                            (Some(GrooveEvent::Quit), State::Ending(handler))
                        } else {
                            (
                                Some(receiver.select_next_some().await),
                                State::Ready(handler, receiver),
                            )
                        }
                    }
                    State::Ending(handler) => {
                        let _ = handler.join();
                        // See https://github.com/iced-rs/iced/issues/1348
                        return (None, State::Idle);
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

#[cfg(test)]
mod tests {
    use super::{GuiStuff, Viewable};
    use crate::utils::tests::{TestControlSourceContinuous, TestMixer};
    use crate::{
        controllers::sequencers::BeatSequencer,
        effects::{
            filter::{BiQuadFilter, FilterParams},
            gain::Gain,
        },
        messages::{tests::TestMessage, EntityMessage, MessageBounds},
    };
    use iced::{
        widget::{container, text},
        Element,
    };
    use std::any::type_name;

    impl<M: MessageBounds> Viewable for TestMixer<M> {
        type ViewMessage = M;
    }
    impl<M: MessageBounds> Viewable for TestControlSourceContinuous<M> {
        type ViewMessage = M;
    }
    impl Viewable for BiQuadFilter<TestMessage> {
        type ViewMessage = TestMessage;

        fn view(&self) -> Element<Self::ViewMessage> {
            container(text("not implemented")).into()
        }
    }
    impl Viewable for BeatSequencer<TestMessage> {
        type ViewMessage = TestMessage;

        fn view(&self) -> Element<Self::ViewMessage> {
            let title = type_name::<Self>();
            let contents = format!("cursor point: {}", "tOdO");
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
        }
    }

    // impl Viewable for PatternManager { type ViewMessage = GrooveMessage;

    //     fn view(&self) -> Element<Self::ViewMessage> { let title =
    //         type_name::<PatternManager>(); let contents = { let pattern_views
    //         = self.patterns().iter().enumerate().map(|(i, item)| {
    //             item.view() .map(move |message|
    //                 Self::ViewMessage::PatternMessage(i, message)) });
    //                     column(pattern_views.collect()) };
    //             GuiStuff::titled_container(title, contents.into()) }
    // }

    // There aren't many assertions in this method, but we know it'll panic or
    // spit out debug messages if something's wrong.
    fn test_one_viewable(
        viewable: Box<dyn Viewable<ViewMessage = EntityMessage>>,
        message: Option<EntityMessage>,
    ) {
        let _ = viewable.view();
        if let Some(_message) = message {
            //viewable.update(message);
        }
    }

    #[ignore]
    #[test]
    fn test_viewables_of_generic_entities() {
        // TODO: some of these commented-out entities could be made generic, but
        // it's a maintenance cost, and I don't know for sure if they're even
        // useful being able to respond to TestMessaage. I think I know how to
        // genericize entities pretty well now, so it's not

        // test_one_viewable( Box::new(WelshSynth::new_with( 44100,
        //     SynthPatch::by_name(&PatchName::Trombone), )), None, );
        //         test_one_viewable(Box::new(DrumkitSampler::new_from_files()),
        //         None); test_one_viewable(Box::new(Sampler::new_with(1024)),
        //     None); TODO - test it! test_one_viewable(Mixer::new_wrapped(),
        //     None);
        test_one_viewable(
            Box::new(Gain::<EntityMessage>::default()),
            Some(EntityMessage::UpdateParam0U8(28)),
        );
        // test_one_viewable( Box::new(Bitcrusher::new_with(7)),
        //     Some(GrooveMessage::BitcrusherValueChanged(4)), );
        test_one_viewable(
            Box::new(BiQuadFilter::<EntityMessage>::new_with(
                &FilterParams::AllPass {
                    cutoff: 1000.0,
                    q: 2.0,
                },
                44100,
            )),
            Some(EntityMessage::UpdateParam1F32(500.0)),
        );
        // test_one_viewable( Box::new(Limiter::new_with(0.0, 1.0)),
        //     Some(GrooveMessage::LimiterMinChanged(0.5)), );
        //     test_one_viewable( Box::new(Arpeggiator::new_with(1)),
        // Some(GrooveMessage::ArpeggiatorChanged(42)), );
        test_one_viewable(
            Box::new(BeatSequencer::<EntityMessage>::default()),
            Some(EntityMessage::EnablePressed(false)),
        );
    }
}
