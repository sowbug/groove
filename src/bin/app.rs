#![windows_subsystem = "windows"]

mod gui;

use groove::{
    app_version,
    gui::{GrooveEvent, GrooveInput},
    traits::{HasUid, TestController, TestEffect, TestInstrument},
    Arpeggiator, AudioSource, BeatSequencer, BiQuadFilter, Bitcrusher, BoxedEntity, Chorus, Clock,
    Compressor, ControlTrip, Delay, DrumkitSampler, EntityMessage, F32ControlValue, FmSynthesizer,
    Gain, GrooveMessage, GrooveSubscription, LfoController, Limiter, MidiHandler, MidiHandlerEvent,
    MidiHandlerInput, MidiHandlerMessage, MidiSubscription, MidiTickSequencer, Mixer, Normal, Note,
    Orchestrator, Pattern, PatternManager, PatternMessage, Reverb, Sampler, SimpleSynthesizer,
    TestSynth, Timer, WelshSynth,
};
use gui::{
    persistence::{LoadError, Preferences, SaveError},
    play_icon, skip_to_prev_icon, stop_icon, GuiStuff,
};
use iced::{
    alignment, executor,
    futures::channel::mpsc,
    theme::{self, Theme},
    widget::{button, column, container, pick_list, row, scrollable, text, text_input},
    window, Alignment, Application, Command, Element, Length, Settings, Subscription,
};
use iced_audio::{HSlider, Knob, Normal as IcedNormal, NormalParam};
use iced_native::Event;
use rustc_hash::FxHashMap;
use std::{
    any::type_name,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Default, PartialEq)]
enum EntityViewState {
    #[default]
    Collapsed,
    Expanded,
}

#[derive(Default, Debug)]
enum MainViews {
    #[default]
    Unstructured,
    Session,
    Arrangement,
    Preferences,
}

struct GrooveApp {
    // Overhead
    preferences: Preferences,
    is_pref_load_complete: bool,
    theme: Theme,
    state: State,

    // We won't set this until all tasks (orchestrator thread, MIDI interface
    // thread, etc.) have been asked to shut down. If we forget, then the Iced
    // process hangs after the main window disappears.
    should_exit: bool,

    // View
    current_view: MainViews,

    // Model
    project_title: Option<String>,
    orchestrator_sender: Option<mpsc::Sender<GrooveInput>>,
    orchestrator: Arc<Mutex<Orchestrator>>,
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
    midi_handler: Option<Arc<Mutex<MidiHandler>>>,

    entity_view_states: FxHashMap<usize, EntityViewState>,
}

impl Default for GrooveApp {
    fn default() -> Self {
        // TODO: these are (probably) temporary until the project is
        // loaded. Make sure they really need to be instantiated.
        let clock = Clock::default();
        let orchestrator = Orchestrator::new_with(clock.settings());
        Self {
            preferences: Default::default(),
            is_pref_load_complete: false,
            theme: Default::default(),
            state: Default::default(),
            should_exit: Default::default(),
            current_view: Default::default(),
            project_title: None,
            orchestrator_sender: Default::default(),
            orchestrator: Arc::new(Mutex::new(orchestrator)),
            clock_mirror: clock,
            reached_end_of_playback: Default::default(),
            midi_handler_sender: Default::default(),
            midi_handler: Default::default(),
            entity_view_states: Default::default(),
        }
    }
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Playing,
}

#[derive(Clone, Debug)]
pub enum AppMessage {
    PrefsLoaded(Result<Preferences, LoadError>),
    PrefsSaved(Result<(), SaveError>),
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
            Command::perform(Preferences::load_prefs(), AppMessage::PrefsLoaded),
        )
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }

    fn title(&self) -> String {
        if let Some(title) = &self.project_title {
            title.clone()
        } else {
            String::from("New Project")
        }
    }

    fn update(&mut self, message: AppMessage) -> Command<AppMessage> {
        match message {
            AppMessage::PrefsLoaded(Ok(preferences)) => {
                self.preferences = preferences;
                self.is_pref_load_complete = true;
            }
            AppMessage::PrefsLoaded(Err(_)) => {
                self.is_pref_load_complete = true;
                self.preferences = Preferences::default();
            }
            AppMessage::Tick(_now) => {
                // TODO: do we still need a tick?
            }
            AppMessage::ControlBarMessage(message) => match message {
                // TODO: not sure if we need ticking for now. it's playing OR
                // midi
                ControlBarMessage::Play => {
                    if self.reached_end_of_playback {
                        self.post_to_orchestrator(GrooveInput::SkipToStart);
                        self.reached_end_of_playback = false;
                    }
                    self.post_to_orchestrator(GrooveInput::Play);
                    self.state = State::Playing
                }
                ControlBarMessage::Stop => {
                    self.post_to_orchestrator(GrooveInput::Pause);
                    self.reached_end_of_playback = false;
                    match self.state {
                        State::Idle => {
                            self.post_to_orchestrator(GrooveInput::SkipToStart);
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
                    return self.handle_close_requested_event();
                }
                if let Event::Keyboard(e) = event {
                    self.handle_keyboard_event(e);
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

                    // We don't start the GrooveSubscription until prefs are
                    // done loading, so this boolean and the corresponding
                    // filename should be set by the time we look at it.
                    if self.preferences.should_reload_last_project {
                        if let Some(last_project_filename) = &self.preferences.last_project_filename
                        {
                            self.post_to_orchestrator(GrooveInput::LoadProject(
                                last_project_filename.to_string(),
                            ));
                        }
                    }
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
                GrooveEvent::OutputComplete => {
                    self.reached_end_of_playback = true;
                    self.state = State::Idle;
                }
                GrooveEvent::Quit => todo!(),
                GrooveEvent::ProjectLoaded(filename, title) => {
                    self.preferences.last_project_filename = Some(filename);
                    self.project_title = title;
                }
            },
            AppMessage::MidiHandlerEvent(event) => match event {
                MidiHandlerEvent::Ready(sender, midi_handler) => {
                    self.midi_handler_sender = Some(sender);
                    self.midi_handler = Some(midi_handler);
                }
                #[allow(unused_variables)]
                MidiHandlerEvent::Midi(channel, event) => {
                    // TODO
                }
                MidiHandlerEvent::Quit => {
                    // TODO: If we were waiting for this to shut down, then
                    // record that we're ready. For now, it's nice to know, but
                    // we won't do anything about it.
                }
            },
            AppMessage::GrooveMessage(message) => match message {
                GrooveMessage::EntityMessage(uid, message) => match message {
                    EntityMessage::ExpandPressed => {
                        // Find whoever else is expanded and maybe collapse them
                        self.set_entity_view_state(uid, EntityViewState::Expanded);
                    }
                    EntityMessage::CollapsePressed => {
                        self.set_entity_view_state(uid, EntityViewState::Collapsed);
                    }
                    _ => {
                        self.entity_update(uid, message);
                    }
                },
                _ => todo!(),
            },
            AppMessage::PrefsSaved(r) => {
                if self.should_exit {
                    return window::close::<Self::Message>();
                } else {
                    match r {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }
            }
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<AppMessage> {
        let mut v = vec![
            iced_native::subscription::events().map(AppMessage::Event),
            MidiSubscription::subscription().map(AppMessage::MidiHandlerEvent),
        ];
        if self.is_pref_load_complete {
            v.push(GrooveSubscription::subscription().map(AppMessage::GrooveEvent));
        }
        Subscription::batch(v)
    }

    fn view(&self) -> Element<AppMessage> {
        match self.state {
            State::Idle | State::Playing => {}
        }

        let control_bar = self.control_bar_view().map(AppMessage::ControlBarMessage);
        let under_construction = container(GuiStuff::<EntityMessage>::container_text(
            "Under Construction",
        ))
        .width(Length::FillPortion(1))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center);
        let main_content = match self.current_view {
            MainViews::Unstructured => {
                let project_view: Element<AppMessage> =
                    self.orchestrator_view().map(AppMessage::GrooveMessage);
                let midi_view: Element<AppMessage> =
                    self.midi_view().map(AppMessage::MidiHandlerMessage);
                let scrollable_content = column![midi_view, project_view];
                let scrollable =
                    container(scrollable(scrollable_content)).width(Length::FillPortion(1));
                row![under_construction, scrollable]
            }
            MainViews::Session => {
                row![under_construction]
            }
            MainViews::Arrangement => {
                row![under_construction]
            }
            MainViews::Preferences => {
                row![under_construction]
            }
        };
        let full_view = column![control_bar, main_content]
            .align_items(Alignment::Center)
            .spacing(20);

        container(full_view)
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

    fn entity_update(&mut self, uid: usize, message: EntityMessage) {
        if let Ok(mut o) = self.orchestrator.lock() {
            if let Some(entity) = o.store_mut().get_mut(uid) {
                // TODO: we don't have a real clock here... solve this.
                match entity {
                    BoxedEntity::BiQuadFilter(e) => match message {
                        EntityMessage::HSliderInt(value) => {
                            e.set_cutoff_pct(value.as_f32());
                        }
                        _ => todo!(),
                    },
                    BoxedEntity::Bitcrusher(e) => match message {
                        EntityMessage::HSliderInt(value) => {
                            e.set_bits_to_crush(
                                e.bits_to_crush_int_range().unmap_to_value(value) as u8
                            );
                        }
                        _ => todo!(),
                    },
                    BoxedEntity::Compressor(e) => match message {
                        EntityMessage::HSliderInt(v) => e.set_threshold(v.as_f32()),
                        _ => todo!(),
                    },
                    BoxedEntity::Gain(e) => match message {
                        EntityMessage::HSliderInt(value) => {
                            e.set_ceiling(Normal::new_from_f32(value.as_f32()));
                        }
                        _ => todo!(),
                    },
                    BoxedEntity::WelshSynth(e) => match message {
                        EntityMessage::Knob(value) => {
                            // TODO: it's annoying to have to plumb this through. I want
                            // everything #controllable to automatically generate the
                            // scaffolding for UI.
                            e.set_control_pan(F32ControlValue(value.as_f32()));
                        }
                        _ => todo!(),
                    },
                    #[allow(unused_variables)]
                    BoxedEntity::PatternManager(e) => match message {
                        EntityMessage::PatternMessage(uid, message) => {
                            todo!()
                        }
                        _ => todo!(),
                    },
                    _ => todo!(),
                }
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
            BoxedEntity::Arpeggiator(_) => {
                let title = type_name::<Arpeggiator>();
                let contents = "Hello!";
                GuiStuff::titled_container(
                    title,
                    GuiStuff::<EntityMessage>::container_text(contents).into(),
                )
            }
            BoxedEntity::AudioSource(e) => self.audio_source_view(e),
            BoxedEntity::BeatSequencer(e) => self.beat_sequencer_view(e),
            BoxedEntity::BiQuadFilter(e) => self.biquad_filter_view(e),
            BoxedEntity::Bitcrusher(e) => self.bitcrusher_view(e),
            BoxedEntity::Chorus(e) => GuiStuff::titled_container(
                type_name::<Chorus>(),
                GuiStuff::<EntityMessage>::container_text(
                    format!("Coming soon: {}", e.uid()).as_str(),
                )
                .into(),
            ),
            BoxedEntity::Compressor(e) => self.compressor_view(e),
            BoxedEntity::ControlTrip(e) => GuiStuff::titled_container(
                type_name::<ControlTrip>(),
                GuiStuff::<EntityMessage>::container_text(
                    format!("Coming soon: {}", e.uid()).as_str(),
                )
                .into(),
            ),
            BoxedEntity::Delay(e) => {
                let title = type_name::<Delay>();
                let contents = format!("delay in seconds: {}", e.seconds());
                GuiStuff::titled_container(
                    title,
                    GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
                )
            }
            BoxedEntity::DrumkitSampler(e) => {
                let title = type_name::<DrumkitSampler>();
                let contents = format!("kit name: {}", e.kit_name());
                GuiStuff::titled_container(
                    title,
                    GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
                )
            }
            BoxedEntity::FmSynthesizer(e) => self.fm_synthesizer_view(e),
            BoxedEntity::Gain(e) => self.gain_view(e),
            BoxedEntity::LfoController(e) => self.lfo_view(e),
            BoxedEntity::Limiter(e) => {
                let title = type_name::<Limiter>();
                let contents = format!("min: {} max: {}", e.min(), e.max());
                GuiStuff::titled_container(
                    title,
                    GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
                )
            }
            BoxedEntity::MidiTickSequencer(e) => GuiStuff::titled_container(
                type_name::<MidiTickSequencer>(),
                GuiStuff::<EntityMessage>::container_text(
                    format!("Coming soon: {}", e.uid()).as_str(),
                )
                .into(),
            ),
            BoxedEntity::Mixer(e) => self.mixer_view(e),
            BoxedEntity::PatternManager(e) => self.pattern_manager_view(e),
            BoxedEntity::Reverb(e) => GuiStuff::titled_container(
                type_name::<Reverb>(),
                GuiStuff::<EntityMessage>::container_text(
                    format!("Coming soon: {}", e.uid()).as_str(),
                )
                .into(),
            ),
            BoxedEntity::Sampler(e) => {
                let title = type_name::<Sampler>();
                let contents = format!("name: {}", e.filename());
                GuiStuff::titled_container(
                    title,
                    GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
                )
            }
            BoxedEntity::SimpleSynthesizer(e) => {
                let title = type_name::<SimpleSynthesizer>();
                let contents = format!("notes playing: {}", e.notes_playing());
                GuiStuff::titled_container(
                    title,
                    GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
                )
            }
            BoxedEntity::TestController(e) => self.test_controller_view(e),
            BoxedEntity::TestEffect(e) => self.test_effect_view(e),
            BoxedEntity::TestInstrument(e) => self.test_instrument_view(e),
            BoxedEntity::TestSynth(e) => self.test_synth_view(e),
            BoxedEntity::Timer(e) => self.timer_view(e),
            BoxedEntity::WelshSynth(e) => self.welsh_synth_view(e),
        }
    }

    fn welsh_synth_view(&self, e: &WelshSynth) -> Element<EntityMessage> {
        self.collapsing_box("Welsh", e.uid(), || {
            let options = vec!["Acid Bass".to_string(), "Piano".to_string()];
            let pan_knob: Element<EntityMessage> = Knob::new(
                // TODO: toil. make it easier to go from bipolar normal to normal
                NormalParam {
                    value: IcedNormal::from_clipped((e.pan() + 1.0) / 2.0),
                    default: IcedNormal::from_clipped(0.5),
                },
                EntityMessage::Knob,
            )
            .into();
            container(column![
                GuiStuff::<EntityMessage>::container_text(
                    format!("Welsh {} {} coming soon", e.uid(), e.preset_name()).as_str()
                ),
                pick_list(options, None, EntityMessage::PickListSelected,).font(gui::SMALL_FONT),
                pan_knob,
            ])
            .into()
        })
    }
    fn mixer_view(&self, e: &Mixer) -> Element<EntityMessage> {
        self.collapsing_box("Mixer", e.uid(), || {
            GuiStuff::<EntityMessage>::container_text(
                format!("Mixer {} coming soon", e.uid()).as_str(),
            )
            .into()
        })
    }

    fn midi_view(&self) -> Element<MidiHandlerMessage> {
        if let Some(midi_handler) = &self.midi_handler {
            if let Ok(midi_handler) = midi_handler.lock() {
                let activity_text = container(GuiStuff::<EntityMessage>::container_text(
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
                    GuiStuff::<EntityMessage>::container_text("Input")
                        .width(iced::Length::FillPortion(1)),
                    pick_list(
                        input_options,
                        input_selected.clone(),
                        MidiHandlerMessage::InputSelected,
                    )
                    .font(gui::SMALL_FONT)
                    .width(iced::Length::FillPortion(3))
                ];
                let (output_selected, output_options) =
                    midi_handler.midi_output().as_ref().unwrap().labels();
                let output_menu = row![
                    GuiStuff::<EntityMessage>::container_text("Output")
                        .width(iced::Length::FillPortion(1)),
                    pick_list(
                        output_options,
                        output_selected.clone(),
                        MidiHandlerMessage::OutputSelected,
                    )
                    .font(gui::SMALL_FONT)
                    .width(iced::Length::FillPortion(3))
                ];
                let port_menus =
                    container(column![input_menu, output_menu]).width(iced::Length::FillPortion(7));
                GuiStuff::titled_container(
                    "MIDI",
                    container(row![activity_text, port_menus]).into(),
                )
            } else {
                panic!()
            }
        } else {
            GuiStuff::titled_container(
                "MIDI",
                GuiStuff::<EntityMessage>::container_text("Initializing...").into(),
            )
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
                .font(gui::SMALL_FONT)
                .size(gui::SMALL_FONT_SIZE)
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
                container(text(app_version())).align_x(alignment::Horizontal::Right)
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
                    .font(gui::NUMBERS_FONT)
                    .size(gui::NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<ControlBarMessage>::number_box_style(&Theme::Dark),
            ))
        };

        let time_signature = self.clock_mirror.settings().time_signature();
        let time_signature_view = {
            container(column![
                text(format!("{}", time_signature.top))
                    .font(gui::SMALL_FONT)
                    .size(gui::SMALL_FONT_SIZE),
                text(format!("{}", time_signature.bottom))
                    .font(gui::SMALL_FONT)
                    .size(gui::SMALL_FONT_SIZE)
            ])
        };

        let beat_counter = {
            let denom = time_signature.top as f32;

            let measures = (self.clock_mirror.beats() / denom) as usize;
            let beats = (self.clock_mirror.beats() % denom) as usize;
            let fractional = (self.clock_mirror.beats().fract() * 10000.0) as usize;
            container(
                text(format!("{measures:04}m{beats:02}b{fractional:03}"))
                    .font(gui::NUMBERS_FONT)
                    .size(gui::NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<AppMessage>::number_box_style(&Theme::Dark),
            ))
        };
        row![time_counter, time_signature_view, beat_counter].into()
    }

    fn beat_sequencer_view(&self, e: &BeatSequencer) -> Element<EntityMessage> {
        self.collapsing_box("Sequencer", e.uid(), || {
            let contents = format!("{}", e.next_instant());
            GuiStuff::<EntityMessage>::container_text(contents.as_str()).into()
        })
    }

    fn test_controller_view(&self, e: &TestController) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<TestController>(),
            GuiStuff::<EntityMessage>::container_text(format!("Tempo: {}", e.tempo).as_str())
                .into(),
        )
    }

    fn test_effect_view(&self, e: &TestEffect) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<TestEffect>(),
            GuiStuff::<EntityMessage>::container_text(format!("Value: {}", e.my_value()).as_str())
                .into(),
        )
    }

    fn test_instrument_view(&self, e: &TestInstrument) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<TestInstrument>(),
            GuiStuff::<EntityMessage>::container_text(
                format!("Fake value: {}", e.fake_value()).as_str(),
            )
            .into(),
        )
    }

    fn test_synth_view(&self, _: &TestSynth) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<TestSynth>(),
            GuiStuff::<EntityMessage>::container_text("Nothing").into(),
        )
    }

    fn timer_view(&self, e: &Timer) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<Timer>(),
            GuiStuff::<EntityMessage>::container_text(
                format!("Runtime: {}", e.time_to_run_seconds()).as_str(),
            )
            .into(),
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
                let cell = text(format!("{:02} ", note.key).to_string())
                    .font(gui::LARGE_FONT)
                    .size(gui::LARGE_FONT_SIZE);
                note_row.push(cell.into());
            }
            let row_note_row = row(note_row).into();
            note_rows.push(row_note_row);
        }
        column(vec![
            button(GuiStuff::<EntityMessage>::container_text(
                format!("{:?}", e.note_value).as_str(),
            ))
            .on_press(PatternMessage::ButtonPressed)
            .into(),
            column(note_rows).into(),
        ])
        .into()
    }
    fn audio_source_view(&self, e: &AudioSource) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<AudioSource>(),
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into(),
        )
    }

    fn entity_view_state(&self, uid: usize) -> EntityViewState {
        if let Some(state) = self.entity_view_states.get(&uid) {
            state.clone()
        } else {
            EntityViewState::default()
        }
    }

    fn set_entity_view_state(&mut self, uid: usize, new_state: EntityViewState) {
        self.entity_view_states.insert(uid, new_state);
    }

    fn collapsing_box<F>(&self, title: &str, uid: usize, contents_fn: F) -> Element<EntityMessage>
    where
        F: FnOnce() -> Element<'static, EntityMessage>,
    {
        if self.entity_view_state(uid) == EntityViewState::Expanded {
            let contents = contents_fn();
            GuiStuff::expanded_container(title, EntityMessage::CollapsePressed, contents)
        } else {
            GuiStuff::<EntityMessage>::collapsed_container(title, EntityMessage::ExpandPressed)
        }
    }

    fn gain_view(&self, e: &Gain) -> Element<EntityMessage> {
        self.collapsing_box("Gain", e.uid(), || {
            let slider = HSlider::new(
                NormalParam {
                    value: IcedNormal::from_clipped(e.ceiling().value() as f32),
                    default: IcedNormal::from_clipped(1.0),
                },
                EntityMessage::HSliderInt,
            );
            container(row![slider]).padding(20).into()
        })
    }

    fn lfo_view(&self, e: &LfoController) -> Element<EntityMessage> {
        self.collapsing_box("LFO", e.uid(), || {
            let slider = HSlider::new(
                NormalParam {
                    value: IcedNormal::from_clipped(0.42_f32),
                    default: IcedNormal::from_clipped(1.0),
                },
                EntityMessage::HSliderInt,
            );
            container(row![slider]).padding(20).into()
        })
    }

    fn fm_synthesizer_view(&self, e: &FmSynthesizer) -> Element<EntityMessage> {
        self.collapsing_box("FM", e.uid(), || {
            let slider = HSlider::new(
                NormalParam {
                    value: IcedNormal::from_clipped(42.0), // TODO
                    default: IcedNormal::from_clipped(1.0),
                },
                EntityMessage::HSliderInt,
            );
            container(row![slider]).padding(20).into()
        })
    }

    fn biquad_filter_view(&self, e: &BiQuadFilter) -> Element<EntityMessage> {
        let title = type_name::<BiQuadFilter>();
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(e.cutoff_pct()),
                default: IcedNormal::from_clipped(1.0),
            },
            EntityMessage::HSliderInt,
        );
        let contents = row![
            container(slider).width(iced::Length::FillPortion(1)),
            container(GuiStuff::<EntityMessage>::container_text(
                format!("cutoff: {}Hz", e.cutoff_hz()).as_str()
            ))
            .width(iced::Length::FillPortion(1))
        ];
        GuiStuff::titled_container(title, contents.into())
    }

    fn bitcrusher_view(&self, e: &Bitcrusher) -> Element<EntityMessage> {
        let title = format!("{}: {}", type_name::<Bitcrusher>(), e.bits_to_crush());
        let contents = container(row![HSlider::new(
            e.bits_to_crush_int_range()
                .normal_param(e.bits_to_crush() as i32, 8),
            EntityMessage::HSliderInt
        )])
        .padding(20);
        GuiStuff::titled_container(&title, contents.into())
    }

    fn compressor_view(&self, e: &Compressor) -> Element<EntityMessage> {
        self.collapsing_box("Compressor", e.uid(), || {
            let slider = HSlider::new(
                NormalParam {
                    value: IcedNormal::from_clipped(e.threshold()),
                    default: IcedNormal::from_clipped(1.0),
                },
                EntityMessage::HSliderInt,
            );
            container(row![slider]).padding(20).into()
        })
    }

    fn switch_main_view(&mut self) {
        self.current_view = match self.current_view {
            MainViews::Unstructured => MainViews::Preferences,
            MainViews::Session => MainViews::Arrangement,
            MainViews::Arrangement => MainViews::Session,
            MainViews::Preferences => MainViews::Unstructured,
        }
    }

    fn handle_close_requested_event(&mut self) -> Command<AppMessage> {
        // See https://github.com/iced-rs/iced/pull/804 and
        // https://github.com/iced-rs/iced/blob/master/examples/events/src/main.rs#L55
        //
        // This is needed to stop an ALSA buffer underrun on close
        self.post_to_midi_handler(MidiHandlerInput::QuitRequested);
        self.post_to_orchestrator(GrooveInput::QuitRequested);

        // Let the PrefsSaved message handler know that it's time to go.
        self.should_exit = true;

        Command::perform(
            Preferences::save_prefs(Preferences {
                selected_midi_input: self.preferences.selected_midi_input.clone(),
                selected_midi_output: self.preferences.selected_midi_output.clone(),
                should_reload_last_project: self.preferences.should_reload_last_project,
                last_project_filename: self.preferences.last_project_filename.clone(),
            }),
            AppMessage::PrefsSaved,
        )
    }

    fn handle_keyboard_event(&mut self, event: iced::keyboard::Event) {
        if let iced::keyboard::Event::KeyPressed {
            key_code,
            modifiers: _,
        } = event
        {
            if key_code == iced::keyboard::KeyCode::Tab {
                self.switch_main_view();
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
