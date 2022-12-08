#![windows_subsystem = "windows"]

mod gui;

use async_std::task::block_on;
use groove::{
    gui::{GuiStuff, Viewable, NUMBERS_FONT, NUMBERS_FONT_SIZE},
    traits::{Internal, Updateable},
    AudioOutput, Clock, GrooveMessage, GrooveOrchestrator, IOHelper, MidiHandler,
    MidiHandlerMessage, TimeSignature,
};
use gui::{
    persistence::{LoadError, SavedState},
    play_icon, skip_to_prev_icon, stop_icon,
};
use iced::{
    alignment, executor,
    theme::{self, Theme},
    time,
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Application, Command, Element, Length, Settings, Subscription,
};
use iced_native::{window, Event};
use std::time::{Duration, Instant};

struct GrooveApp {
    // Overhead
    theme: Theme,
    state: State,
    should_exit: bool,
    last_tick: Instant, // A monotonically increasing value to tell when things have happened

    // UI components
    control_bar: ControlBar,

    // Model
    project_name: String,
    orchestrator: Box<GrooveOrchestrator>,
    clock: Clock,
    audio_output: AudioOutput,

    // This is true when playback went all the way to the end of the song. The
    // reason it's nice to track this is that after pressing play and listening
    // to the song, the user can press play again without manually resetting the
    // clock to the start. But we don't want to just reset the clock at the end
    // of playback, because that means the clock would read zero at the end of
    // playback, which is undesirable because it's natural to want to know how
    // long the song was after listening, and it's nice to be able to glance at
    // the stopped clock and get that answer.
    reached_end_of_playback: bool,

    // External interfaces
    midi: Box<MidiHandler>,
}

impl Default for GrooveApp {
    fn default() -> Self {
        Self {
            theme: Default::default(),
            state: Default::default(),
            should_exit: Default::default(),
            last_tick: Instant::now(),
            control_bar: Default::default(),
            project_name: Default::default(),
            orchestrator: Default::default(),
            clock: Default::default(),
            audio_output: Default::default(),
            reached_end_of_playback: Default::default(),
            midi: Default::default(),
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
    midi: Midi,
}

impl ControlBar {
    pub fn view(&self, clock: &Clock, last_tick: Instant) -> Element<AppMessage> {
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
                container(self.midi.view(last_tick)).width(Length::FillPortion(1)),
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

#[derive(Debug, Clone)]
pub enum MidiControlBarMessage {
    Inputs(Vec<(usize, String)>),
    Activity(Instant),
}

#[derive(Debug, Clone)]
struct Midi {
    inputs: Vec<(usize, String)>,
    activity_tick: Instant,
}

impl Default for Midi {
    fn default() -> Self {
        Self {
            inputs: Vec::default(),
            activity_tick: Instant::now(),
        }
    }
}

impl Midi {
    #[allow(dead_code)]
    pub fn update(&mut self, message: MidiControlBarMessage) {
        match message {
            MidiControlBarMessage::Inputs(inputs) => {
                self.inputs = inputs;
            }
            MidiControlBarMessage::Activity(now) => self.activity_tick = now,
        }
    }

    pub fn view(&self, last_tick: Instant) -> Element<AppMessage> {
        let mut s = String::new();
        for input in self.inputs.iter() {
            s = format!("{s} {}", input.1);
        }
        let input_dropdown = container(text(
            if last_tick.duration_since(self.activity_tick) > Duration::from_millis(250) {
                " "
            } else {
                "•"
            },
        ))
        .width(Length::FillPortion(1));
        let activity_indicator = container(text(s)).width(Length::FillPortion(7));
        row![input_dropdown, activity_indicator].into()
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
                last_tick: Instant::now(),
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
                *self = Self {
                    theme: self.theme.clone(),
                    project_name: state.project_name,
                    orchestrator: state.song_settings.instantiate(false).unwrap(),
                    midi: Default::default(),
                    ..Default::default()
                };
                self.audio_output.start();
                if let Err(err) = self.midi.start() {
                    println!("error starting MIDI: {}", err)
                }
            }
            AppMessage::Loaded(Err(_e)) => {
                todo!()
            }
            AppMessage::Tick(now) => {
                self.last_tick = now;
                if let State::Playing = &mut self.state {
                    self.update_clock();
                    let (messages, done) = block_on(IOHelper::fill_audio_buffer(
                        self.audio_output.recommended_buffer_size(),
                        &mut self.orchestrator,
                        &mut self.clock,
                        &mut self.audio_output,
                    ));
                    if done {
                        self.reached_end_of_playback = true;
                        self.state = State::Idle;
                    }
                    messages.iter().for_each(|m| {
                        if let GrooveMessage::MidiToExternal(channel, message) = *m {
                            self.midi.update(
                                &self.clock,
                                MidiHandlerMessage::MidiToExternal(channel, message),
                            );
                        }
                    });
                }
                // TODO: decide what a Tick means. The app thinks it's 10
                // milliseconds. Orchestrator thinks it's 1/sample rate.

                // TODO: these conversion routines are getting tedious. I'm not
                // yet convinced they're in the right place, rather than just
                // being a very expensive band-aid to patch holes.
                match self.midi.update(&self.clock, MidiHandlerMessage::Tick).0 {
                    Internal::None => {}
                    Internal::Single(_) => {
                        // This doesn't happen, currently
                    }
                    Internal::Batch(messages) => {
                        for message in messages {
                            match message {
                                MidiHandlerMessage::MidiToExternal(channel, message) => {
                                    self.orchestrator.update(
                                        &self.clock,
                                        GrooveMessage::MidiFromExternal(channel, message),
                                    );
                                    self.control_bar
                                        .midi
                                        .update(MidiControlBarMessage::Activity(Instant::now()));
                                }
                                _ => todo!(),
                            }
                        }
                    }
                }
            }
            AppMessage::ControlBarMessage(message) => match message {
                // TODO: not sure if we need ticking for now. it's playing OR
                // midi
                ControlBarMessage::Play => {
                    if self.reached_end_of_playback {
                        self.clock.reset();
                        self.reached_end_of_playback = false;
                    }
                    self.state = State::Playing
                }
                ControlBarMessage::Stop => {
                    self.reached_end_of_playback = false;
                    match self.state {
                        State::Idle => {
                            self.clock.reset();
                            self.update_clock();
                        }
                        State::Playing => self.state = State::Idle,
                    }
                }
                ControlBarMessage::SkipToStart => todo!(),
            },
            AppMessage::ControlBarBpm(new_value) => {
                if let Ok(bpm) = new_value.parse() {
                    self.clock.settings_mut().set_bpm(bpm);
                }
            }
            AppMessage::Event(event) => {
                if let Event::Window(window::Event::CloseRequested) = event {
                    // See https://github.com/iced-rs/iced/pull/804 and
                    // https://github.com/iced-rs/iced/blob/master/examples/events/src/main.rs#L55
                    //
                    // This is needed to stop an ALSA buffer underrun on close
                    // TODO BROKEN self.midi.stop();
                    self.audio_output.stop();

                    self.should_exit = true;
                }
            }
            AppMessage::GrooveMessage(message) => {
                // TODO: we're swallowing the EvenNewerCommands we're getting
                // from update()
                //
                // TODO: is this clock the right one to base everything off?
                // TODO: aaaargh so much cloning
                self.dispatch_groove_message(&self.clock.clone(), message);
            }
            AppMessage::MidiHandlerMessage(message) => match message {
                MidiHandlerMessage::InputSelected(which) => self.midi.select_input(which),
                MidiHandlerMessage::OutputSelected(which) => self.midi.select_output(which),
                _ => todo!(),
            },
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<AppMessage> {
        Subscription::batch([
            iced_native::subscription::events().map(AppMessage::Event),
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

        let control_bar = self.control_bar.view(&self.clock, self.last_tick);
        let project_view = self.orchestrator.view().map(Self::Message::GrooveMessage);
        let midi_view = self.midi.view().map(Self::Message::MidiHandlerMessage);
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
            .update(ClockMessage::Time(self.clock.seconds()));
        self.control_bar
            .gui_clock
            .update(ClockMessage::Beats(self.clock.beats()));
    }

    /// GrooveMessages returned in Orchestrator update() calls have made it all
    /// the way up to us. Let's look at them and decide what to do. If we have
    /// any work to do, we'll emit it in the form of one or more AppMessages.
    fn dispatch_groove_message(
        &mut self,
        clock: &Clock,
        message: GrooveMessage,
    ) -> Command<AppMessage> {
        let mut v = Vec::new();
        match self.orchestrator.update(&self.clock, message).0 {
            Internal::None => {}
            Internal::Single(action) => v.push(self.handle_groove_message(clock, action)),
            Internal::Batch(actions) => {
                for action in actions {
                    v.push(self.handle_groove_message(clock, action))
                }
            }
        }
        Command::batch(v)
    }
    fn handle_groove_message(
        &mut self,
        clock: &Clock,
        message: GrooveMessage,
    ) -> Command<AppMessage> {
        match message {
            GrooveMessage::Nop => panic!(),
            GrooveMessage::Tick => panic!(),
            GrooveMessage::EntityMessage(_, _) => panic!(),
            GrooveMessage::MidiFromExternal(_, _) => panic!(),
            GrooveMessage::MidiToExternal(channel, message) => {
                self.midi
                    .handle_message(clock, MidiHandlerMessage::MidiToExternal(channel, message));
                Command::none()
            }
            GrooveMessage::AudioOutput(_) => {
                // IOHelper::fill_audio_buffer() should have already looked at this
                Command::none()
            }
            GrooveMessage::OutputComplete => {
                // IOHelper::fill_audio_buffer() should have already looked at this
                Command::none()
            }
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
