use crate::{
    common::{MonoSample, MONO_SAMPLE_SILENCE},
    instruments::oscillators::Oscillator,
    messages::{EntityMessage, MessageBounds},
    midi::MidiChannel,
    traits::{Response, TestController, TestEffect, TestInstrument},
    utils::{AudioSource, Timer, Trigger},
    AudioOutput, Clock, GrooveMessage, GrooveOrchestrator, Orchestrator,
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
use std::marker::PhantomData;
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
    Ready(JoinHandle<()>, mpsc::Receiver<GrooveMessage>),
    Ending(JoinHandle<()>),
    Idle,
}

#[derive(Clone, Debug)]
pub(crate) enum GrooveInput {
    Start,
    Tick,
    EntityMessage(usize, EntityMessage),
    MidiFromExternal(MidiChannel, MidiMessage),
    Quit,
}

#[derive(Clone, Debug)]
pub enum GrooveEvent {
    Ready(mpsc::Sender<GrooveMessage>, Arc<Mutex<GrooveOrchestrator>>),
    GrooveMessage(GrooveMessage),
    ProgressReport(i32),
    MidiToExternal(MidiChannel, MidiMessage),
    AudioOutput(MonoSample),
    OutputComplete,
    Quit,
}

struct Runner {
    orchestrator: Arc<Mutex<GrooveOrchestrator>>,
    clock: Clock,
    messages: Vec<GrooveMessage>,
    sender: mpsc::Sender<GrooveMessage>,
    receiver: mpsc::Receiver<GrooveMessage>,
    audio_output: Option<AudioOutput>,
}
impl Runner {
    pub fn new_with(
        orchestrator: Arc<Mutex<GrooveOrchestrator>>,
        sender: mpsc::Sender<GrooveMessage>,
        receiver: mpsc::Receiver<GrooveMessage>,
    ) -> Self {
        Self {
            orchestrator,
            clock: Default::default(),
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

    /// Processes any queued-up messages that we can handle, and sends the rest
    /// to the app.
    ///
    /// Returns an audio sample if found, and returns true if the orchestrator
    /// has indicated that it's done with its work.
    fn send_pending_messages(&mut self) -> (MonoSample, bool) {
        let mut sample: MonoSample = MONO_SAMPLE_SILENCE;
        let mut done = false;
        while let Some(message) = self.messages.pop() {
            match message {
                GrooveMessage::AudioOutput(output_sample) => sample = output_sample,
                GrooveMessage::OutputComplete => done = true,
                _ => {
                    // Any unmatched message is OK to send to the app.
                    let _ = self.sender.try_send(message);
                }
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
        loop {
            // Send any pending received messages to Orchestrator before asking
            // it to handle Tick.
            while let Ok(Some(message)) = self.receiver.try_next() {
                let response = if let Ok(mut o) = self.orchestrator.lock() {
                    o.update(&mut self.clock, message)
                } else {
                    Response::none()
                };
                self.push_response(response);
            }
            // Any responses we get at this point are to messages that aren't
            // Tick, so we can ignore the return values from
            // send_pending_messages().
            let (_, _) = self.send_pending_messages();

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
            let (sample, done) = self.send_pending_messages();

            self.clock.tick();
            if done {
                println!("we're exiting!");
                // break;
            }

            // If everyone thought we were done on this spin of the loop, then
            // the audio sample that was returned is garbage, so we dispatch the
            // sample only after exiting on done.
            self.dispatch_sample(sample);
        }
    }

    pub fn start_audio(&mut self) {
        let mut audio_output = AudioOutput::default();
        audio_output.start();
        self.audio_output = Some(audio_output);
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
                        let (app_sender, app_receiver) = mpsc::channel::<GrooveMessage>(1024);

                        // This channel surfaces event messages from
                        // Runner/Orchestrator as subscription events.
                        let (thread_sender, thread_receiver) = mpsc::channel::<GrooveMessage>(1024);

                        let orchestrator = Arc::new(Mutex::new(Orchestrator::default()));
                        let orchestrator_for_app = Arc::clone(&orchestrator);
                        let handler = std::thread::spawn(move || {
                            let mut runner =
                                Runner::new_with(orchestrator, thread_sender, app_receiver);
                            runner.start_audio();
                            runner.do_loop();
                        });

                        (
                            Some(GrooveEvent::Ready(app_sender, orchestrator_for_app)),
                            State::Ready(handler, thread_receiver),
                        )
                    }
                    State::Ready(handler, mut receiver) => {
                        use iced_native::futures::StreamExt;

                        let event = receiver.select_next_some().await;
                        let mut done = false;
                        match event {
                            GrooveMessage::OutputComplete => {
                                println!("Subscription peeked at GrooveMessage::OutputComplete");
                                done = true;
                            }
                            _ => {
                                println!("I'm ignoring {:?}", &event);
                            }
                        }

                        if done {
                            println!("Subscription is forwarding GrooveEvent::Quit to app");
                            (
                                Some(GrooveEvent::Quit),
                                State::Ending(handler),
                            )
                        } else {
                            (
                                Some(GrooveEvent::GrooveMessage(event)),
                                State::Ready(handler, receiver),
                            )
                        }
                    }
                    State::Ending(handler) => {
                        println!("Subscription ThingState::Ending");
                        if let Ok(_) = handler.join() {
                            println!("Subscription handler.join()");
                        }
                        // See https://github.com/iced-rs/iced/issues/1348
                        return (None, State::Idle);
                    }
                    State::Idle => {
                        println!("Subscription ThingState::Idle");

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
    use std::any::type_name;

    use iced::{
        widget::{container, text},
        Element,
    };

    use super::{GuiStuff, Viewable};
    use crate::utils::tests::{TestControlSourceContinuous, TestLfo, TestMixer, TestSynth};
    use crate::{
        controllers::sequencers::BeatSequencer,
        effects::{
            filter::{BiQuadFilter, FilterParams},
            gain::Gain,
        },
        messages::{tests::TestMessage, EntityMessage, MessageBounds},
    };

    impl<M: MessageBounds> Viewable for TestSynth<M> {
        type ViewMessage = M;
    }
    impl<M: MessageBounds> Viewable for TestMixer<M> {
        type ViewMessage = M;
    }
    impl<M: MessageBounds> Viewable for TestLfo<M> {
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
