// Copyright (c) 2023 Mike Tsao. All rights reserved.

use self::views::EntityParams;
use super::{
    GuiStuff, IconType, Icons, LARGE_FONT, LARGE_FONT_SIZE, NUMBERS_FONT, NUMBERS_FONT_SIZE,
    SMALL_FONT, SMALL_FONT_SIZE,
};
use groove::{app_version, Entity};
use groove_core::{
    time::{Clock, TimeSignature},
    traits::HasUid,
    Normal, ParameterType, StereoSample,
};
use groove_entities::{
    controllers::{
        Arpeggiator, ControlTrip, LfoController, MidiTickSequencer, Note, Pattern, PatternManager,
        PatternMessage, Sequencer, SignalPassthroughController, Timer,
    },
    effects::{BiQuadFilter, Bitcrusher, Chorus, Compressor, Delay, Gain, Limiter, Mixer, Reverb},
    instruments::{Drumkit, FmSynthesizer, Sampler, WelshSynth},
    EntityMessage,
};
use groove_toys::{ToyAudioSource, ToyController, ToyEffect, ToyInstrument, ToySynth};
use iced::{
    alignment, theme,
    widget::{button, column, container, pick_list, row, text, text_input, Column, Container, Row},
    Alignment, Element, Length, Renderer, Theme,
};
use iced_audio::{FloatRange, HSlider, IntRange, Knob, Normal as IcedNormal, NormalParam};
use iced_native::{mouse, widget::Tree, Event, Widget};
use rustc_hash::FxHashMap;
use std::any::type_name;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum EntityViewState {
    #[default]
    Collapsed,
    Expanded,
}

#[derive(Debug)]
pub(crate) struct EntityView {
    entity_view_states: FxHashMap<usize, EntityViewState>,
    entity_enabled_states: FxHashMap<usize, bool>,
    entity_audio_outputs: FxHashMap<usize, StereoSample>,

    pub(crate) fm_synthesizer_ratio_range: IntRange,
    pub(crate) fm_synthesizer_beta_range: FloatRange,
}
impl Default for EntityView {
    fn default() -> Self {
        Self {
            entity_view_states: Default::default(),
            entity_enabled_states: Default::default(),
            entity_audio_outputs: Default::default(),
            fm_synthesizer_ratio_range: IntRange::new(1, 32),
            fm_synthesizer_beta_range: FloatRange::new(0.5, 32.0),
        }
    }
}
impl EntityView {
    const LABEL_FONT_SIZE: u16 = 14;

    const ITEM_OUTER_PADDING: u16 = 16;
    const ITEM_PADDING: u16 = 8;
    const ITEM_WIDTH: Length = Length::Fixed(48.0);

    pub(crate) fn set_entity_view_state(&mut self, uid: usize, new_state: EntityViewState) {
        self.entity_view_states.insert(uid, new_state);
    }

    pub(crate) fn set_entity_enabled_state(&mut self, uid: usize, enabled: bool) {
        self.entity_enabled_states.insert(uid, enabled);
    }

    pub(crate) fn reset(&mut self) {
        self.entity_view_states.clear();
    }

    pub(crate) fn view(&self, entity: &Entity) -> Element<EntityMessage> {
        match entity {
            Entity::Arpeggiator(e) => self.arpeggiator_view(e),
            Entity::BiQuadFilter(e) => self.biquad_filter_view(e),
            Entity::Bitcrusher(e) => self.bitcrusher_view(e),
            Entity::Chorus(e) => self.chorus_view(e),
            Entity::Compressor(e) => self.compressor_view(e),
            Entity::ControlTrip(e) => self.control_trip_view(e),
            Entity::Delay(e) => self.delay_view(e),
            Entity::Drumkit(e) => self.drumkit_view(e),
            Entity::FmSynthesizer(e) => self.fm_synthesizer_view(e),
            Entity::Gain(e) => self.gain_view(e),
            Entity::LfoController(e) => self.lfo_view(e),
            Entity::Limiter(e) => self.limiter_view(e),
            Entity::MidiTickSequencer(e) => self.midi_tick_sequencer_view(e),
            Entity::Mixer(e) => self.mixer_view(e),
            Entity::PatternManager(e) => self.pattern_manager_view(e),
            Entity::Reverb(e) => self.reverb_view(e),
            Entity::Sampler(e) => self.sampler_view(e),
            Entity::Sequencer(e) => self.sequencer_view(e),
            Entity::SignalPassthroughController(e) => self.signal_controller_view(e),
            Entity::Timer(e) => self.timer_view(e),
            Entity::ToyAudioSource(e) => self.audio_source_view(e),
            Entity::ToyController(e) => self.toy_controller_view(e),
            Entity::ToyEffect(e) => self.toy_effect_view(e),
            Entity::ToyInstrument(e) => self.test_instrument_view(e),
            Entity::ToySynth(e) => self.test_synth_view(e),
            Entity::WelshSynth(e) => self.welsh_synth_view(e),
        }
    }

    fn arpeggiator_view(&self, e: &Arpeggiator) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text("I'm an arpeggiator!").into()
        })
    }

    fn audio_source_view(&self, e: &ToyAudioSource) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into()
        })
    }

    fn biquad_filter_view(&self, e: &BiQuadFilter) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            let slider = HSlider::new(
                NormalParam {
                    value: IcedNormal::from_clipped(e.cutoff_pct()),
                    default: IcedNormal::from_clipped(1.0),
                },
                EntityMessage::HSliderInt,
            );
            row![
                container(slider).width(iced::Length::FillPortion(1)),
                container(GuiStuff::<EntityMessage>::container_text(
                    format!("cutoff: {}Hz", e.cutoff_hz()).as_str()
                ))
                .width(iced::Length::FillPortion(1))
            ]
            .into()
        })
    }

    fn bitcrusher_view(&self, e: &Bitcrusher) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            container(row![HSlider::new(
                IntRange::new(0, 15).normal_param(e.bits_to_crush().into(), 8),
                EntityMessage::HSliderInt
            )])
            .padding(Self::ITEM_OUTER_PADDING)
            .into()
        })
    }

    fn chorus_view(&self, e: &Chorus) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into()
        })
    }

    fn collapsing_box<F>(&self, entity: &impl HasUid, contents_fn: F) -> Element<EntityMessage>
    where
        F: FnOnce() -> Element<'static, EntityMessage>,
    {
        let uid = entity.uid();
        let title = entity.name();
        let enabled = self.entity_enabled_state(uid);
        let audio = *self
            .entity_audio_outputs
            .get(&uid)
            .unwrap_or(&StereoSample::default());

        if self.entity_view_state(uid) == EntityViewState::Expanded {
            let contents = contents_fn();
            GuiStuff::expanded_container(
                title,
                EntityMessage::CollapsePressed,
                EntityMessage::EnablePressed,
                enabled,
                audio,
                contents,
            )
        } else {
            GuiStuff::<EntityMessage>::collapsed_container(
                title,
                EntityMessage::ExpandPressed,
                EntityMessage::EnablePressed,
                enabled,
                audio,
            )
        }
    }

    fn compressor_view(&self, e: &Compressor) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            let slider = HSlider::new(
                NormalParam {
                    value: IcedNormal::from_clipped(e.threshold()),
                    default: IcedNormal::from_clipped(1.0),
                },
                EntityMessage::HSliderInt,
            );
            container(row![slider])
                .padding(Self::ITEM_OUTER_PADDING)
                .into()
        })
    }

    fn control_trip_view(&self, e: &ControlTrip) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into()
        })
    }

    fn delay_view(&self, e: &Delay) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text("I'm a delay effect!").into()
        })
    }

    fn drumkit_view(&self, e: &Drumkit) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text("I'm a drumkit!").into()
        })
    }

    fn entity_view_state(&self, uid: usize) -> EntityViewState {
        if let Some(state) = self.entity_view_states.get(&uid) {
            state.clone()
        } else {
            EntityViewState::default()
        }
    }

    fn entity_enabled_state(&self, uid: usize) -> bool {
        if let Some(enabled) = self.entity_enabled_states.get(&uid) {
            *enabled
        } else {
            true
        }
    }

    fn fm_synthesizer_view(&self, e: &FmSynthesizer) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            let depth = e.depth().value_as_f32();
            let label_depth = text("Depth").size(Self::LABEL_FONT_SIZE);
            let text_depth =
                text(format!("{:0.1}%", depth * 100.0).as_str()).size(Self::LABEL_FONT_SIZE);
            let ratio = e.ratio();
            let label_ratio = text("Ratio").size(Self::LABEL_FONT_SIZE);
            let text_ratio = text(format!("{:0.2}", ratio).as_str()).size(Self::LABEL_FONT_SIZE);
            let beta = e.beta();
            let label_beta = text("Beta").size(Self::LABEL_FONT_SIZE);
            let text_beta = text(format!("{:0.2}", beta).as_str()).size(Self::LABEL_FONT_SIZE);
            let depth_slider = Column::new()
                .push(label_depth)
                .push(Knob::new(
                    NormalParam {
                        value: IcedNormal::from_clipped(depth),
                        default: IcedNormal::from_clipped(0.5),
                    },
                    EntityMessage::Knob,
                ))
                .push(text_depth)
                .align_items(Alignment::Center)
                .padding(Self::ITEM_PADDING)
                .width(Self::ITEM_WIDTH);
            let ratio_slider = Column::new()
                .push(label_ratio)
                .push(Knob::new(
                    self.fm_synthesizer_ratio_range
                        .normal_param(ratio as i32, 2),
                    EntityMessage::Knob2,
                ))
                .push(text_ratio)
                .align_items(Alignment::Center)
                .padding(Self::ITEM_PADDING)
                .width(Self::ITEM_WIDTH);
            let beta_slider = Column::new()
                .push(label_beta)
                .push(Knob::new(
                    self.fm_synthesizer_beta_range
                        .normal_param(beta as f32, 2.0),
                    EntityMessage::Knob3,
                ))
                .push(text_beta)
                .align_items(Alignment::Center)
                .padding(Self::ITEM_PADDING)
                .width(Self::ITEM_WIDTH);
            container(row![depth_slider, ratio_slider, beta_slider])
                .padding(Self::ITEM_OUTER_PADDING)
                .into()
        })
    }

    fn gain_view(&self, e: &Gain) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            let slider = HSlider::new(
                NormalParam {
                    value: IcedNormal::from_clipped(e.ceiling().value() as f32),
                    default: IcedNormal::from_clipped(1.0),
                },
                EntityMessage::HSliderInt,
            );
            container(row![slider])
                .padding(Self::ITEM_OUTER_PADDING)
                .into()
        })
    }

    fn lfo_view(&self, e: &LfoController) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            let slider = HSlider::new(
                NormalParam {
                    value: IcedNormal::from_clipped(0.42_f32),
                    default: IcedNormal::from_clipped(1.0),
                },
                EntityMessage::HSliderInt,
            );
            container(row![slider])
                .padding(Self::ITEM_OUTER_PADDING)
                .into()
        })
    }

    fn limiter_view(&self, e: &Limiter) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            let contents = format!("min: {} max: {}", e.min(), e.max());
            GuiStuff::<EntityMessage>::container_text(contents.as_str()).into()
        })
    }

    fn midi_tick_sequencer_view(&self, e: &MidiTickSequencer) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into()
        })
    }

    fn mixer_view(&self, e: &Mixer) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(
                format!("Mixer {} coming soon", e.uid()).as_str(),
            )
            .into()
        })
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
                    .font(LARGE_FONT)
                    .size(LARGE_FONT_SIZE);
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

    fn reverb_view(&self, e: &Reverb) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into()
        })
    }

    fn sampler_view(&self, e: &Sampler) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text("I'm a sampler!").into()
        })
    }

    fn sequencer_view(&self, e: &Sequencer) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            let contents = format!("{}", e.next_instant());
            GuiStuff::<EntityMessage>::container_text(contents.as_str()).into()
        })
    }

    fn signal_controller_view(&self, e: &SignalPassthroughController) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text("nothing").into()
        })
    }

    fn test_instrument_view(&self, e: &ToyInstrument) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(
                format!("Fake value: {}", e.fake_value()).as_str(),
            )
            .into()
        })
    }

    fn test_synth_view(&self, e: &ToySynth) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text("Nothing").into()
        })
    }

    fn timer_view(&self, e: &Timer) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(
                format!("Runtime: {}", e.time_to_run_seconds()).as_str(),
            )
            .into()
        })
    }

    fn toy_controller_view(&self, e: &ToyController<EntityMessage>) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(format!("Tempo: {}", e.tempo).as_str()).into()
        })
    }

    fn toy_effect_view(&self, e: &ToyEffect) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
            GuiStuff::<EntityMessage>::container_text(format!("Value: {}", e.my_value()).as_str())
                .into()
        })
    }

    fn welsh_synth_view(&self, e: &WelshSynth) -> Element<EntityMessage> {
        self.collapsing_box(e, || {
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
                pick_list(options, None, EntityMessage::PickListSelected,).font(SMALL_FONT),
                pan_knob,
            ])
            .into()
        })
    }

    pub(crate) fn update_audio_outputs(&mut self, uid: &usize, sample: &StereoSample) {
        self.entity_audio_outputs.insert(*uid, *sample);
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ControlBarInput {
    SetClock(usize),
    SetBpm(f64),
    SetTimeSignature(TimeSignature),
}

#[derive(Debug, Clone)]
pub(crate) enum ControlBarEvent {
    Play,
    Stop,
    SkipToStart,
    Bpm(String),
    OpenProject,
    ExportWav,
    #[allow(dead_code)]
    ExportMp3,
}

#[derive(Debug)]
pub(crate) struct ControlBarView {
    clock: Clock,
    time_signature: TimeSignature,
    audio_buffer_fullness: Normal,
}
impl ControlBarView {
    pub fn new_with(clock: Clock, time_signature: TimeSignature) -> Self {
        Self {
            clock,
            time_signature,
            audio_buffer_fullness: Default::default(),
        }
    }

    pub fn view(&self, is_playing: bool) -> Element<ControlBarEvent> {
        let full_row = Row::new()
            .push(self.bpm_view())
            .push(self.media_buttons(is_playing))
            .push(self.clock_view())
            .push(self.util_buttons())
            .align_items(Alignment::Center);

        container(full_row)
            .width(Length::Fill)
            .padding(4)
            .style(theme::Container::Box)
            .into()
    }

    pub fn update(&mut self, message: ControlBarInput) {
        match message {
            ControlBarInput::SetClock(frames) => self.set_clock(frames),
            ControlBarInput::SetBpm(bpm) => self.set_bpm(bpm),
            ControlBarInput::SetTimeSignature(time_signature) => {
                self.set_time_signature(time_signature)
            }
        }
    }

    fn set_clock(&mut self, frames: usize) {
        self.clock.seek(frames);
    }

    fn set_bpm(&mut self, bpm: ParameterType) {
        self.clock.set_bpm(bpm);
    }

    fn set_time_signature(&mut self, time_signature: TimeSignature) {
        self.time_signature = time_signature;
    }

    fn media_buttons(&self, is_playing: bool) -> Container<ControlBarEvent> {
        let start_button =
            Icons::button_icon(IconType::Start).on_press(ControlBarEvent::SkipToStart);
        let play_button = (if is_playing {
            Icons::button_icon(IconType::Pause)
        } else {
            Icons::button_icon(IconType::Play)
        })
        .on_press(ControlBarEvent::Play);
        let stop_button = Icons::button_icon(IconType::Stop).on_press(ControlBarEvent::Stop);
        container(
            Row::new()
                .push(start_button)
                .push(play_button)
                .push(stop_button),
        )
    }

    fn clock_view(&self) -> Element<ControlBarEvent> {
        let time_counter = {
            let minutes: u8 = (self.clock.seconds() / 60.0).floor() as u8;
            let seconds = self.clock.seconds() as usize % 60;
            let thousandths = (self.clock.seconds().fract() * 1000.0) as u16;
            container(
                text(format!("{minutes:03}:{seconds:02}:{thousandths:03}"))
                    .font(NUMBERS_FONT)
                    .size(NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<ControlBarEvent>::number_box_style(&Theme::Dark),
            ))
            .align_x(alignment::Horizontal::Center)
        };

        let time_signature_view = {
            container(
                Column::new()
                    .push(
                        text(format!("{}", self.time_signature.top))
                            .font(SMALL_FONT)
                            .size(SMALL_FONT_SIZE),
                    )
                    .push(
                        text(format!("{}", self.time_signature.bottom))
                            .font(SMALL_FONT)
                            .size(SMALL_FONT_SIZE),
                    )
                    .width(Length::Fixed(16.0))
                    .align_items(Alignment::Center),
            )
        };

        let beat_counter = {
            let denom = self.time_signature.top as f64;

            let measures = (self.clock.beats() / denom) as usize;
            let beats = (self.clock.beats() % denom) as usize;
            let fractional = (self.clock.beats().fract() * 10000.0) as usize;
            container(
                text(format!("{measures:04}m{beats:02}b{fractional:04}"))
                    .font(NUMBERS_FONT)
                    .size(NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<ControlBarEvent>::number_box_style(&Theme::Dark),
            ))
            .align_x(alignment::Horizontal::Center)
        };
        Row::new()
            .push(time_counter)
            .push(time_signature_view)
            .push(beat_counter)
            .align_items(Alignment::Center)
            .padding(8)
            .into()
    }

    fn bpm_view(&self) -> Container<ControlBarEvent> {
        container(
            text_input(
                "BPM",
                self.clock.bpm().round().to_string().as_str(),
                ControlBarEvent::Bpm,
            )
            .font(SMALL_FONT)
            .size(SMALL_FONT_SIZE),
        )
        .width(Length::Fixed(60.0))
        .padding(8)
    }

    fn util_buttons(&self) -> Container<ControlBarEvent> {
        let audiobuf_container = container(Column::new().push(text("Audio")).push(text(
            format!("{:0.2}%", self.audio_buffer_fullness.value() * 100.0).as_str(),
        )))
        .width(Length::FillPortion(1));
        let open_button =
            Icons::button_icon(IconType::OpenProject).on_press(ControlBarEvent::OpenProject);
        let export_wav_button =
            Icons::button_icon(IconType::ExportWav).on_press(ControlBarEvent::ExportWav);
        let export_mp3_button = Icons::button_icon(IconType::ExportMp3); /* disabled for now .on_press(ControlBarMessage::ExportMp3) */
        let app_version = container(text(app_version())).align_x(alignment::Horizontal::Right);

        container(
            Row::new()
                .push(audiobuf_container)
                .push(open_button)
                .push(export_wav_button)
                .push(export_mp3_button)
                .push(app_version)
                .align_items(Alignment::Center),
        )
    }

    pub fn set_audio_buffer_fullness(&mut self, audio_buffer_fullness: Normal) {
        self.audio_buffer_fullness = audio_buffer_fullness;
    }
}

struct ControlTargetWidget<'a, Message> {
    inner: Element<'a, Message>,
    on_mousein: Option<Message>,
    on_mouseout: Option<Message>,
    on_mousedown: Option<Message>,
    on_mouseup: Option<Message>,
    on_mouseup_outside: Option<Message>,
}
impl<'a, Message> ControlTargetWidget<'a, Message>
where
    Message: 'a + Clone,
{
    pub fn new<T>(
        content: T,
        on_mousein: Option<Message>,
        on_mouseout: Option<Message>,
        on_mousedown: Option<Message>,
        on_mouseup: Option<Message>,
        on_mouseup_outside: Option<Message>,
    ) -> Self
    where
        T: Into<Element<'a, Message>>,
    {
        Self {
            inner: content.into(),
            on_mousein,
            on_mouseout,
            on_mousedown,
            on_mouseup,
            on_mouseup_outside,
        }
    }
}
impl<'a, Message> Widget<Message, Renderer> for ControlTargetWidget<'a, Message>
where
    Message: 'a + Clone,
    Renderer: 'a + iced_native::Renderer,
{
    fn on_event(
        &mut self,
        _tree: &mut iced_native::widget::Tree,
        event: iced::Event,
        layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, Message>,
    ) -> iced::event::Status {
        let in_bounds = if let Some(children) = layout.children().next() {
            children.bounds().contains(cursor_position)
        } else {
            layout.bounds().contains(cursor_position)
        };

        match event {
            Event::Mouse(event) => match event {
                mouse::Event::ButtonPressed(_) => {
                    if let Some(msg) = self.on_mousedown.as_ref() {
                        if in_bounds {
                            shell.publish(msg.clone());
                            return iced::event::Status::Captured;
                        }
                    }
                }
                mouse::Event::ButtonReleased(_) => {
                    if in_bounds {
                        if let Some(msg) = self.on_mouseup.as_ref() {
                            shell.publish(msg.clone());
                            return iced::event::Status::Captured;
                        }
                    } else {
                        if let Some(msg) = self.on_mouseup_outside.as_ref() {
                            shell.publish(msg.clone());
                            return iced::event::Status::Captured;
                        }
                    }
                }
                #[allow(unused_variables)]
                mouse::Event::CursorMoved { position } => {
                    if in_bounds {
                        if let Some(msg) = self.on_mousein.as_ref() {
                            shell.publish(msg.clone());
                            return iced::event::Status::Captured;
                        }
                    } else {
                        if let Some(msg) = self.on_mouseout.as_ref() {
                            shell.publish(msg.clone());
                            return iced::event::Status::Captured;
                        }
                    }
                }
                mouse::Event::WheelScrolled { delta } => {
                    eprintln!("scrolled {:?}", delta);
                    return iced::event::Status::Captured;
                }
                _ => {}
            },
            _ => {}
        }
        iced::event::Status::Ignored
    }

    fn width(&self) -> Length {
        self.inner.as_widget().width()
    }

    fn height(&self) -> Length {
        self.inner.as_widget().height()
    }

    fn layout(
        &self,
        renderer: &Renderer,
        limits: &iced_native::layout::Limits,
    ) -> iced_native::layout::Node {
        self.inner.as_widget().layout(renderer, limits)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.inner)]
    }

    fn draw(
        &self,
        state: &iced_native::widget::Tree,
        renderer: &mut Renderer,
        theme: &<Renderer as iced_native::Renderer>::Theme,
        style: &iced_native::renderer::Style,
        layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
        viewport: &iced::Rectangle,
    ) {
        self.inner.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor_position,
            viewport,
        )
    }
}

impl<'a, Message> From<ControlTargetWidget<'a, Message>> for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced_native::Renderer,
{
    fn from(widget: ControlTargetWidget<'a, Message>) -> Self {
        Element::new(widget)
    }
}

#[derive(Debug, Default)]
struct EntityStore {
    entities: FxHashMap<usize, Box<EntityParams>>,
}
impl EntityStore {
    fn get(&self, uid: &usize) -> Option<&Box<EntityParams>> {
        self.entities.get(uid)
    }
    fn get_mut(&mut self, uid: &usize) -> Option<&mut Box<EntityParams>> {
        self.entities.get_mut(uid)
    }
}

// #[derive(Clone, Debug)]
// pub enum AutomationMessage {
//     MouseIn(usize),
//     MouseOut(usize),
//     MouseDown(usize),
//     MouseUp(usize),
//     Connect(usize, usize, usize),
// }

/// A [Controller] represents a view of an IsController for the Automation view
/// pane.
#[derive(Debug)]
pub(crate) struct Controller {
    pub uid: usize,
    pub name: String,
}
impl Controller {
    pub fn new(uid: usize, name: &str) -> Self {
        Self {
            uid,
            name: name.to_string(),
        }
    }

    #[allow(dead_code)]
    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

// /// A [Controllable] represents a view of something implementing the
// /// [groove_core::traits::Controllable] trait for the Automation view pane.
// #[derive(Debug)]
// pub(crate) struct Controllable {
//     pub uid: usize,
//     pub name: String,
//     pub controllables: Vec<ControlPoint>,
// }
// impl Controllable {
//     pub fn new(uid: usize, name: &str, control_points: Vec<&str>) -> Self {
//         let mut r = Self {
//             uid: uid,
//             name: name.to_string(),
//             controllables: Vec::default(),
//         };
//         r.controllables = control_points.iter().fold(Vec::default(), |mut v, name| {
//             v.push(ControlPoint::new(name));
//             v
//         });
//         r
//     }

//     #[allow(dead_code)]
//     fn set_uid(&mut self, uid: usize) {
//         self.uid = uid;
//     }
// }

// /// A [ControlPoint] is one of the things that a [Controllable] allows to be
// /// automated.
// #[derive(Debug)]
// pub(crate) struct ControlPoint {
//     pub name: String,
// }
// impl ControlPoint {
//     pub fn new(name: &str) -> Self {
//         Self {
//             name: name.to_string(),
//         }
//     }
// }

pub(crate) mod views {
    use super::{ControlTargetWidget, EntityStore};
    use groove::Entity;
    use groove_core::{traits::Controllable, BipolarNormal, Normal};
    use groove_entities::{
        controllers::{
            ArpeggiatorParams, ArpeggiatorParamsMessage, LfoControllerParams,
            LfoControllerParamsMessage, PatternManagerParams, PatternManagerParamsMessage,
            SequencerParams, SequencerParamsMessage, WaveformParams,
        },
        effects::{
            BitcrusherParams, BitcrusherParamsMessage, GainParams, GainParamsMessage, MixerParams,
            MixerParamsMessage, ReverbParams, ReverbParamsMessage,
        },
        instruments::{WelshSynthParams, WelshSynthParamsMessage},
    };
    use iced::{
        widget::{column, container, text, Column, Row, Text},
        Element, Length,
    };
    use iced_aw::{
        style::{BadgeStyles, CardStyles},
        Badge, Card,
    };
    use rustc_hash::FxHashMap;
    use strum::EnumCount;
    use strum_macros::{EnumCount as EnumCountMacro, FromRepr};

    #[derive(Debug)]
    pub(crate) struct AudioLane {
        pub name: String,
        pub items: Vec<usize>,
    }

    trait Viewable<Message> {
        type Message;

        fn view(&self) -> Element<Self::Message>;
    }

    ///////////////////////////////
    #[derive(Clone, Debug)]
    pub enum DrumkitMessage {
        Cowbell(f32),
    }

    #[derive(Debug)]
    pub(crate) struct DrumkitView {
        cowbell: f32,
    }
    impl Viewable<DrumkitMessage> for DrumkitView {
        type Message = DrumkitMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("cowbell: {}", self.cowbell))).into()
        }
    }
    ///////////////////////////////

    impl Viewable<ArpeggiatorParamsMessage> for ArpeggiatorParams {
        type Message = ArpeggiatorParamsMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("bpm: {}", self.bpm()))).into()
        }
    }

    impl Viewable<BitcrusherParamsMessage> for BitcrusherParams {
        type Message = BitcrusherParamsMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("bits: {}", self.bits()))).into()
        }
    }

    impl Viewable<GainParamsMessage> for GainParams {
        type Message = GainParamsMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("ceiling: {}", self.ceiling().value()))).into()
        }
    }

    impl Viewable<LfoControllerParamsMessage> for LfoControllerParams {
        type Message = LfoControllerParamsMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!(
                "waveform: {:?} frequency: {}",
                self.waveform(), // TODO: proper string conversion
                self.frequency()
            )))
            .into()
        }
    }

    impl Viewable<MixerParamsMessage> for MixerParams {
        type Message = MixerParamsMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("I'm a mixer! {}", 261))).into()
        }
    }

    impl Viewable<PatternManagerParamsMessage> for PatternManagerParams {
        type Message = PatternManagerParamsMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("nothing {}", 42))).into()
        }
    }

    impl Viewable<ReverbParamsMessage> for ReverbParams {
        type Message = ReverbParamsMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!(
                "attenuation: {}",
                self.attenuation().value()
            )))
            .into()
        }
    }

    impl Viewable<SequencerParamsMessage> for SequencerParams {
        type Message = SequencerParamsMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("BPM: {}", self.bpm()))).into()
        }
    }

    impl Viewable<WelshSynthParamsMessage> for WelshSynthParams {
        type Message = WelshSynthParamsMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("pan: {}", self.pan().value()))).into()
        }
    }

    macro_rules! register_impl {
        ($trait_:ident for $ty:ty, true) => {
            impl<'a> MaybeImplements<'a, dyn $trait_> for $ty {
                fn as_trait_ref(&self) -> Option<&(dyn $trait_ + 'static)> {
                    Some(self)
                }
                fn as_trait_mut(&mut self) -> Option<&mut (dyn $trait_ + 'static)> {
                    Some(self)
                }
            }
        };
        ($trait_:ident for $ty:ty, false) => {
            impl<'a> MaybeImplements<'a, dyn $trait_> for $ty {
                fn as_trait_ref(&self) -> Option<&(dyn $trait_ + 'static)> {
                    None
                }
                fn as_trait_mut(&mut self) -> Option<&mut (dyn $trait_ + 'static)> {
                    None
                }
            }
        };
    }

    macro_rules! all_entities {
    ($($entity:ident; $params:tt; $message:ident; $is_controller:tt; $is_controllable:tt ,)*) => {
        #[derive(Clone, Debug)]
        pub(crate) enum OtherEntityMessage {
            $( $params($message) ),*
        }
        #[derive(Debug)]
        pub(crate) enum EntityParams {
            $( $entity(Box<$params>) ),*
        }
        impl EntityParams {
            pub(crate) fn is_controller(&self) -> bool {
                match self {
                    $( EntityParams::$entity(e) => $is_controller, )*
                }
            }
            pub(crate) fn is_controllable(&self) -> bool {
                match self {
                    $( EntityParams::$entity(e) => $is_controllable, )*
                }
            }
            pub(crate) fn as_controllable_ref(&self) -> Option<&(dyn Controllable + 'static)> {
                match self {
                    $( EntityParams::$entity(e) => e.as_trait_ref(), )*
                }
            }
            pub(crate) fn as_controllable_mut(&mut self) -> Option<&mut (dyn Controllable + 'static)> {
                match self {
                    $( EntityParams::$entity(e) => e.as_trait_mut(), )*
                }
            }
        }
        trait MaybeImplements<'a, Trait: ?Sized> {
            fn as_trait_ref(&'a self) -> Option<&'a Trait>;
            fn as_trait_mut(&mut self) -> Option<&mut Trait>;
        }
        $( register_impl!(Controllable for $params, $is_controllable); )*
    };
}

    all_entities! {
        // struct; params; message; is_controller; is_controllable,
        Arpeggiator; ArpeggiatorParams; ArpeggiatorParamsMessage; true; true,
        Bitcrusher; BitcrusherParams; BitcrusherParamsMessage; false; true,
        Gain; GainParams; GainParamsMessage; false; true,
        LfoController; LfoControllerParams; LfoControllerParamsMessage; true; false,
        Mixer; MixerParams; MixerParamsMessage; false; true,
        PatternManager; PatternManagerParams; PatternManagerParamsMessage; true; false,
        Reverb; ReverbParams; ReverbParamsMessage; false; true,
        Sequencer; SequencerParams; SequencerParamsMessage; false; true,
        WelshSynth; WelshSynthParams; WelshSynthParamsMessage; false; true,
    }

    #[derive(Debug)]
    struct AudioLaneView {
        viewable_items: FxHashMap<usize, Box<EntityParams>>,
    }
    impl AudioLaneView {
        fn new() -> Self {
            let mut r = Self {
                viewable_items: Default::default(),
                //                lanes: Default::default(),
            };
            // r.viewable_items.insert(
            //     1,
            //     Box::new(EntityParams::Drumkit(DrumkitView { cowbell: 0.5 })),
            // );
            // r.viewable_items.insert(
            //     2,
            //     Box::new(EntityParams::Reverb(ReverbView { amount: 0.1 })),
            // );
            // r.viewable_items.insert(
            //     3,
            //     Box::new(EntityParams::Drumkit(DrumkitView { cowbell: -0.25 })),
            // );
            // r.viewable_items.insert(
            //     4,
            //     Box::new(EntityParams::Reverb(ReverbView { amount: 0.0 })),
            // );
            // r.viewable_items.insert(
            //     5,
            //     Box::new(EntityParams::WelshSynth(WelshSynthView { pan: 0.14159 })),
            // );

            // r.lanes = vec![
            //     AudioLane {
            //         name: String::from("Rhythm"),
            //         items: vec![1, 2],
            //     },
            //     AudioLane {
            //         name: String::from("Rhythm B"),
            //         items: vec![3, 4],
            //     },
            //     AudioLane {
            //         name: String::from("Lead"),
            //         items: vec![5],
            //     },
            // ];
            r
        }

        // fn update(
        //     &mut self,
        //     uid: usize,
        //     message: OtherEntityMessage,
        // ) -> Option<OtherEntityMessage> {
        //     if let Some(entity) = self.viewable_items.get_mut(&uid) {
        //         match message {
        //             OtherEntityMessage::ArpeggiatorParams(message) => {
        //                 if let EntityParams::Arpeggiator(entity) = entity.as_mut() {
        //                     entity.update(message); // TODO: handle reply
        //                 }
        //             }
        //             OtherEntityMessage::BitcrusherParams(message) => {
        //                 if let EntityParams::Bitcrusher(entity) = entity.as_mut() {
        //                     entity.update(message); // TODO: handle reply
        //                 }
        //             }
        //             OtherEntityMessage::GainParams(message) => {
        //                 if let EntityParams::Gain(entity) = entity.as_mut() {
        //                     entity.update(message); // TODO: handle reply
        //                 }
        //             }
        //             OtherEntityMessage::LfoControllerParams(message) => {
        //                 if let EntityParams::LfoController(entity) = entity.as_mut() {
        //                     entity.update(message); // TODO: handle reply
        //                 }
        //             }
        //         }
        //     }
        //     None
        // }
    }

    #[derive(Clone, Debug)]
    pub(crate) enum MainViewThingyMessage {
        NextView,
        OtherEntityMessage(usize, OtherEntityMessage),
        MouseIn(usize),
        MouseOut(usize),
        MouseDown(usize),
        MouseUp(usize),

        /// Please ask the engine to connect controller_uid to controllable_uid's control #param_index.
        /// TODO: do we get this for free with the synchronization infra?
        Connect(usize, usize, usize),
    }

    #[derive(Clone, Copy, Debug, Default, FromRepr, EnumCountMacro)]
    pub(crate) enum MainViewThingyViews {
        AudioLanes,
        #[default]
        Automation,
    }

    #[derive(Debug)]
    pub(crate) struct MainViewThingy {
        current_view: MainViewThingyViews,
        entity_store: EntityStore,

        is_dragging: bool,
        source_id: usize,
        target_id: usize,

        controller_uids: Vec<usize>,
        controllable_uids: Vec<usize>,
        controllable_uids_to_control_names: FxHashMap<usize, Vec<String>>,
        connections: Vec<(usize, usize)>,

        lanes: Vec<AudioLane>,
    }

    macro_rules! build_entity_fns {
        ($($entity:ident: $params:tt,)*) => {
            fn entity_view<'a>(&self, uid: usize, entity: &'a EntityParams) -> Element<'a, MainViewThingyMessage> {
                match entity {
                $(
                    EntityParams::$entity(e) => {
                        e.view().map(move |message| {
                            MainViewThingyMessage::OtherEntityMessage(
                                uid,
                                OtherEntityMessage::$params(
                                    message,
                                ),
                            )
                        })
                    } ),*
                }
            }

            fn entity_update(
                &mut self,
                uid: usize,
                message: OtherEntityMessage,
            ) -> Option<MainViewThingyMessage> {
                if let Some(entity) = self.entity_store.get_mut(&uid) {
                    match message {
                    $(
                        OtherEntityMessage::$params(message) => {
                            if let EntityParams::$entity(entity) = entity.as_mut() {
                                entity.update(message); // TODO: handle reply
                            }
                        }
                    ),*
                    }
                }
                None
            }

        }
    }

    impl MainViewThingy {
        pub(crate) fn new() -> Self {
            Self {
                current_view: Default::default(),
                entity_store: Default::default(),

                is_dragging: false,
                source_id: 0,
                target_id: 0,

                controller_uids: Default::default(),
                controllable_uids: Default::default(),
                controllable_uids_to_control_names: Default::default(),
                connections: Default::default(),

                lanes: Default::default(),
            }
        }

        pub(crate) fn clear(&mut self) {
            self.controller_uids.clear();
            self.controllable_uids.clear();
            self.controllable_uids_to_control_names.clear();
            self.connections.clear(); // TODO: this shouldn't exist, or else should be like the Params things (synchronized)
        }

        pub(crate) fn view(&self) -> Element<MainViewThingyMessage> {
            match self.current_view {
                MainViewThingyViews::AudioLanes => self.audio_lane_view(&self.entity_store),
                MainViewThingyViews::Automation => self.automation_view(&self.entity_store),
            }
        }

        fn automation_view(&self, entity_store: &EntityStore) -> Element<MainViewThingyMessage> {
            let controller_columns = self.controller_uids.iter().enumerate().fold(
                Vec::default(),
                |mut v, (_index, controller_uid)| {
                    let column = Column::new();
                    let controller_id = *controller_uid;
                    let card_style = if self.is_dragging {
                        if controller_id == self.source_id {
                            CardStyles::Primary
                        } else {
                            CardStyles::Default
                        }
                    } else {
                        CardStyles::Default
                    };
                    let controller = entity_store.get(controller_uid).unwrap(); // TODO no unwraps!
                    let card = Card::new(
                        ControlTargetWidget::<MainViewThingyMessage>::new(
                            Text::new("TBD"),
                            if self.is_dragging && controller_id != self.source_id {
                                // entering the bounds of a potential target.
                                Some(MainViewThingyMessage::MouseIn(controller_id))
                            } else {
                                None
                            },
                            if self.is_dragging && controller_id == self.target_id {
                                // leaving the bounds of a potential target
                                Some(MainViewThingyMessage::MouseOut(controller_id))
                            } else {
                                None
                            },
                            if !self.is_dragging {
                                // starting a drag operation
                                Some(MainViewThingyMessage::MouseDown(controller_id))
                            } else {
                                None
                            },
                            if self.is_dragging {
                                if controller_id == self.source_id {
                                    // user pressed and released on source card
                                    Some(MainViewThingyMessage::MouseUp(0))
                                } else {
                                    // ending the drag on a target
                                    Some(MainViewThingyMessage::MouseUp(controller_id))
                                }
                            } else {
                                None
                            },
                            if self.is_dragging && controller_id == self.source_id {
                                // ending the drag somewhere that's not the source... but it could be a target!
                                // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                                Some(MainViewThingyMessage::MouseUp(0))
                            } else {
                                None
                            },
                        ),
                        column,
                    )
                    .style(card_style);
                    v.push(card);
                    v
                },
            );

            let controllable_columns = self.controllable_uids.iter().enumerate().fold(
                Vec::default(),
                |mut v, (_index, controllable_uid)| {
                    let mut column = Column::new();
                    let controllable_id = *controllable_uid;
                    if let Some(controllable) = entity_store.get(controllable_uid) {
                        if let Some(controllable) = controllable.as_controllable_ref() {
                            for param_id in 0..controllable.control_index_count() {
                                let param_app_id = controllable_id * 10000 + param_id;
                                let badge_style = if self.is_dragging {
                                    if param_app_id == self.source_id {
                                        BadgeStyles::Danger // This shouldn't happen (I think) because it's a source, not a target
                                    } else if param_app_id == self.target_id {
                                        BadgeStyles::Success // Hovering over target, so highlight it specially
                                    } else {
                                        BadgeStyles::Info // Indicate that it's a potential target
                                    }
                                } else {
                                    BadgeStyles::Default // Regular state
                                };
                                let child = ControlTargetWidget::<MainViewThingyMessage>::new(
                                    Badge::new(Text::new(
                                        controllable
                                            .control_name_for_index(param_id)
                                            .unwrap_or_default()
                                            .to_string(),
                                    ))
                                    .style(badge_style),
                                    if self.is_dragging && param_app_id != self.source_id {
                                        // entering the bounds of a potential target.
                                        Some(MainViewThingyMessage::MouseIn(param_app_id))
                                    } else {
                                        None
                                    },
                                    if self.is_dragging && param_app_id == self.target_id {
                                        // leaving the bounds of a potential target
                                        Some(MainViewThingyMessage::MouseOut(param_app_id))
                                    } else {
                                        None
                                    },
                                    if !self.is_dragging {
                                        // starting a drag operation
                                        Some(MainViewThingyMessage::MouseDown(param_app_id))
                                    } else {
                                        None
                                    },
                                    if self.is_dragging && param_app_id != self.source_id {
                                        // ending the drag on a target
                                        //                            Some(MainViewThingyMessage::MouseUp(id))
                                        Some(MainViewThingyMessage::Connect(
                                            self.source_id,
                                            controllable_id,
                                            param_id,
                                        ))
                                    } else {
                                        None
                                    },
                                    if self.is_dragging && param_app_id == self.source_id {
                                        // ending the drag somewhere that's not the source... but it could be a target!
                                        // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                                        Some(MainViewThingyMessage::MouseUp(0))
                                    } else {
                                        None
                                    },
                                    // Other cases to handle
                                    // - leaving the window entirely
                                    // - keyboard stuff
                                );
                                column = column.push(child);
                            }
                            let card_style = if self.is_dragging {
                                if controllable_id == self.source_id {
                                    CardStyles::Primary
                                } else if controllable_id == self.target_id {
                                    CardStyles::Danger
                                } else {
                                    CardStyles::Default
                                }
                            } else {
                                CardStyles::Default
                            };
                            let card = Card::new(
                                ControlTargetWidget::<MainViewThingyMessage>::new(
                                    Text::new(
                                        format!("I don't know my name yet! {}", controllable_uid)
                                            .to_string(),
                                    ),
                                    // Sources aren't targets.
                                    None,
                                    // Don't care.
                                    None,
                                    // starting a drag operation
                                    Some(MainViewThingyMessage::MouseDown(controllable_id)),
                                    if self.is_dragging && controllable_id != self.source_id {
                                        // ending the drag on a target
                                        Some(MainViewThingyMessage::MouseUp(controllable_id))
                                    } else {
                                        None
                                    },
                                    if self.is_dragging && controllable_id == self.source_id {
                                        // ending the drag somewhere that's not the source... but it could be a target!
                                        // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                                        Some(MainViewThingyMessage::MouseUp(0))
                                    } else {
                                        None
                                    },
                                ),
                                column,
                            )
                            .style(card_style);
                            v.push(card);
                            v
                        } else {
                            panic!()
                        }
                    } else {
                        panic!()
                    }
                },
            );

            let controller_row =
                controller_columns
                    .into_iter()
                    .fold(Row::new(), |mut row, item| {
                        row = row.push(item);
                        row
                    });
            let controllable_row =
                controllable_columns
                    .into_iter()
                    .fold(Row::new(), |mut row, item| {
                        row = row.push(item);
                        row
                    });
            container(column![controller_row, controllable_row]).into()
        }

        build_entity_fns! {
            Arpeggiator: ArpeggiatorParams,
            Bitcrusher: BitcrusherParams,
            Gain: GainParams,
            LfoController: LfoControllerParams,
            Mixer: MixerParams,
            PatternManager: PatternManagerParams,
            Reverb: ReverbParams,
            Sequencer: SequencerParams,
            WelshSynth: WelshSynthParams,
        }

        fn audio_lane_view<'a, 'b: 'a>(
            &'a self,
            entity_store: &'b EntityStore,
        ) -> Element<'a, MainViewThingyMessage> {
            let lane_views =
                self.lanes
                    .iter()
                    .enumerate()
                    .fold(Vec::default(), |mut v, (i, lane)| {
                        let lane_row = Card::new(
                            text(&format!("Lane #{}: {}", i, lane.name)),
                            lane.items.iter().enumerate().fold(
                                Row::new(),
                                |r, (item_index, uid)| {
                                    if let Some(item) = entity_store.get(uid) {
                                        let name: &'static str = "no idea 2345983495";
                                        let view = self.entity_view(*uid, item.as_ref());
                                        r.push(
                                            Card::new(
                                                text(&format!("#{}: {}", item_index, name)),
                                                container(view).height(Length::Fill),
                                            )
                                            .width(Length::FillPortion(1))
                                            .height(Length::FillPortion(1)),
                                        )
                                    } else {
                                        r
                                    }
                                },
                            ),
                        );
                        v.push(lane_row);
                        v
                    });
            let view_column = lane_views
                .into_iter()
                .fold(Column::new(), |c, lane_view| {
                    c.push(lane_view.height(Length::FillPortion(1)))
                })
                .width(Length::Fill)
                .height(Length::Fill);
            let mixer = Card::new(text("Mixer"), text("coming soon").height(Length::Fill))
                .width(Length::Fixed(96.0));
            let overall_view = Row::new().height(Length::Fill);
            container(overall_view.push(view_column).push(mixer)).into()
        }

        pub(crate) fn update(
            &mut self,
            message: MainViewThingyMessage,
        ) -> Option<MainViewThingyMessage> {
            match message {
                MainViewThingyMessage::NextView => {
                    self.current_view = MainViewThingyViews::from_repr(
                        (self.current_view as usize + 1) % MainViewThingyViews::COUNT,
                    )
                    .unwrap_or_default();
                    None
                }
                MainViewThingyMessage::OtherEntityMessage(uid, message) => {
                    self.entity_update(uid, message)
                } //
                //
                //
                // AutomationMessage::Connect(controller_uid, controllable_uid, control_index) => {
                //     Some(MainViewThingyEvent::Connect(
                //         controller_uid,
                //         controllable_uid,
                //         control_index,
                //     ))
                // }
                MainViewThingyMessage::MouseDown(id) => {
                    self.is_dragging = true;
                    self.source_id = id;
                    self.target_id = 0;
                    eprintln!("Start dragging on {}", id);
                    None
                }
                MainViewThingyMessage::MouseIn(id) => {
                    // if dragging, highlight potential target
                    self.target_id = id;
                    None
                }
                MainViewThingyMessage::MouseOut(_id) => {
                    // if dragging, un-highlight potential target
                    self.target_id = 0;
                    None
                }
                MainViewThingyMessage::MouseUp(id) => {
                    // This is probably going to have a bug later. Currently, the
                    // only way we get a MouseUp message is if we're in a drag
                    // operation, so we can afford to get both the MouseUp(id) and
                    // MouseUp(0) messages (MouseUp(0) is a drag ended, and
                    // MouseUp(id) if a drag succeeded).
                    //
                    // TODO: figure out a way for the drag target to tell the drag
                    // source "I got this" and suppress the MouseUp(0) message. Or
                    // just deal with how it is now.
                    if id == 0 {
                        eprintln!("Drag ended.");
                        self.is_dragging = false;
                    } else {
                        self.target_id = id;
                        eprintln!(
                            "Drag completed from {} to {}",
                            self.source_id, self.target_id
                        );
                        //   self.connect_points();
                    }
                    None
                }
                MainViewThingyMessage::Connect(_controller_id, controllable_id, _control_index) => {
                    self.target_id = controllable_id;
                    eprintln!(
                        "Drag completed from {} to {}",
                        self.source_id, self.target_id
                    );
                    //                    self.connect_points();
                    return Some(message);
                }
            }
        }

        pub(crate) fn add_entity(&mut self, uid: usize, entity: &Entity) {
            match entity {
                Entity::Arpeggiator(e) => self.add_viewable_item(
                    uid,
                    EntityParams::Arpeggiator(Box::new(ArpeggiatorParams { bpm: 99.0 })),
                ),
                Entity::Bitcrusher(e) => self.add_viewable_item(
                    uid,
                    EntityParams::Bitcrusher(Box::new(BitcrusherParams { bits: 8 })),
                ),
                Entity::LfoController(e) => self.add_viewable_item(
                    uid,
                    EntityParams::LfoController(Box::new(LfoControllerParams {
                        waveform: WaveformParams::Sine, // TODO - map to actual (TODO - never do that, because this whole thing will go away with messages)
                        frequency: e.frequency(),
                    })),
                ),
                Entity::Sequencer(e) => self.add_viewable_item(
                    uid,
                    EntityParams::Sequencer(Box::new(SequencerParams { bpm: 112.0 })),
                ),
                Entity::ControlTrip(_) => todo!(),
                Entity::MidiTickSequencer(_) => todo!(),
                Entity::PatternManager(e) => self.add_viewable_item(
                    uid,
                    EntityParams::PatternManager(Box::new(PatternManagerParams {})),
                ),
                Entity::SignalPassthroughController(_) => todo!(),
                Entity::ToyController(_) => todo!(),
                Entity::Timer(_) => todo!(),
                Entity::BiQuadFilter(_) => todo!(),
                Entity::Chorus(_) => todo!(),
                Entity::Compressor(_) => todo!(),
                Entity::Delay(_) => todo!(),
                Entity::Gain(e) => self.add_viewable_item(
                    uid,
                    EntityParams::Gain(Box::new(GainParams {
                        ceiling: e.ceiling(),
                    })),
                ),
                Entity::Limiter(_) => todo!(),
                Entity::Mixer(e) => {
                    self.add_viewable_item(uid, EntityParams::Mixer(Box::new(MixerParams {})))
                }
                Entity::Reverb(e) => self.add_viewable_item(
                    uid,
                    EntityParams::Reverb(Box::new(ReverbParams {
                        attenuation: Normal::new(0.5),
                    })),
                ),
                Entity::ToyEffect(_) => todo!(),
                Entity::Drumkit(_) => todo!(),
                Entity::FmSynthesizer(_) => todo!(),
                Entity::Sampler(_) => todo!(),
                Entity::ToyAudioSource(_) => todo!(),
                Entity::ToyInstrument(_) => todo!(),
                Entity::ToySynth(_) => todo!(),
                Entity::WelshSynth(e) => self.add_viewable_item(
                    uid,
                    EntityParams::WelshSynth(Box::new(WelshSynthParams {
                        pan: BipolarNormal::from(e.pan()),
                    })),
                ),
            }
        }

        fn add_viewable_item(&mut self, uid: usize, item: EntityParams) {
            // TODO: do we care about displaced items that had the same key?
            self.entity_store.entities.insert(uid, Box::new(item));
        }

        pub(crate) fn add_temp_controller(&mut self, uid: &usize, entity: &Entity) {
            if !self.entity_store.entities.contains_key(uid) {
                self.add_entity(*uid, entity);
            }
            self.controller_uids.push(*uid);
        }

        pub(crate) fn add_temp_controllable(&mut self, uid: &usize, entity: &Entity) {
            let mut params = Vec::default();
            if let Some(controllable) = (*entity).as_controllable() {
                for i in 0..controllable.control_index_count() {
                    if let Some(name) = controllable.control_name_for_index(i) {
                        params.push(name.to_string());
                    }
                }
                if !self.entity_store.entities.contains_key(uid) {
                    self.add_entity(*uid, entity);
                }
                self.controllable_uids.push(*uid);
                self.controllable_uids_to_control_names.insert(*uid, params);
            }
        }
    }
}
