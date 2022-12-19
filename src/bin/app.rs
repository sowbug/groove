#![windows_subsystem = "windows"]

mod gui;

use groove::{
    gui::{GrooveEvent, GrooveInput, GuiStuff, NUMBERS_FONT, NUMBERS_FONT_SIZE},
    traits::{HasUid, TestController, TestEffect, TestInstrument},
    AdsrEnvelope, Arpeggiator, AudioSource, BeatSequencer, BiQuadFilter, Bitcrusher, BoxedEntity,
    Chorus, Clock, ControlTrip, Delay, DrumkitSampler, EntityMessage, Gain, GrooveMessage,
    GrooveOrchestrator, GrooveSubscription, Limiter, MidiHandler, MidiHandlerEvent,
    MidiHandlerInput, MidiHandlerMessage, MidiSubscription, MidiTickSequencer, Mixer, Note,
    Oscillator, Pattern, PatternManager, PatternMessage, Reverb, Sampler, TestLfo, TestSynth,
    Timer, WelshSynth,
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
    widget::{button, column, container, pick_list, row, scrollable, slider, text, text_input},
    Alignment, Application, Command, Element, Length, Settings, Subscription,
};
use iced_audio::{HSlider, Normal, NormalParam};
use iced_native::{window, Event};
use std::{
    any::type_name,
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
    midi_handler: Arc<Mutex<MidiHandler>>,
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
            midi_handler: Default::default(),
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
                MidiHandlerMessage::InputSelected(which) => {
                    self.post_to_midi_handler(MidiHandlerInput::SelectMidiInput(which));
                }
                MidiHandlerMessage::OutputSelected(which) => {
                    self.post_to_midi_handler(MidiHandlerInput::SelectMidiOutput(which));
                }
                MidiHandlerMessage::Tick => todo!(),
                MidiHandlerMessage::Midi(_, _) => {
                    panic!("We send this. A coding error exists if we receive it.")
                }
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
                    self.post_to_midi_handler(MidiHandlerInput::Midi(channel, message));
                }
                GrooveEvent::AudioOutput(_) => todo!(),
                GrooveEvent::OutputComplete => todo!(),
                GrooveEvent::Quit => todo!(),
                GrooveEvent::ProjectLoaded(filename) => self.project_name = filename,
            },
            AppMessage::MidiHandlerEvent(event) => match event {
                MidiHandlerEvent::Ready(sender, midi_handler) => {
                    self.midi_handler_sender = Some(sender);
                    self.midi_handler = midi_handler;
                }
                #[allow(unused_variables)]
                MidiHandlerEvent::Midi(channel, event) => {
                    // TODO
                }
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
    }

    fn entity_view(&self, entity: &BoxedEntity) -> Element<EntityMessage> {
        match entity {
            BoxedEntity::AdsrEnvelope(e) => GuiStuff::titled_container(
                type_name::<AdsrEnvelope>(),
                GuiStuff::container_text(format!("Coming soon: {}", e.uid()).as_str()),
            )
            .into(),
            BoxedEntity::Arpeggiator(_) => {
                let title = type_name::<Arpeggiator>();
                let contents = "Hello!";
                GuiStuff::titled_container(title, GuiStuff::container_text(contents))
            }
            BoxedEntity::AudioSource(e) => self.audio_source_view(e),
            BoxedEntity::BeatSequencer(e) => self.beat_sequencer_view(e),
            BoxedEntity::BiQuadFilter(e) => self.biquad_filter_view(e),
            BoxedEntity::Bitcrusher(e) => self.bitcrusher_view(e),
            BoxedEntity::Chorus(e) => GuiStuff::titled_container(
                type_name::<Chorus>(),
                GuiStuff::container_text(format!("Coming soon: {}", e.uid()).as_str()),
            )
            .into(),
            BoxedEntity::ControlTrip(e) => GuiStuff::titled_container(
                type_name::<ControlTrip<EntityMessage>>(),
                GuiStuff::container_text(format!("Coming soon: {}", e.uid()).as_str()),
            )
            .into(),
            BoxedEntity::Delay(e) => {
                let title = type_name::<Delay>();
                let contents = format!("delay in seconds: {}", e.delay_seconds());
                GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
            }
            BoxedEntity::DrumkitSampler(e) => {
                let title = type_name::<DrumkitSampler>();
                let contents = format!("kit name: {}", e.kit_name());
                GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
            }
            BoxedEntity::Gain(e) => self.gain_view(e),
            BoxedEntity::Limiter(e) => {
                let title = type_name::<Limiter>();
                let contents = format!("min: {} max: {}", e.min(), e.max());
                GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
            }
            BoxedEntity::MidiTickSequencer(e) => GuiStuff::titled_container(
                type_name::<MidiTickSequencer<EntityMessage>>(),
                GuiStuff::container_text(format!("Coming soon: {}", e.uid()).as_str()),
            )
            .into(),
            BoxedEntity::Mixer(e) => GuiStuff::titled_container(
                type_name::<Mixer<EntityMessage>>(),
                text(format!("Mixer {} coming soon", e.uid())).into(),
            )
            .into(),
            BoxedEntity::Oscillator(e) => GuiStuff::titled_container(
                type_name::<Oscillator>(),
                GuiStuff::container_text(format!("Coming soon: {}", e.uid()).as_str()),
            )
            .into(),
            BoxedEntity::PatternManager(e) => self.pattern_manager_view(e),
            BoxedEntity::Reverb(e) => GuiStuff::titled_container(
                type_name::<Reverb>(),
                GuiStuff::container_text(format!("Coming soon: {}", e.uid()).as_str()),
            )
            .into(),
            BoxedEntity::Sampler(e) => {
                let title = type_name::<Sampler>();
                let contents = format!("name: {}", e.filename());
                GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
            }
            BoxedEntity::TestController(e) => self.test_controller_view(e),
            BoxedEntity::TestEffect(e) => self.test_effect_view(e),
            BoxedEntity::TestInstrument(e) => self.test_instrument_view(e),
            BoxedEntity::TestLfo(e) => self.test_lfo_view(e),
            BoxedEntity::TestSynth(e) => self.test_synth_view(e),
            BoxedEntity::Timer(e) => self.timer_view(e),
            BoxedEntity::WelshSynth(e) => {
                let options = vec!["Acid Bass".to_string(), "Piano".to_string()];
                GuiStuff::titled_container(
                    type_name::<WelshSynth>(),
                    container(column![
                        text(format!("Welsh {} {} coming soon", e.uid(), e.preset_name())),
                        pick_list(options, None, EntityMessage::PickListSelected,)
                    ])
                    .into(),
                )
                .into()
            }
        }
    }

    fn midi_view(&self) -> Element<MidiHandlerMessage> {
        if let Ok(midi_handler) = self.midi_handler.lock() {
            let activity_text = container(text(
                if Instant::now().duration_since(midi_handler.activity_tick())
                    > Duration::from_millis(250)
                {
                    " "
                } else {
                    "•"
                },
            ))
            .width(iced::Length::FillPortion(1));
            let (input_selected, input_options) =
                midi_handler.midi_input().as_ref().unwrap().labels();
            let input_menu = row![
                text("Input").width(iced::Length::FillPortion(1)),
                pick_list(
                    input_options,
                    input_selected.clone(),
                    MidiHandlerMessage::InputSelected,
                )
                .width(iced::Length::FillPortion(3))
            ];
            let (output_selected, output_options) =
                midi_handler.midi_output().as_ref().unwrap().labels();
            let output_menu = row![
                text("Output").width(iced::Length::FillPortion(1)),
                pick_list(
                    output_options,
                    output_selected.clone(),
                    MidiHandlerMessage::OutputSelected,
                )
                .width(iced::Length::FillPortion(3))
            ];
            let port_menus =
                container(column![input_menu, output_menu]).width(iced::Length::FillPortion(7));
            GuiStuff::titled_container("MIDI", container(row![activity_text, port_menus]).into())
        } else {
            panic!()
        }
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
        let title = type_name::<BeatSequencer<EntityMessage>>();
        let contents = format!("{}", e.next_instant());
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }

    fn test_controller_view(&self, e: &TestController<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<TestController<EntityMessage>>(),
            GuiStuff::container_text(format!("Tempo: {}", e.tempo).as_str()),
        )
    }

    fn test_effect_view(&self, e: &TestEffect<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<TestEffect<EntityMessage>>(),
            GuiStuff::container_text(format!("Value: {}", e.my_value()).as_str()),
        )
    }

    fn test_instrument_view(&self, e: &TestInstrument<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<TestInstrument<EntityMessage>>(),
            GuiStuff::container_text(format!("Fake value: {}", e.fake_value()).as_str()),
        )
    }

    fn test_lfo_view(&self, e: &TestLfo<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<TestLfo<EntityMessage>>(),
            GuiStuff::container_text(
                format!("Frequency: {} current value: {}", e.frequency(), e.value()).as_str(),
            ),
        )
    }

    fn test_synth_view(&self, _: &TestSynth<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<TestSynth<EntityMessage>>(),
            GuiStuff::container_text(format!("Nothing").as_str()),
        )
    }

    fn timer_view(&self, e: &Timer<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<Timer<EntityMessage>>(),
            GuiStuff::container_text(format!("Runtime: {}", e.time_to_run_seconds()).as_str()),
        )
    }
    fn pattern_manager_view(&self, e: &PatternManager) -> Element<EntityMessage> {
        let title = type_name::<PatternManager>();
        let contents = {
            let pattern_views = e.patterns().iter().enumerate().map(|(i, item)| {
                self.pattern_view(item)
                    .map(move |message| EntityMessage::PatternMessage(i, message))
            });
            column(pattern_views.collect())
        };
        GuiStuff::titled_container(title, contents.into())
    }
    fn pattern_view(&self, e: &Pattern<Note>) -> Element<PatternMessage> {
        let mut note_rows = Vec::new();
        for track in e.notes.iter() {
            let mut note_row = Vec::new();
            for note in track {
                let cell = text(format!("{:02} ", note.key).to_string());
                note_row.push(cell.into());
            }
            let row_note_row = row(note_row).into();
            note_rows.push(row_note_row);
        }
        column(vec![
            button(text(format!("{:?}", e.note_value)))
                .on_press(PatternMessage::ButtonPressed)
                .into(),
            column(note_rows).into(),
        ])
        .into()
    }
    fn audio_source_view(&self, e: &AudioSource<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<AudioSource<EntityMessage>>(),
            GuiStuff::container_text(format!("Coming soon: {}", e.uid()).as_str()),
        )
        .into()
    }

    fn gain_view(&self, e: &Gain<EntityMessage>) -> Element<EntityMessage> {
        let title = format!("{}: {}", type_name::<Gain<EntityMessage>>(), e.ceiling());
        let slider = HSlider::new(
            NormalParam {
                value: Normal::from_clipped(e.ceiling()),
                default: Normal::from_clipped(1.0),
            },
            EntityMessage::HSliderInt,
        );
        let contents = container(row![slider]).padding(20);
        GuiStuff::titled_container(&title, contents.into())
    }

    fn biquad_filter_view(&self, e: &BiQuadFilter<EntityMessage>) -> Element<EntityMessage> {
        let title = type_name::<BiQuadFilter<EntityMessage>>();
        let contents = row![
            container(slider(
                0..=100,
                (e.cutoff_pct() * 100.0) as u8,
                EntityMessage::UpdateParam1U8 // CutoffPct
            ))
            .width(iced::Length::FillPortion(1)),
            container(GuiStuff::container_text(
                format!("cutoff: {}Hz", e.cutoff_hz()).as_str()
            ))
            .width(iced::Length::FillPortion(1))
        ];
        GuiStuff::titled_container(title, contents.into())
    }

    fn bitcrusher_view(&self, e: &Bitcrusher) -> Element<EntityMessage> {
        let title = format!("{}: {}", type_name::<Bitcrusher>(), e.bits_to_crush());
        let contents = container(row![HSlider::new(
            e.int_range().normal_param(e.bits_to_crush() as i32, 8),
            EntityMessage::HSliderInt
        )])
        .padding(20);
        GuiStuff::titled_container(&title, contents.into())
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
