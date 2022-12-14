#![windows_subsystem = "windows"]

mod gui;

use groove::{
    gui::{GrooveEvent, GuiStuff, NUMBERS_FONT, NUMBERS_FONT_SIZE},
    AudioOutput, Clock, GrooveMessage, GrooveOrchestrator, GrooveSubscription, MidiHandlerEvent,
    MidiHandlerInput, MidiHandlerMessage, MidiSubscription, TimeSignature,
};
use gui::{
    persistence::{LoadError, SavedState},
    play_icon, skip_to_prev_icon, stop_icon,
};
use iced::{
    alignment, executor,
    futures::channel::mpsc,
    theme::{self, Theme},
    time,
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Application, Command, Element, Length, Settings, Subscription,
};
use iced_native::{window, Event};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct GrooveApp {
    // Overhead
    theme: Theme,
    state: State,
    should_exit: bool,

    // UI components
    control_bar: ControlBar,

    // Model
    project_name: String,
    sender: Option<mpsc::Sender<GrooveMessage>>,
    orchestrator: Arc<Mutex<GrooveOrchestrator>>,
    clock_mirror: Clock, // this clock is just a cache of the real clock in Orchestrator.

    // This is true when playback went all the way to the end of the song. The
    // reason it's nice to track this is that after pressing play and listening
    // to the song, the user can press play again without manually resetting the
    // clock to the start. But we don't want to just reset the clock at the end
    // of playback, because that means the clock would read zero at the end of
    // playback, which is undesirable because it's natural to want to know how
    // long the song was after listening, and it's nice to be able to glance at
    // the stopped clock and get that answer.
    reached_end_of_playback: bool,

    midi_handler_sender: Option<mpsc::Sender<MidiHandlerInput>>,
}

impl Default for GrooveApp {
    fn default() -> Self {
        Self {
            theme: Default::default(),
            state: Default::default(),
            should_exit: Default::default(),
            control_bar: Default::default(),
            project_name: Default::default(),
            sender: Default::default(),
            orchestrator: Default::default(),
            clock_mirror: Default::default(),
            reached_end_of_playback: Default::default(),
            midi_handler_sender: Default::default(),
        }
    }
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Playing,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    Loaded(Result<SavedState, LoadError>),
    ControlBarMessage(ControlBarMessage),
    ControlBarBpm(String),
    GrooveMessage(GrooveMessage),
    MidiHandlerMessage(MidiHandlerMessage),
    MidiHandlerEvent(MidiHandlerEvent),
    Tick(Instant),
    Event(iced::Event),
    GrooveEvent(GrooveEvent),
}

#[derive(Debug, Clone)]
pub enum ControlBarMessage {
    Play,
    Stop,
    SkipToStart,
}

#[derive(Debug, Default, Clone)]
pub struct ControlBar {
    gui_clock: GuiClock,
}

impl ControlBar {
    pub fn view(&self, clock: &Clock) -> Element<AppMessage> {
        container(
            row![
                text_input(
                    "BPM",
                    clock.bpm().round().to_string().as_str(),
                    AppMessage::ControlBarBpm
                )
                .width(Length::Units(60)),
                container(row![
                    button(skip_to_prev_icon())
                        .width(Length::Units(32))
                        .on_press(AppMessage::ControlBarMessage(
                            ControlBarMessage::SkipToStart
                        )),
                    button(play_icon())
                        .width(Length::Units(32))
                        .on_press(AppMessage::ControlBarMessage(ControlBarMessage::Play)),
                    button(stop_icon())
                        .width(Length::Units(32))
                        .on_press(AppMessage::ControlBarMessage(ControlBarMessage::Stop))
                ])
                .align_x(alignment::Horizontal::Center)
                .width(Length::FillPortion(1)),
                container(self.gui_clock.view()).width(Length::FillPortion(1)),
            ]
            .padding(8)
            .spacing(4)
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(4)
        .style(theme::Container::Box)
        .into()
    }
}

#[derive(Debug, Clone)]
pub enum ClockMessage {
    TimeSignature(u8, u8),
    Time(f32),
    Beats(f32),
}

#[derive(Debug, Default, Clone)]
struct GuiClock {
    time_signature: TimeSignature,
    seconds: f32,
    beats: f32,
}

impl GuiClock {
    pub fn update(&mut self, message: ClockMessage) {
        match message {
            ClockMessage::TimeSignature(top, bottom) => {
                // TODO: nobody sends this message. In order to send this
                // message correctly, either Clock needs a live pointer to
                // orchestrator, or we need to look into some way to subscribe
                // to orchestrator changes.
                self.time_signature = TimeSignature::new_with(top.into(), bottom.into());
            }
            ClockMessage::Time(value) => {
                self.seconds = value;
            }
            ClockMessage::Beats(value) => {
                self.beats = value;
            }
        }
    }

    pub fn view(&self) -> Element<AppMessage> {
        let time_counter = {
            let minutes: u8 = (self.seconds / 60.0).floor() as u8;
            let seconds = self.seconds as usize % 60;
            let thousandths = (self.seconds.fract() * 1000.0) as u16;
            container(
                text(format!("{minutes:02}:{seconds:02}:{thousandths:03}"))
                    .font(NUMBERS_FONT)
                    .size(NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<AppMessage>::number_box_style(&Theme::Dark),
            ))
        };

        let time_signature = {
            container(column![
                text(format!("{}", self.time_signature.top)),
                text(format!("{}", self.time_signature.bottom))
            ])
        };

        let beat_counter = {
            let denom = self.time_signature.top as f32;

            let measures = (self.beats / denom) as usize;
            let beats = (self.beats % denom) as usize;
            let fractional = (self.beats.fract() * 10000.0) as usize;
            container(
                text(format!("{measures:04}m{beats:02}b{fractional:03}"))
                    .font(NUMBERS_FONT)
                    .size(NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<AppMessage>::number_box_style(&Theme::Dark),
            ))
        };
        row![time_counter, time_signature, beat_counter].into()
    }
}

impl Application for GrooveApp {
    type Message = AppMessage;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (GrooveApp, Command<AppMessage>) {
        (
            GrooveApp {
                theme: Theme::Dark,
                ..Default::default()
            },
            Command::perform(SavedState::load(), AppMessage::Loaded),
        )
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }

    fn title(&self) -> String {
        String::from(&self.project_name)
    }

    fn update(&mut self, message: AppMessage) -> Command<AppMessage> {
        match message {
            AppMessage::Loaded(Ok(state)) => {
                // TODO: these are (probably) temporary until the project is
                // loaded. Make sure they really need to be instantiated.
                let orchestrator = GrooveOrchestrator::default();
                let clock = Clock::new_with(orchestrator.clock_settings());
                *self = Self {
                    theme: self.theme.clone(),
                    project_name: state.project_name,
                    orchestrator: Arc::new(Mutex::new(orchestrator)),
                    ..Default::default()
                };
            }
            AppMessage::Loaded(Err(_e)) => {
                todo!()
            }
            AppMessage::Tick(_now) => {
                // TODO: decide what a Tick means. The app thinks it's 10
                // milliseconds. Orchestrator thinks it's 1/sample rate.

                //                self.send_midi_handler_tick();

                // TODO: do we still need a tick?
            }
            AppMessage::ControlBarMessage(message) => match message {
                // TODO: not sure if we need ticking for now. it's playing OR
                // midi
                ControlBarMessage::Play => {
                    if self.reached_end_of_playback {
                        // self.clock.reset();
                        self.reached_end_of_playback = false;
                    }
                    self.state = State::Playing
                }
                ControlBarMessage::Stop => {
                    self.reached_end_of_playback = false;
                    match self.state {
                        State::Idle => {
                            // self.clock.reset();
                            self.update_clock();
                        }
                        State::Playing => self.state = State::Idle,
                    }
                }
                ControlBarMessage::SkipToStart => todo!(),
            },
            AppMessage::ControlBarBpm(new_value) => {
                // if let Ok(bpm) = new_value.parse() {
                //     // self.clock.settings_mut().set_bpm(bpm);
                // }
            }
            AppMessage::Event(event) => {
                if let Event::Window(window::Event::CloseRequested) = event {
                    // See https://github.com/iced-rs/iced/pull/804 and
                    // https://github.com/iced-rs/iced/blob/master/examples/events/src/main.rs#L55
                    //
                    // This is needed to stop an ALSA buffer underrun on close

                    // TODO
                    // self.midi.stop();
                    // self.audio_output.stop();

                    self.should_exit = true;
                }
            }
            AppMessage::GrooveMessage(message) => {
                // TODO: we're swallowing the Responses we're getting from
                // update()
                //
                // TODO: is this clock the right one to base everything off?
                // TODO: aaaargh so much cloning
                self.dispatch_groove_message(&self.clock_mirror.clone(), message);
                todo!();
            }
            AppMessage::MidiHandlerMessage(message) => match message {
                // MidiHandlerMessage::InputSelected(which) => self.midi.select_input(which),
                // MidiHandlerMessage::OutputSelected(which) => self.midi.select_output(which),
                _ => todo!(),
            },
            AppMessage::GrooveEvent(event) => match event {
                GrooveEvent::Ready(sender, orchestrator) => {
                    self.sender = Some(sender);
                    self.orchestrator = orchestrator;

                    if let Some(sender) = self.sender.as_mut() {
                        sender.try_send(GrooveMessage::LoadProject("low-cpu.yaml".to_string()));
                    }
                }
                GrooveEvent::GrooveMessage(message) => {
                    match message {
                        GrooveMessage::Nop => todo!(),
                        GrooveMessage::Tick => todo!(),
                        GrooveMessage::EntityMessage(_, _) => todo!(),
                        GrooveMessage::MidiFromExternal(_, _) => todo!(),
                        GrooveMessage::MidiToExternal(channel, message) => {
                            if let Some(sender) = self.midi_handler_sender.as_mut() {
                                sender.try_send(MidiHandlerInput::MidiMessage(channel, message));
                            }
                        }
                        GrooveMessage::AudioOutput(_) => todo!(),
                        GrooveMessage::OutputComplete => todo!(),
                        GrooveMessage::LoadProject(_) => todo!(),
                    }
                    println!("I'm the app, and I'm ignoring {:?}", message);
                }
                GrooveEvent::ProgressReport(_) => todo!(),
                GrooveEvent::MidiToExternal(_, _) => todo!(),
                GrooveEvent::AudioOutput(_) => todo!(),
                GrooveEvent::OutputComplete => todo!(),
                GrooveEvent::Quit => todo!(),
            },
            AppMessage::MidiHandlerEvent(event) => match event {
                MidiHandlerEvent::Ready(sender) => {
                    self.midi_handler_sender = Some(sender);
                }
                MidiHandlerEvent::MidiMessage(_, _) => todo!(),
                MidiHandlerEvent::Quit => todo!(),
            },
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<AppMessage> {
        Subscription::batch([
            iced_native::subscription::events().map(AppMessage::Event),
            GrooveSubscription::subscription().map(AppMessage::GrooveEvent),
            MidiSubscription::subscription().map(AppMessage::MidiHandlerEvent),
            // This is duplicative because I think we'll want different activity
            // levels depending on whether we're playing
            match self.state {
                State::Idle => time::every(Duration::from_millis(10)).map(AppMessage::Tick),
                State::Playing { .. } => {
                    time::every(Duration::from_millis(10)).map(AppMessage::Tick)
                }
            },
        ])
    }

    fn should_exit(&self) -> bool {
        if self.should_exit {
            // I think that self.midi or self.audio_output are causing the app
            // to hang randomly on exit. I'm going to keep this here to be
            // certain that the close code is really running.
            dbg!("Exiting now!");
        }
        self.should_exit
    }

    fn view(&self) -> Element<AppMessage> {
        match self.state {
            State::Idle => {}
            State::Playing => {}
        }

        let control_bar = self.control_bar.view(&self.clock_mirror);
        let project_view: Element<AppMessage> = container(text("coming soon")).into(); //self.orchestrator.view().map(Self::Message::GrooveMessage);
        let midi_view: Element<AppMessage> = container(text("coming soon")).into(); //self.midi.view().map(Self::Message::MidiHandlerMessage);
        let scrollable_content = column![midi_view, project_view];
        let under_construction = text("Under Construction").width(Length::FillPortion(1));
        let scrollable = container(scrollable(scrollable_content)).width(Length::FillPortion(1));
        let main_workspace = row![under_construction, scrollable];
        let content = column![control_bar, main_workspace]
            .align_items(Alignment::Center)
            .spacing(20);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .align_y(alignment::Vertical::Top)
            .into()
    }
}

impl GrooveApp {
    fn update_clock(&mut self) {
        // TODO law of demeter - should we be reaching in here, or just tell
        // ControlBar?
        self.control_bar
            .gui_clock
            .update(ClockMessage::Time(self.clock_mirror.seconds()));
        self.control_bar
            .gui_clock
            .update(ClockMessage::Beats(self.clock_mirror.beats()));
    }

    /// GrooveMessages returned in Orchestrator update() calls have made it all
    /// the way up to us. Let's look at them and decide what to do. If we have
    /// any work to do, we'll emit it in the form of one or more AppMessages.
    ///
    /// TODO: Should Orchestrator be handling these? Why did it give us commands
    /// to hand back to it?
    fn dispatch_groove_message(
        &mut self,
        clock: &Clock,
        message: GrooveMessage,
    ) -> Command<AppMessage> {
        todo!();
        // match self.orchestrator.update(clock, message).0 {
        //     Internal::None => Command::none(),
        //     Internal::Single(message) => self.handle_groove_message(clock, message),
        //     Internal::Batch(messages) => {
        //         Command::batch(messages.iter().fold(Vec::new(), |mut v, m| {
        //             v.push(self.handle_groove_message(clock, m.clone()));
        //             v
        //         }))
        //     }
        // }
    }
    fn handle_groove_message(
        &mut self,
        _clock: &Clock,
        message: GrooveMessage,
    ) -> Command<AppMessage> {
        match message {
            GrooveMessage::Nop => panic!(),
            GrooveMessage::Tick => panic!(),
            GrooveMessage::EntityMessage(_, _) => panic!(),
            GrooveMessage::MidiFromExternal(_, _) => panic!(),
            GrooveMessage::MidiToExternal(_channel, _message) => {
                panic!("This is handled in send_midi_handler_tick()")
            }
            GrooveMessage::AudioOutput(_) => {
                // IOHelper::fill_audio_buffer() should have already looked at
                // this
            }
            GrooveMessage::OutputComplete => {
                // IOHelper::fill_audio_buffer() should have already looked at
                // this
            }
            GrooveMessage::LoadProject(_) => todo!(),
        }
        Command::none()
    }

    // // TODO: these conversion routines are getting tedious. I'm not yet
    // // convinced they're in the right place, rather than just being a very
    // // expensive band-aid to patch holes.
    // fn send_midi_handler_tick(&mut self) {
    //     match self.midi.update(&self.clock, MidiHandlerMessage::Tick).0 {
    //         Internal::None => {}
    //         Internal::Single(_) => {
    //             // This doesn't happen, currently
    //             panic!("maybe it does happen after all");
    //         }
    //         Internal::Batch(messages) => {
    //             let mut saw_midi_message = false;
    //             for message in messages {
    //                 match message {
    //                     MidiHandlerMessage::MidiToExternal(channel, message) => {
    //                         // self.orchestrator.update(
    //                         //     &self.clock,
    //                         //     GrooveMessage::MidiFromExternal(channel, message),
    //                         // );
    //                         todo!();
    //                         saw_midi_message = true;
    //                     }
    //                     _ => todo!(),
    //                 }
    //             }
    //             if saw_midi_message {
    //                 self.midi
    //                     .update(&self.clock, MidiHandlerMessage::Activity(Instant::now()));
    //             }
    //         }
    //     }
    //}
}

pub fn main() -> iced::Result {
    GrooveApp::run(Settings {
        // This override is needed so that we get the CloseRequested event that
        // we need to turn off the cpal Stream. See
        // https://github.com/iced-rs/iced/blob/master/examples/events/ for an
        // example.
        exit_on_close_request: false,
        ..Settings::default()
    })
}
