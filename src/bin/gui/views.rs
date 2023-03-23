// Copyright (c) 2023 Mike Tsao. All rights reserved.

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
    widget::{
        button, column, container, pick_list, row, text, text_input, Column, Container, Row, Text,
    },
    Alignment, Element, Length, Renderer, Theme,
};
use iced_audio::{FloatRange, HSlider, IntRange, Knob, Normal as IcedNormal, NormalParam};
use iced_aw::{
    native::Badge,
    style::{BadgeStyles, CardStyles},
    Card,
};
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

#[derive(Clone, Debug)]
pub enum AutomationMessage {
    MouseIn(usize),
    MouseOut(usize),
    MouseDown(usize),
    MouseUp(usize),
    Connect(usize, usize, usize),
}

#[derive(Debug)]
struct AutomationView {
    is_dragging: bool,
    source_id: usize,
    target_id: usize,

    controllers: Vec<Controller>,
    controllables: Vec<Controllable>,
    connections: Vec<(usize, usize)>,
}
impl AutomationView {
    fn new() -> Self {
        Self {
            is_dragging: false,
            source_id: 0,
            target_id: 0,
            controllers: Vec::default(),
            controllables: Vec::default(),
            connections: Default::default(),
        }
    }

    fn view(&self) -> Element<AutomationMessage> {
        let controller_columns = self.controllers.iter().enumerate().fold(
            Vec::default(),
            |mut v, (_index, controller)| {
                let column = Column::new();
                let controller_id = controller.uid;
                let card_style = if self.is_dragging {
                    if controller_id == self.source_id {
                        CardStyles::Primary
                    } else {
                        CardStyles::Default
                    }
                } else {
                    CardStyles::Default
                };
                let card = Card::new(
                    ControlTargetWidget::<AutomationMessage>::new(
                        Text::new(controller.name.to_string()),
                        if self.is_dragging && controller_id != self.source_id {
                            // entering the bounds of a potential target.
                            Some(AutomationMessage::MouseIn(controller_id))
                        } else {
                            None
                        },
                        if self.is_dragging && controller_id == self.target_id {
                            // leaving the bounds of a potential target
                            Some(AutomationMessage::MouseOut(controller_id))
                        } else {
                            None
                        },
                        if !self.is_dragging {
                            // starting a drag operation
                            Some(AutomationMessage::MouseDown(controller_id))
                        } else {
                            None
                        },
                        if self.is_dragging {
                            if controller_id == self.source_id {
                                // user pressed and released on source card
                                Some(AutomationMessage::MouseUp(0))
                            } else {
                                // ending the drag on a target
                                Some(AutomationMessage::MouseUp(controller_id))
                            }
                        } else {
                            None
                        },
                        if self.is_dragging && controller_id == self.source_id {
                            // ending the drag somewhere that's not the source... but it could be a target!
                            // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                            Some(AutomationMessage::MouseUp(0))
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

        let controllable_columns = self.controllables.iter().enumerate().fold(
            Vec::default(),
            |mut v, (_index, controllable)| {
                let mut column = Column::new();
                let controllable_id = controllable.uid;
                for (param_id, point) in controllable.controllables.iter().enumerate() {
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
                    let child = ControlTargetWidget::<AutomationMessage>::new(
                        Badge::new(Text::new(point.name.to_string())).style(badge_style),
                        if self.is_dragging && param_app_id != self.source_id {
                            // entering the bounds of a potential target.
                            Some(AutomationMessage::MouseIn(param_app_id))
                        } else {
                            None
                        },
                        if self.is_dragging && param_app_id == self.target_id {
                            // leaving the bounds of a potential target
                            Some(AutomationMessage::MouseOut(param_app_id))
                        } else {
                            None
                        },
                        if !self.is_dragging {
                            // starting a drag operation
                            Some(AutomationMessage::MouseDown(param_app_id))
                        } else {
                            None
                        },
                        if self.is_dragging && param_app_id != self.source_id {
                            // ending the drag on a target
                            //                            Some(AutomationMessage::MouseUp(id))
                            Some(AutomationMessage::Connect(
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
                            Some(AutomationMessage::MouseUp(0))
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
                    ControlTargetWidget::<AutomationMessage>::new(
                        Text::new(controllable.name.to_string()),
                        // Sources aren't targets.
                        None,
                        // Don't care.
                        None,
                        // starting a drag operation
                        Some(AutomationMessage::MouseDown(controllable_id)),
                        if self.is_dragging && controllable_id != self.source_id {
                            // ending the drag on a target
                            Some(AutomationMessage::MouseUp(controllable_id))
                        } else {
                            None
                        },
                        if self.is_dragging && controllable_id == self.source_id {
                            // ending the drag somewhere that's not the source... but it could be a target!
                            // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                            Some(AutomationMessage::MouseUp(0))
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

        let controller_row = controller_columns
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

    fn update(&mut self, message: AutomationMessage) -> Option<AutomationMessage> {
        match message {
            AutomationMessage::MouseDown(id) => {
                self.is_dragging = true;
                self.source_id = id;
                self.target_id = 0;
                eprintln!("Start dragging on {}", id);
            }
            AutomationMessage::MouseIn(id) => {
                // if dragging, highlight potential target
                self.target_id = id;
            }
            AutomationMessage::MouseOut(_id) => {
                // if dragging, un-highlight potential target
                self.target_id = 0;
            }
            AutomationMessage::MouseUp(id) => {
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
                    self.connect_points();
                }
            }
            AutomationMessage::Connect(_controller_id, controllable_id, _control_index) => {
                self.target_id = controllable_id;
                eprintln!(
                    "Drag completed from {} to {}",
                    self.source_id, self.target_id
                );
                self.connect_points();
                return Some(message);
            }
        }
        None
    }

    pub(crate) fn clear(&mut self) {
        self.controllables.clear();
        self.controllers.clear();
        self.connections.clear();
    }

    fn connect_points(&mut self) {
        self.connections.push((self.source_id, self.target_id));
        eprintln!("we just connected {} to {}", self.source_id, self.target_id);
        eprintln!("now our connections are {:?}", self.connections);
    }
}

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

/// A [Controllable] represents a view of something implementing the
/// [groove_core::traits::Controllable] trait for the Automation view pane.
#[derive(Debug)]
pub(crate) struct Controllable {
    pub uid: usize,
    pub name: String,
    pub controllables: Vec<ControlPoint>,
}
impl Controllable {
    pub fn new(uid: usize, name: &str, control_points: Vec<&str>) -> Self {
        let mut r = Self {
            uid: uid,
            name: name.to_string(),
            controllables: Vec::default(),
        };
        r.controllables = control_points.iter().fold(Vec::default(), |mut v, name| {
            v.push(ControlPoint::new(name));
            v
        });
        r
    }

    #[allow(dead_code)]
    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

/// A [ControlPoint] is one of the things that a [Controllable] allows to be
/// automated.
#[derive(Debug)]
pub(crate) struct ControlPoint {
    pub name: String,
}
impl ControlPoint {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

pub(crate) mod views {
    use super::{AutomationMessage, AutomationView, Controllable, Controller};
    use groove::Entity;
    use groove_core::{generators::Waveform, ParameterType};
    use groove_entities::{
        controllers::{ArpeggiatorParams, LfoController, LfoControllerParams, WaveformParams},
        effects::BitcrusherParams,
        WelshSynthMessage,
    };
    use groove_settings::WaveformType;
    use iced::{
        widget::{container, text, Column, Row},
        Element, Length,
    };
    use iced_aw::Card;
    use rustc_hash::FxHashMap;
    use strum_macros::IntoStaticStr;

    #[derive(Clone, Debug)]
    pub enum AudioLaneMessage {
        ArpeggiatorMessage(usize, ArpeggiatorMessage),
        BitcrusherMessage(usize, BitcrusherMessage),
        DrumkitMessage(usize, DrumkitMessage),
        LfoControllerMessage(usize, LfoControllerMessage),
        ReverbMessage(usize, ReverbMessage),
        WelshSynthMessage(usize, WelshSynthMessage),
    }

    #[derive(Debug)]
    pub(crate) struct AudioLane {
        pub name: String,
        pub items: Vec<usize>,
    }

    trait Viewable<Message> {
        type Message;

        fn view(&self) -> Element<Self::Message>;
    }

    #[derive(Clone, Debug)]
    pub enum ArpeggiatorMessage {
        Bpm(ParameterType),
    }

    impl Viewable<ArpeggiatorMessage> for ArpeggiatorParams {
        type Message = ArpeggiatorMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("bpm: {}", self.bpm()))).into()
        }
    }

    #[derive(Clone, Debug)]
    pub enum BitcrusherMessage {
        Bits(u8),
    }

    impl Viewable<BitcrusherMessage> for BitcrusherParams {
        type Message = BitcrusherMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("bits: {}", self.bits()))).into()
        }
    }

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

    #[derive(Clone, Debug)]
    pub enum LfoControllerMessage {
        Waveform(WaveformParams),
        Frequency(ParameterType),
    }

    impl Viewable<LfoControllerMessage> for LfoControllerParams {
        type Message = LfoControllerMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!(
                "waveform: {:?} frequency: {}",
                self.waveform(), // TODO: proper string conversion
                self.frequency()
            )))
            .into()
        }
    }

    #[derive(Debug)]
    pub(crate) struct WelshSynthView {
        pan: f32,
    }
    impl Viewable<WelshSynthMessage> for WelshSynthView {
        type Message = WelshSynthMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("pan: {}", self.pan))).into()
        }
    }

    #[derive(Debug, IntoStaticStr)]
    pub(crate) enum ViewableItems {
        Arpeggiator(ArpeggiatorParams),
        BiQuadFilter,
        Bitcrusher(BitcrusherParams),
        Chorus,
        Compressor,
        ControlTrip,
        Delay,
        FmSynthesizer,
        Gain,
        LfoController(LfoControllerParams),
        Limiter,
        MidiTickSequencer,
        Mixer,
        PatternManager,
        Sampler,
        Sequencer,
        SignalPassthroughController,
        Timer,
        ToyAudioSource,
        ToyController,
        ToyEffect,
        ToyInstrument,
        ToySynth,
        WelshSynth(WelshSynthView),
        Drumkit(DrumkitView),
        Reverb(ReverbView),
    }

    #[derive(Clone, Debug)]
    pub enum ReverbMessage {
        Amount(f32),
    }

    #[derive(Debug)]
    pub(crate) struct ReverbView {
        amount: f32,
    }
    impl Viewable<ReverbMessage> for ReverbView {
        type Message = ReverbMessage;

        fn view(&self) -> Element<Self::Message> {
            container(text(&format!("amount: {}", self.amount))).into()
        }
    }

    #[derive(Debug)]
    struct AudioLaneView {
        viewable_items: FxHashMap<usize, Box<ViewableItems>>,
        pub(crate) lanes: Vec<AudioLane>,
    }
    impl AudioLaneView {
        fn new() -> Self {
            let mut r = Self {
                viewable_items: Default::default(),
                lanes: Default::default(),
            };
            r.viewable_items.insert(
                1,
                Box::new(ViewableItems::Drumkit(DrumkitView { cowbell: 0.5 })),
            );
            r.viewable_items.insert(
                2,
                Box::new(ViewableItems::Reverb(ReverbView { amount: 0.1 })),
            );
            r.viewable_items.insert(
                3,
                Box::new(ViewableItems::Drumkit(DrumkitView { cowbell: -0.25 })),
            );
            r.viewable_items.insert(
                4,
                Box::new(ViewableItems::Reverb(ReverbView { amount: 0.0 })),
            );
            r.viewable_items.insert(
                5,
                Box::new(ViewableItems::WelshSynth(WelshSynthView { pan: 0.14159 })),
            );

            r.lanes = vec![
                AudioLane {
                    name: String::from("Rhythm"),
                    items: vec![1, 2],
                },
                AudioLane {
                    name: String::from("Rhythm B"),
                    items: vec![3, 4],
                },
                AudioLane {
                    name: String::from("Lead"),
                    items: vec![5],
                },
            ];
            r
        }

        fn view(&self) -> Element<AudioLaneMessage> {
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
                                    if let Some(item) = self.viewable_items.get(uid) {
                                        let name: &'static str = item.as_ref().into();
                                        let view = match item.as_ref() {
                                            ViewableItems::WelshSynth(e) => {
                                                e.view().map(move |message| {
                                                    AudioLaneMessage::WelshSynthMessage(
                                                        *uid, message,
                                                    )
                                                })
                                            }
                                            ViewableItems::Drumkit(e) => {
                                                e.view().map(move |message| {
                                                    AudioLaneMessage::DrumkitMessage(*uid, message)
                                                })
                                            }
                                            ViewableItems::Reverb(e) => {
                                                e.view().map(move |message| {
                                                    AudioLaneMessage::ReverbMessage(*uid, message)
                                                })
                                            }
                                            ViewableItems::Arpeggiator(e) => {
                                                e.view().map(move |message| {
                                                    AudioLaneMessage::ArpeggiatorMessage(
                                                        *uid, message,
                                                    )
                                                })
                                            }
                                            ViewableItems::BiQuadFilter => todo!(),
                                            ViewableItems::Bitcrusher(e) => {
                                                e.view().map(move |message| {
                                                    AudioLaneMessage::BitcrusherMessage(
                                                        *uid, message,
                                                    )
                                                })
                                            }
                                            ViewableItems::Chorus => todo!(),
                                            ViewableItems::Compressor => todo!(),
                                            ViewableItems::ControlTrip => todo!(),
                                            ViewableItems::Delay => todo!(),
                                            ViewableItems::FmSynthesizer => todo!(),
                                            ViewableItems::Gain => todo!(),
                                            ViewableItems::LfoController(e) => {
                                                e.view().map(move |message| {
                                                    AudioLaneMessage::LfoControllerMessage(
                                                        *uid, message,
                                                    )
                                                })
                                            }
                                            ViewableItems::Limiter => todo!(),
                                            ViewableItems::MidiTickSequencer => todo!(),
                                            ViewableItems::Mixer => todo!(),
                                            ViewableItems::PatternManager => todo!(),
                                            ViewableItems::Sampler => todo!(),
                                            ViewableItems::Sequencer => todo!(),
                                            ViewableItems::SignalPassthroughController => todo!(),
                                            ViewableItems::Timer => todo!(),
                                            ViewableItems::ToyAudioSource => todo!(),
                                            ViewableItems::ToyController => todo!(),
                                            ViewableItems::ToyEffect => todo!(),
                                            ViewableItems::ToyInstrument => todo!(),
                                            ViewableItems::ToySynth => todo!(),
                                        };
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

        fn update(&mut self, message: AudioLaneMessage) -> Option<AudioLaneMessage> {
            match message {
                AudioLaneMessage::ArpeggiatorMessage(uid, message) => {
                    if let Some(entity) = self.viewable_items.get_mut(&uid) {
                        if let ViewableItems::Arpeggiator(entity) = entity.as_mut() {
                            match message {
                                ArpeggiatorMessage::Bpm(bpm) => entity.set_bpm(bpm),
                            }
                        }
                    }
                }
                AudioLaneMessage::BitcrusherMessage(uid, message) => {
                    if let Some(entity) = self.viewable_items.get_mut(&uid) {
                        if let ViewableItems::Bitcrusher(entity) = entity.as_mut() {
                            match message {
                                BitcrusherMessage::Bits(bits) => entity.set_bits(bits),
                            }
                        }
                    }
                }
                AudioLaneMessage::DrumkitMessage(uid, message) => {
                    if let Some(entity) = self.viewable_items.get_mut(&uid) {
                        if let ViewableItems::Drumkit(entity) = entity.as_mut() {
                            match message {
                                DrumkitMessage::Cowbell(cowbell) => {
                                    entity.cowbell = cowbell;
                                }
                            }
                        }
                    }
                }
                AudioLaneMessage::LfoControllerMessage(uid, message) => {
                    if let Some(entity) = self.viewable_items.get_mut(&uid) {
                        if let ViewableItems::LfoController(entity) = entity.as_mut() {
                            match message {
                                LfoControllerMessage::Waveform(waveform) => {
                                    entity.set_waveform(waveform.into())
                                }
                                LfoControllerMessage::Frequency(frequency) => {
                                    entity.set_frequency(frequency)
                                }
                            }
                        }
                    }
                }
                AudioLaneMessage::ReverbMessage(uid, message) => {
                    if let Some(entity) = self.viewable_items.get_mut(&uid) {
                        if let ViewableItems::Reverb(entity) = entity.as_mut() {
                            match message {
                                ReverbMessage::Amount(amount) => entity.amount = amount,
                            }
                        }
                    }
                }
                AudioLaneMessage::WelshSynthMessage(uid, message) => {
                    if let Some(entity) = self.viewable_items.get_mut(&uid) {
                        if let ViewableItems::WelshSynth(entity) = entity.as_mut() {
                            match message {
                                WelshSynthMessage::Pan(pan) => {
                                    entity.pan = pan;
                                }
                            }
                        }
                    }
                }
            }
            None
        }
    }

    #[derive(Debug)]
    pub(crate) struct MainViewThingy {
        automation_view: AutomationView,
        audio_lane_view: AudioLaneView,
    }

    impl MainViewThingy {
        pub(crate) fn new() -> Self {
            Self {
                automation_view: AutomationView::new(),
                audio_lane_view: AudioLaneView::new(),
            }
        }

        pub(crate) fn clear(&mut self) {
            self.automation_view.clear();
        }

        pub(crate) fn automation_view(&self) -> Element<AutomationMessage> {
            self.automation_view.view()
        }

        pub(crate) fn automation_update(
            &mut self,
            message: AutomationMessage,
        ) -> Option<AutomationMessage> {
            self.automation_view.update(message)
        }

        pub(crate) fn audio_lane_view(&self) -> Element<AudioLaneMessage> {
            self.audio_lane_view.view()
        }

        pub(crate) fn audio_lane_update(
            &mut self,
            message: AudioLaneMessage,
        ) -> Option<AudioLaneMessage> {
            self.audio_lane_view.update(message)
        }

        pub(crate) fn add_entity(&mut self, uid: usize, entity: &Entity) {
            match entity {
                Entity::Arpeggiator(e) => self.add_viewable_item(
                    uid,
                    ViewableItems::Arpeggiator(ArpeggiatorParams { bpm: 99.0 }),
                ),
                Entity::BiQuadFilter(e) => {
                    self.add_viewable_item(uid, ViewableItems::BiQuadFilter {})
                }
                Entity::Bitcrusher(e) => self.add_viewable_item(
                    uid,
                    ViewableItems::Bitcrusher(BitcrusherParams { bits: 8 }),
                ),
                Entity::Chorus(e) => self.add_viewable_item(uid, ViewableItems::Chorus {}),
                Entity::Compressor(e) => self.add_viewable_item(uid, ViewableItems::Compressor {}),
                Entity::ControlTrip(e) => {
                    self.add_viewable_item(uid, ViewableItems::ControlTrip {})
                }
                Entity::Delay(e) => self.add_viewable_item(uid, ViewableItems::Delay {}),
                Entity::Drumkit(e) => self
                    .add_viewable_item(uid, ViewableItems::Drumkit(DrumkitView { cowbell: 0.5 })),
                Entity::FmSynthesizer(e) => {
                    self.add_viewable_item(uid, ViewableItems::FmSynthesizer {})
                }
                Entity::Gain(e) => self.add_viewable_item(uid, ViewableItems::Gain {}),
                Entity::LfoController(e) => self.add_viewable_item(
                    uid,
                    ViewableItems::LfoController(LfoControllerParams {
                        waveform: WaveformParams::Sine,
                        frequency: 2.5,
                    }),
                ),
                Entity::Limiter(e) => self.add_viewable_item(uid, ViewableItems::Limiter {}),
                Entity::MidiTickSequencer(e) => {
                    self.add_viewable_item(uid, ViewableItems::MidiTickSequencer {})
                }
                Entity::Mixer(e) => self.add_viewable_item(uid, ViewableItems::Mixer {}),
                Entity::PatternManager(e) => {
                    self.add_viewable_item(uid, ViewableItems::PatternManager {})
                }
                Entity::Reverb(e) => {
                    self.add_viewable_item(uid, ViewableItems::Reverb(ReverbView { amount: 0.2 }))
                }
                Entity::Sampler(e) => self.add_viewable_item(uid, ViewableItems::Sampler {}),
                Entity::Sequencer(e) => self.add_viewable_item(uid, ViewableItems::Sequencer {}),
                Entity::SignalPassthroughController(e) => {
                    self.add_viewable_item(uid, ViewableItems::SignalPassthroughController {})
                }
                Entity::Timer(e) => self.add_viewable_item(uid, ViewableItems::Timer {}),
                Entity::ToyAudioSource(e) => {
                    self.add_viewable_item(uid, ViewableItems::ToyAudioSource {})
                }
                Entity::ToyController(e) => {
                    self.add_viewable_item(uid, ViewableItems::ToyController {})
                }
                Entity::ToyEffect(e) => self.add_viewable_item(uid, ViewableItems::ToyEffect {}),
                Entity::ToyInstrument(e) => {
                    self.add_viewable_item(uid, ViewableItems::ToyInstrument {})
                }
                Entity::ToySynth(e) => self.add_viewable_item(uid, ViewableItems::ToySynth {}),
                Entity::WelshSynth(e) => self.add_viewable_item(
                    uid,
                    ViewableItems::WelshSynth(WelshSynthView { pan: 0.334 }),
                ),
            }
        }

        fn add_viewable_item(&mut self, uid: usize, item: ViewableItems) {}

        pub(crate) fn add_temp_controller(&mut self, uid: &usize, entity: &Entity) {
            self.automation_view
                .controllers
                .push(Controller::new(*uid, (*entity).as_has_uid().name()));
        }

        pub(crate) fn add_temp_controllable(&mut self, uid: &usize, entity: &Entity) {
            let mut params = Vec::default();
            if let Some(controllable) = (*entity).as_controllable() {
                for i in 0..controllable.control_index_count() {
                    params.push(controllable.control_name_for_index(i));
                }
                self.automation_view.controllables.push(Controllable::new(
                    *uid,
                    (*entity).as_has_uid().name(),
                    params,
                ));
            }
        }
    }
}
