#![windows_subsystem = "windows"]

mod gui;

use groove::{
    gui::{GrooveEvent, GrooveInput, GuiStuff, NUMBERS_FONT, NUMBERS_FONT_SIZE},
    Clock, GrooveOrchestrator, GrooveSubscription, MidiHandlerEvent, MidiHandlerInput,
    MidiHandlerMessage, MidiSubscription,
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
    orchestrator_sender: Option<mpsc::Sender<GrooveInput>>,
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
            orchestrator_sender: Default::default(),
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
    GrooveEvent(GrooveEvent),
    MidiHandlerMessage(MidiHandlerMessage),
    MidiHandlerEvent(MidiHandlerEvent),
    Tick(Instant),
    Event(iced::Event),
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
                container(self.gui_clock.view(clock)).width(Length::FillPortion(1)),
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

#[derive(Debug, Default, Clone)]
struct GuiClock {}

impl GuiClock {
    pub fn view(&self, clock: &Clock) -> Element<AppMessage> {
        let time_counter = {
            let minutes: u8 = (clock.seconds() / 60.0).floor() as u8;
            let seconds = clock.seconds() as usize % 60;
            let thousandths = (clock.seconds().fract() * 1000.0) as u16;
            container(
                text(format!("{minutes:02}:{seconds:02}:{thousandths:03}"))
                    .font(NUMBERS_FONT)
                    .size(NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<AppMessage>::number_box_style(&Theme::Dark),
            ))
        };

        let time_signature = clock.settings().time_signature();
        let time_signature_view = {
            container(column![
                text(format!("{}", time_signature.top)),
                text(format!("{}", time_signature.bottom))
            ])
        };

        let beat_counter = {
            let denom = time_signature.top as f32;

            let measures = (clock.beats() / denom) as usize;
            let beats = (clock.beats() % denom) as usize;
            let fractional = (clock.beats().fract() * 10000.0) as usize;
            container(
                text(format!("{measures:04}m{beats:02}b{fractional:03}"))
                    .font(NUMBERS_FONT)
                    .size(NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<AppMessage>::number_box_style(&Theme::Dark),
            ))
        };
        row![time_counter, time_signature_view, beat_counter].into()
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
        self.project_name.clone()
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
                    clock_mirror: clock,
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
                        self.post_to_orchestrator(GrooveInput::Restart);
                        self.reached_end_of_playback = false;
                    } else {
                        self.post_to_orchestrator(GrooveInput::Play);
                    }
                    self.state = State::Playing
                }
                ControlBarMessage::Stop => {
                    self.post_to_orchestrator(GrooveInput::Pause);
                    self.reached_end_of_playback = false;
                    match self.state {
                        State::Idle => {
                            self.post_to_orchestrator(GrooveInput::Restart);
                        }
                        State::Playing => self.state = State::Idle,
                    }
                }
                ControlBarMessage::SkipToStart => todo!(),
            },
            AppMessage::ControlBarBpm(_new_value) => {
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

                    self.post_to_midi_handler(MidiHandlerInput::QuitRequested);
                    self.should_exit = true;
                }
            }
            // AppMessage::GrooveMessage(message) => {
            //     // TODO: we're swallowing the Responses we're getting from
            //     // update()
            //     //
            //     // TODO: is this clock the right one to base everything off?
            //     // TODO: aaaargh so much cloning
            //     self.dispatch_groove_message(&self.clock_mirror.clone(), message);
            //     todo!();
            // }
            AppMessage::MidiHandlerMessage(message) => match message {
                // MidiHandlerMessage::InputSelected(which) => self.midi.select_input(which),
                // MidiHandlerMessage::OutputSelected(which) => self.midi.select_output(which),
                _ => todo!(),
            },
            AppMessage::GrooveEvent(event) => match event {
                GrooveEvent::Ready(sender, orchestrator) => {
                    self.orchestrator_sender = Some(sender);
                    self.orchestrator = orchestrator;

                    self.post_to_orchestrator(GrooveInput::LoadProject("low-cpu.yaml".to_string()));
                }
                GrooveEvent::ClockUpdate(samples) => self.clock_mirror.set_samples(samples),
                GrooveEvent::MidiToExternal(channel, message) => {
                    self.post_to_midi_handler(MidiHandlerInput::MidiMessage(channel, message));
                }
                GrooveEvent::AudioOutput(_) => todo!(),
                GrooveEvent::OutputComplete => todo!(),
                GrooveEvent::Quit => todo!(),
                GrooveEvent::ProjectLoaded(filename) => self.project_name = filename,
            },
            AppMessage::MidiHandlerEvent(event) => match event {
                MidiHandlerEvent::Ready(sender) => {
                    self.midi_handler_sender = Some(sender);
                    // TODO: now that we can talk to the midi handler, we should ask it for inputs and outputs.
                }
                MidiHandlerEvent::MidiMessage(_, _) => todo!(),
                MidiHandlerEvent::Quit => {
                    todo!("If we were waiting for this to shut down, then record that we're ready");
                }
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
    fn post_to_midi_handler(&mut self, input: MidiHandlerInput) {
        if let Some(sender) = self.midi_handler_sender.as_mut() {
            // TODO: deal with this
            let _ = sender.try_send(input);
        }
    }

    fn post_to_orchestrator(&mut self, input: GrooveInput) {
        if let Some(sender) = self.orchestrator_sender.as_mut() {
            // TODO: deal with this
            let _ = sender.try_send(input);
        }
    }
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
