#![windows_subsystem = "windows"]

mod gui;

use groove::{
    gui::{GrooveEvent, GrooveInput, GuiStuff, NUMBERS_FONT, NUMBERS_FONT_SIZE},
    traits::{HasUid, TestController, TestEffect, TestInstrument},
    BeatSequencer, BoxedEntity, Clock, EntityMessage, GrooveMessage, GrooveOrchestrator,
    GrooveSubscription, MidiHandlerEvent, MidiHandlerInput, MidiHandlerMessage, MidiSubscription,
    TestLfo, TestSynth, Timer,
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
    widget::{button, column, container, pick_list, row, scrollable, text, text_input},
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
    GrooveMessage(GrooveMessage),
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
    Bpm(String),
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
                ControlBarMessage::Bpm(value) => {
                    if let Ok(bpm) = value.parse() {
                        self.post_to_orchestrator(GrooveInput::SetBpm(bpm));
                    }
                }
            },
            AppMessage::Event(event) => {
                if let Event::Window(window::Event::CloseRequested) = event {
                    // See https://github.com/iced-rs/iced/pull/804 and
                    // https://github.com/iced-rs/iced/blob/master/examples/events/src/main.rs#L55
                    //
                    // This is needed to stop an ALSA buffer underrun on close

                    self.post_to_midi_handler(MidiHandlerInput::QuitRequested);
                    self.post_to_orchestrator(GrooveInput::QuitRequested);
                    self.should_exit = true;
                }
            }
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
                GrooveEvent::SetClock(samples) => self.clock_mirror.set_samples(samples),
                GrooveEvent::SetBpm(bpm) => self.clock_mirror.set_bpm(bpm),
                GrooveEvent::SetTimeSignature(time_signature) => {
                    self.clock_mirror.set_time_signature(time_signature)
                }
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
                MidiHandlerEvent::MidiMessage(_, _) => todo!("we got a MIDI message from outside"),
                MidiHandlerEvent::Quit => {
                    todo!("If we were waiting for this to shut down, then record that we're ready");
                }
            },
            AppMessage::GrooveMessage(message) => match message {
                GrooveMessage::Nop => todo!(),
                GrooveMessage::Tick => todo!(),
                GrooveMessage::EntityMessage(uid, message) => {
                    self.update_entity(uid, message);
                }
                GrooveMessage::MidiFromExternal(_, _) => todo!(),
                GrooveMessage::MidiToExternal(_, _) => todo!(),
                GrooveMessage::AudioOutput(_) => todo!(),
                GrooveMessage::OutputComplete => todo!(),
                GrooveMessage::LoadProject(_) => todo!(),
                GrooveMessage::LoadedProject(_) => todo!(),
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
        // Ideally we won't set this until all tasks (orchestrator thread, MIDI
        // interface thread, etc.) have been asked to shut down. If we forget,
        // then the Iced process hangs after the main window disappears.
        self.should_exit
    }

    fn view(&self) -> Element<AppMessage> {
        match self.state {
            State::Idle => {}
            State::Playing => {}
        }

        let control_bar = self.control_bar_view().map(AppMessage::ControlBarMessage);
        let project_view: Element<AppMessage> =
            self.orchestrator_view().map(AppMessage::GrooveMessage);
        let midi_view: Element<AppMessage> = self.midi_view().map(AppMessage::MidiHandlerMessage);
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

    fn update_entity(&mut self, uid: usize, message: EntityMessage) {
        if let Ok(mut o) = self.orchestrator.lock() {
            if let Some(entity) = o.store_mut().get_mut(uid) {
                // TODO: we don't have a real clock here... solve this.
                entity
                    .as_updateable_mut()
                    .update(&self.clock_mirror, message);
            }
        }
    }

    fn orchestrator_view(&self) -> Element<GrooveMessage> {
        if let Ok(orchestrator) = self.orchestrator.lock() {
            let views = orchestrator
                .store()
                .iter()
                .fold(Vec::new(), |mut v, (&uid, e)| {
                    v.push(
                        self.entity_view(e)
                            .map(move |message| GrooveMessage::EntityMessage(uid, message)),
                    );
                    v
                });
            column(views).into()
        } else {
            panic!()
        }
        //        let pattern_view = self.pattern_manager().view();
    }

    fn entity_view(&self, entity: &BoxedEntity) -> Element<EntityMessage> {
        match entity {
            BoxedEntity::AdsrEnvelope(e) => todo!(),
            BoxedEntity::Arpeggiator(_) => todo!(),
            BoxedEntity::AudioSource(_) => todo!(),
            BoxedEntity::BeatSequencer(e) => self.beat_sequencer_view(e),
            BoxedEntity::BiQuadFilter(_) => todo!(),
            BoxedEntity::Bitcrusher(_) => todo!(),
            BoxedEntity::ControlTrip(_) => todo!(),
            BoxedEntity::Delay(_) => todo!(),
            BoxedEntity::DrumkitSampler(_) => todo!(),
            BoxedEntity::Gain(_) => todo!(),
            BoxedEntity::Limiter(_) => todo!(),
            BoxedEntity::MidiTickSequencer(_) => todo!(),
            BoxedEntity::Mixer(e) => {
                container(text(format!("Mixer {} coming soon", e.uid()))).into()
            }
            BoxedEntity::Oscillator(_) => todo!(),
            BoxedEntity::PatternManager(_) => todo!(),
            BoxedEntity::Reverb(_) => todo!(),
            BoxedEntity::Sampler(_) => todo!(),
            BoxedEntity::TestController(e) => self.test_controller_view(e),
            BoxedEntity::TestEffect(e) => self.test_effect_view(e),
            BoxedEntity::TestInstrument(e) => self.test_instrument_view(e),
            BoxedEntity::TestLfo(e) => self.test_lfo_view(e),
            BoxedEntity::TestSynth(e) => self.test_synth_view(e),
            BoxedEntity::Timer(e) => self.timer_view(e),
            BoxedEntity::WelshSynth(e) => {
                let options = vec!["Acid Bass".to_string(), "Piano".to_string()];
                container(column![
                    text(format!("Welsh {} {} coming soon", e.uid(), e.preset_name())),
                    pick_list(options, None, EntityMessage::PickListSelected,)
                ])
                .into()
            }
            _ => container(text("Coming soon")).into(),
        }
    }

    fn midi_view(&self) -> Element<MidiHandlerMessage> {
        container(text("MIDI coming soon")).into()
    }

    fn control_bar_view(&self) -> Element<ControlBarMessage> {
        container(
            row![
                text_input(
                    "BPM",
                    self.clock_mirror.bpm().round().to_string().as_str(),
                    ControlBarMessage::Bpm
                )
                .width(Length::Units(60)),
                container(row![
                    button(skip_to_prev_icon())
                        .width(Length::Units(32))
                        .on_press(ControlBarMessage::SkipToStart),
                    button(play_icon())
                        .width(Length::Units(32))
                        .on_press(ControlBarMessage::Play),
                    button(stop_icon())
                        .width(Length::Units(32))
                        .on_press(ControlBarMessage::Stop)
                ])
                .align_x(alignment::Horizontal::Center)
                .width(Length::FillPortion(1)),
                container(self.clock_view()).width(Length::FillPortion(1)),
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

    fn clock_view(&self) -> Element<ControlBarMessage> {
        let time_counter = {
            let minutes: u8 = (self.clock_mirror.seconds() / 60.0).floor() as u8;
            let seconds = self.clock_mirror.seconds() as usize % 60;
            let thousandths = (self.clock_mirror.seconds().fract() * 1000.0) as u16;
            container(
                text(format!("{minutes:02}:{seconds:02}:{thousandths:03}"))
                    .font(NUMBERS_FONT)
                    .size(NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<ControlBarMessage>::number_box_style(&Theme::Dark),
            ))
        };

        let time_signature = self.clock_mirror.settings().time_signature();
        let time_signature_view = {
            container(column![
                text(format!("{}", time_signature.top)),
                text(format!("{}", time_signature.bottom))
            ])
        };

        let beat_counter = {
            let denom = time_signature.top as f32;

            let measures = (self.clock_mirror.beats() / denom) as usize;
            let beats = (self.clock_mirror.beats() % denom) as usize;
            let fractional = (self.clock_mirror.beats().fract() * 10000.0) as usize;
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

    fn beat_sequencer_view(&self, e: &BeatSequencer<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container("Sequencer", text(format!("{}", e.next_instant())).into())
    }

    fn test_controller_view(&self, e: &TestController<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container("TestController", text(format!("Tempo: {}", e.tempo)).into())
    }

    fn test_effect_view(&self, e: &TestEffect<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            "TestEffect",
            text(format!("Value: {}", e.my_value())).into(),
        )
    }

    fn test_instrument_view(&self, e: &TestInstrument<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            "TestInstrument",
            text(format!("Fake value: {}", e.fake_value())).into(),
        )
    }

    fn test_lfo_view(&self, e: &TestLfo<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            "TestLfo",
            text(format!(
                "Frequency: {} current value: {}",
                e.frequency(),
                e.value()
            ))
            .into(),
        )
    }

    fn test_synth_view(&self, e: &TestSynth<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container("TestSynth", text(format!("Nothing")).into())
    }

    fn timer_view(&self, e: &Timer<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            "Timer",
            text(format!("Runtime: {}", e.time_to_run_seconds())).into(),
        )
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
