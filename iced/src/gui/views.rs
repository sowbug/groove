// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    GuiStuff, IconType, Icons, LARGE_FONT, LARGE_FONT_SIZE, NUMBERS_FONT, NUMBERS_FONT_SIZE,
    SMALL_FONT, SMALL_FONT_SIZE,
};
//use groove::{app_version, Orchestrator};
use groove_core::{
    time::{Clock, ClockMessage, ClockNano, TimeSignature},
    traits::{HasUid, MessageBounds},
    BipolarNormal, Normal, ParameterType, StereoSample,
};
use groove_entities::{
    controllers::{
        Arpeggiator, ArpeggiatorMessage, ArpeggiatorNano, ControlTrip, ControlTripMessage,
        ControlTripNano, LfoController, LfoControllerMessage, LfoControllerNano, MidiTickSequencer,
        MidiTickSequencerMessage, MidiTickSequencerNano, Note, Pattern, PatternManager,
        PatternManagerMessage, PatternManagerNano, PatternMessage, Sequencer, SequencerMessage,
        SequencerNano, SignalPassthroughController, SignalPassthroughControllerMessage,
        SignalPassthroughControllerNano, Timer, TimerMessage, TimerNano, Trigger, TriggerMessage,
        TriggerNano,
    },
    effects::{
        BiQuadFilterAllPass, BiQuadFilterAllPassMessage, BiQuadFilterAllPassNano,
        BiQuadFilterBandPass, BiQuadFilterBandPassMessage, BiQuadFilterBandPassNano,
        BiQuadFilterBandStop, BiQuadFilterBandStopMessage, BiQuadFilterBandStopNano,
        BiQuadFilterHighPass, BiQuadFilterHighPassMessage, BiQuadFilterHighPassNano,
        BiQuadFilterHighShelf, BiQuadFilterHighShelfMessage, BiQuadFilterHighShelfNano,
        BiQuadFilterLowPass12db, BiQuadFilterLowPass12dbMessage, BiQuadFilterLowPass12dbNano,
        BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbMessage, BiQuadFilterLowPass24dbNano,
        BiQuadFilterLowShelf, BiQuadFilterLowShelfMessage, BiQuadFilterLowShelfNano,
        BiQuadFilterNone, BiQuadFilterNoneMessage, BiQuadFilterNoneNano, BiQuadFilterPeakingEq,
        BiQuadFilterPeakingEqMessage, BiQuadFilterPeakingEqNano, Bitcrusher, BitcrusherMessage,
        BitcrusherNano, Chorus, ChorusMessage, ChorusNano, Compressor, CompressorMessage,
        CompressorNano, Delay, DelayMessage, DelayNano, Gain, GainMessage, GainNano, Limiter,
        LimiterMessage, LimiterNano, Mixer, MixerMessage, MixerNano, Reverb, ReverbMessage,
        ReverbNano,
    },
    instruments::{
        Drumkit, DrumkitMessage, DrumkitNano, FmSynth, FmSynthMessage, FmSynthNano, Metronome,
        MetronomeMessage, MetronomeNano, Sampler, SamplerMessage, SamplerNano, WelshSynth,
        WelshSynthMessage, WelshSynthNano,
    },
    EntityMessage,
};
use groove_orchestration::{
    messages::ControlLink, Entity, EntityNano, Orchestrator, OtherEntityMessage,
};
use groove_proc_macros::Views;
use groove_toys::{
    DebugSynth, DebugSynthMessage, DebugSynthNano, ToyAudioSource, ToyAudioSourceMessage,
    ToyAudioSourceNano, ToyController, ToyControllerMessage, ToyControllerNano, ToyEffect,
    ToyEffectMessage, ToyEffectNano, ToyInstrument, ToyInstrumentMessage, ToyInstrumentNano,
    ToySynth,
};
use groove_toys::{ToySynthMessage, ToySynthNano};
use iced::{
    alignment, theme,
    widget::{
        button, column, container, pick_list, row, text, text_input, Button, Column, Container,
        Row, Text,
    },
    Alignment, Element, Length, Renderer, Theme,
};
use iced_audio::{FloatRange, HSlider, IntRange, Knob, Normal as IcedNormal, NormalParam};
use iced_aw::{
    style::{BadgeStyles, CardStyles},
    Badge, Card,
};
use iced_native::{mouse, widget::Tree, Event, Widget};
use rustc_hash::{FxHashMap, FxHashSet};
use strum::EnumCount;
use strum_macros::{EnumCount as EnumCountMacro, FromRepr, IntoStaticStr};

#[derive(Clone, Debug, Default, PartialEq)]
pub enum EntityViewState {
    #[default]
    Collapsed,
    Expanded,
}

#[derive(Debug)]
#[deprecated]
pub struct EntityView {
    entity_view_states: FxHashMap<usize, EntityViewState>,
    entity_enabled_states: FxHashMap<usize, bool>,
    entity_audio_outputs: FxHashMap<usize, StereoSample>,
}
impl Default for EntityView {
    fn default() -> Self {
        Self {
            entity_view_states: Default::default(),
            entity_enabled_states: Default::default(),
            entity_audio_outputs: Default::default(),
        }
    }
}
impl EntityView {
    pub fn set_entity_view_state(&mut self, uid: usize, new_state: EntityViewState) {
        self.entity_view_states.insert(uid, new_state);
    }

    pub fn set_entity_enabled_state(&mut self, uid: usize, enabled: bool) {
        self.entity_enabled_states.insert(uid, enabled);
    }

    pub fn reset(&mut self) {
        self.entity_view_states.clear();
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
    pub fn update_audio_outputs(&mut self, uid: &usize, sample: &StereoSample) {
        self.entity_audio_outputs.insert(*uid, *sample);
    }
}

#[derive(Debug, Clone)]
pub enum ControlBarInput {
    SetClock(usize),
    SetBpm(f64),
    SetTimeSignature(TimeSignature),
}

#[derive(Debug, Clone)]
pub enum ControlBarEvent {
    Play,
    Stop,
    SkipToStart,
    Bpm(String),
    OpenProject,
    ExportWav,
    #[allow(dead_code)]
    ExportMp3,
    ToggleSettings,
}

#[derive(Debug)]
pub struct ControlBar {
    app_version: String,
    clock: Clock,
    audio_buffer_fullness: Normal,
}
impl ControlBar {
    pub fn new_with(app_version: &str, clock: Clock) -> Self {
        Self {
            app_version: app_version.to_string(),
            clock,
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

    pub fn set_clock(&mut self, frames: usize) {
        self.clock.seek(frames);
    }

    pub fn tick_batch(&mut self, count: usize) {
        self.clock.tick_batch(count);
    }

    fn set_bpm(&mut self, bpm: ParameterType) {
        self.clock.set_bpm(bpm);
    }

    fn set_time_signature(&mut self, time_signature: TimeSignature) {
        self.clock.set_time_signature(time_signature);
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
                        text(format!("{}", self.clock.time_signature().top))
                            .font(SMALL_FONT)
                            .size(SMALL_FONT_SIZE),
                    )
                    .push(
                        text(format!("{}", self.clock.time_signature().bottom))
                            .font(SMALL_FONT)
                            .size(SMALL_FONT_SIZE),
                    )
                    .width(Length::Fixed(16.0))
                    .align_items(Alignment::Center),
            )
        };

        let beat_counter = {
            let denom = self.clock.time_signature().top as f64;

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
                &format!("{:0.2}", self.clock.bpm()),
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
        let settings_button =
            Icons::button_icon(IconType::Settings).on_press(ControlBarEvent::ToggleSettings);
        let app_version =
            container(text(self.app_version.clone())).align_x(alignment::Horizontal::Right);

        container(
            Row::new()
                .push(audiobuf_container)
                .push(open_button)
                .push(export_wav_button)
                .push(export_mp3_button)
                .push(settings_button)
                .push(app_version)
                .align_items(Alignment::Center),
        )
    }

    pub fn set_audio_buffer_fullness(&mut self, audio_buffer_fullness: Normal) {
        self.audio_buffer_fullness = audio_buffer_fullness;
    }

    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        self.clock.set_sample_rate(sample_rate);
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
    entities: FxHashMap<usize, Box<EntityNano>>,
}
impl EntityStore {
    fn get(&self, uid: &usize) -> Option<&Box<EntityNano>> {
        self.entities.get(uid)
    }
    fn get_mut(&mut self, uid: &usize) -> Option<&mut Box<EntityNano>> {
        self.entities.get_mut(uid)
    }
}

#[derive(Debug)]
pub struct AudioLane {
    pub name: String,
    pub items: Vec<usize>,
}

trait Viewable {
    type Message;

    fn view(&self) -> Element<Self::Message> {
        container(text("coming soon")).into()
    }
}

impl Viewable for Arpeggiator {
    type Message = ArpeggiatorMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("bpm: {}", self.bpm()))).into()
    }
}

impl Viewable for Bitcrusher {
    type Message = BitcrusherMessage;

    fn view(&self) -> Element<Self::Message> {
        container(row![HSlider::new(
            IntRange::new(0, 15).normal_param(self.bits().into(), 8),
            |n| { BitcrusherMessage::Bits((n.as_f32() * 16.0) as u8) }
        )])
        .padding(View::ITEM_OUTER_PADDING)
        .into()
    }
}
impl Viewable for BiQuadFilterLowPass12db {
    type Message = BiQuadFilterLowPass12dbMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.cutoff()).value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterLowPass12dbMessage::Cutoff(Normal::from(n.as_f32()).into()),
        );
        row![
            container(slider).width(iced::Length::FillPortion(1)),
            container(GuiStuff::<EntityMessage>::container_text(&format!(
                "cutoff: {:.1}Hz q: {:0.3}",
                self.cutoff().value(),
                self.q()
            )))
            .width(iced::Length::FillPortion(1))
        ]
        .into()
    }
}
impl Viewable for BiQuadFilterLowPass24db {
    type Message = BiQuadFilterLowPass24dbMessage;

    fn view(&self) -> Element<Self::Message> {
        let cutoff_slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.cutoff()).value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterLowPass24dbMessage::Cutoff(Normal::from(n.as_f32()).into()),
        );
        let passband_slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(
                    Normal::from(self.passband_ripple()).value_as_f32(),
                ),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterLowPass24dbMessage::PassbandRipple(Normal::from(n.as_f32()).into()),
        );
        Row::new()
            .push(Container::new(cutoff_slider).width(iced::Length::FillPortion(1)))
            .push(Container::new(passband_slider).width(iced::Length::FillPortion(1)))
            .push(
                Container::new(GuiStuff::<EntityMessage>::container_text(&format!(
                    "cutoff: {:.1}Hz passband_ripple: {:0.3}",
                    self.cutoff().value(),
                    self.passband_ripple()
                )))
                .width(iced::Length::FillPortion(1)),
            )
            .into()
    }
}
impl Viewable for BiQuadFilterAllPass {
    type Message = BiQuadFilterAllPassMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.cutoff()).value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterAllPassMessage::Cutoff(Normal::from(n.as_f32()).into()),
        );
        row![
            container(slider).width(iced::Length::FillPortion(1)),
            container(GuiStuff::<EntityMessage>::container_text(&format!(
                "cutoff: {:.1}Hz q: {:0.3}",
                self.cutoff().value(),
                self.q()
            )))
            .width(iced::Length::FillPortion(1))
        ]
        .into()
    }
}
impl Viewable for BiQuadFilterHighPass {
    type Message = BiQuadFilterHighPassMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.cutoff()).value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterHighPassMessage::Cutoff(Normal::from(n.as_f32()).into()),
        );
        row![
            container(slider).width(iced::Length::FillPortion(1)),
            container(GuiStuff::<EntityMessage>::container_text(&format!(
                "cutoff: {:.1}Hz q: {:0.3}",
                self.cutoff().value(),
                self.q()
            )))
            .width(iced::Length::FillPortion(1))
        ]
        .into()
    }
}
impl Viewable for BiQuadFilterBandPass {
    type Message = BiQuadFilterBandPassMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.cutoff()).value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterBandPassMessage::Cutoff(Normal::from(n.as_f32()).into()),
        );
        row![
            container(slider).width(iced::Length::FillPortion(1)),
            container(GuiStuff::<EntityMessage>::container_text(&format!(
                "cutoff: {:.1}Hz bandwidth: {:0.3}",
                self.cutoff().value(),
                self.bandwidth()
            )))
            .width(iced::Length::FillPortion(1))
        ]
        .into()
    }
}
impl Viewable for BiQuadFilterBandStop {
    type Message = BiQuadFilterBandStopMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.cutoff()).value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterBandStopMessage::Cutoff(Normal::from(n.as_f32()).into()),
        );
        row![
            container(slider).width(iced::Length::FillPortion(1)),
            container(GuiStuff::<EntityMessage>::container_text(&format!(
                "cutoff: {:.1}Hz bandwidth: {:0.3}",
                self.cutoff().value(),
                self.bandwidth()
            )))
            .width(iced::Length::FillPortion(1))
        ]
        .into()
    }
}
impl Viewable for BiQuadFilterHighShelf {
    type Message = BiQuadFilterHighShelfMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.cutoff()).value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterHighShelfMessage::Cutoff(Normal::from(n.as_f32()).into()),
        );
        row![
            container(slider).width(iced::Length::FillPortion(1)),
            container(GuiStuff::<EntityMessage>::container_text(&format!(
                "cutoff: {:.1}Hz dB gain: {:0.3}",
                self.cutoff().value(),
                self.db_gain()
            )))
            .width(iced::Length::FillPortion(1))
        ]
        .into()
    }
}
impl Viewable for BiQuadFilterLowShelf {
    type Message = BiQuadFilterLowShelfMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.cutoff()).value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterLowShelfMessage::Cutoff(Normal::from(n.as_f32()).into()),
        );
        row![
            container(slider).width(iced::Length::FillPortion(1)),
            container(GuiStuff::<EntityMessage>::container_text(&format!(
                "cutoff: {:.1}Hz dB gain: {:0.3}",
                self.cutoff().value(),
                self.db_gain()
            )))
            .width(iced::Length::FillPortion(1))
        ]
        .into()
    }
}
impl Viewable for BiQuadFilterNone {
    type Message = BiQuadFilterNoneMessage;

    fn view(&self) -> Element<Self::Message> {
        row![
            container(GuiStuff::<EntityMessage>::container_text("(no parameters)"))
                .width(iced::Length::FillPortion(1))
        ]
        .into()
    }
}
impl Viewable for BiQuadFilterPeakingEq {
    type Message = BiQuadFilterPeakingEqMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.cutoff()).value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| BiQuadFilterPeakingEqMessage::Cutoff(Normal::from(n.as_f32()).into()),
        );
        row![
            container(slider).width(iced::Length::FillPortion(1)),
            container(GuiStuff::<EntityMessage>::container_text(&format!(
                "cutoff: {:.1}Hz Q (A/Q): {:0.3}",
                self.cutoff().value(),
                self.q()
            )))
            .width(iced::Length::FillPortion(1))
        ]
        .into()
    }
}
impl Viewable for Chorus {
    type Message = ChorusMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("delay seconds: {}", self.delay_seconds()))).into()
    }
}
impl Viewable for Clock {
    type Message = ClockMessage;

    fn view(&self) -> Element<Self::Message> {
        // TODO: oops, how do we get frames()?
        container(text(&format!("BPM: {}", self.bpm()))).into()
    }
}
impl Viewable for Compressor {
    type Message = CompressorMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(self.threshold().value_as_f32()),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| CompressorMessage::Threshold(n.as_f32().into()),
        );
        container(row![slider])
            .padding(View::ITEM_OUTER_PADDING)
            .into()
    }
}
impl Viewable for ControlTrip {
    type Message = ControlTripMessage;
}
impl Viewable for DebugSynth {
    type Message = DebugSynthMessage;
}
impl Viewable for Delay {
    type Message = DelayMessage;
}
impl Viewable for Drumkit {
    type Message = DrumkitMessage;
}
impl Viewable for FmSynth {
    type Message = FmSynthMessage;

    fn view(&self) -> Element<Self::Message> {
        let fm_synthesizer_ratio_range = IntRange::new(1, 32);
        let fm_synthesizer_beta_range = FloatRange::new(0.5, 32.0);

        let depth = self.depth().value();
        let label_depth = text("Depth").size(View::LABEL_FONT_SIZE);
        let text_depth =
            text(format!("{:0.1}%", depth * 100.0).as_str()).size(View::LABEL_FONT_SIZE);
        let ratio = self.ratio();
        let label_ratio = text("Ratio").size(View::LABEL_FONT_SIZE);
        let text_ratio =
            text(format!("{:0.2}", ratio.value()).as_str()).size(View::LABEL_FONT_SIZE);
        let beta = self.beta();
        let label_beta = text("Beta").size(View::LABEL_FONT_SIZE);
        let text_beta = text(format!("{:0.2}", beta).as_str()).size(View::LABEL_FONT_SIZE);
        let depth_slider = Column::new()
            .push(label_depth)
            .push(Knob::new(
                NormalParam {
                    value: IcedNormal::from_clipped(depth as f32),
                    default: IcedNormal::from_clipped(0.5),
                },
                |n| FmSynthMessage::Depth(n.as_f32().into()),
            ))
            .push(text_depth)
            .align_items(Alignment::Center)
            .padding(View::ITEM_PADDING)
            .width(View::ITEM_WIDTH);
        let ratio_slider = Column::new()
            .push(label_ratio)
            .push(Knob::new(
                fm_synthesizer_ratio_range.normal_param(ratio.value() as i32, 2),
                |n| FmSynthMessage::Ratio(n.as_f32().into()),
            ))
            .push(text_ratio)
            .align_items(Alignment::Center)
            .padding(View::ITEM_PADDING)
            .width(View::ITEM_WIDTH);
        let beta_slider = Column::new()
            .push(label_beta)
            .push(Knob::new(
                fm_synthesizer_beta_range.normal_param(beta as f32, 2.0),
                |n| FmSynthMessage::Beta(n.as_f32().into()),
            ))
            .push(text_beta)
            .align_items(Alignment::Center)
            .padding(View::ITEM_PADDING)
            .width(View::ITEM_WIDTH);
        container(row![depth_slider, ratio_slider, beta_slider])
            .padding(View::ITEM_OUTER_PADDING)
            .into()
    }
}
impl Viewable for Gain {
    type Message = GainMessage;

    fn view(&self) -> Element<Self::Message> {
        let slider = HSlider::new(
            NormalParam {
                value: IcedNormal::from_clipped(self.ceiling().value() as f32),
                default: IcedNormal::from_clipped(1.0),
            },
            |n| GainMessage::Ceiling(n.as_f32().into()),
        );
        container(row![slider])
            .padding(View::ITEM_OUTER_PADDING)
            .into()
    }
}
impl Viewable for LfoController {
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
impl Viewable for Limiter {
    type Message = LimiterMessage;
}
impl Viewable for Metronome {
    type Message = MetronomeMessage;
}
impl Viewable for MidiTickSequencer {
    type Message = MidiTickSequencerMessage;
}
impl Viewable for Mixer {
    type Message = MixerMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("I'm a mixer! {}", 261))).into()
    }
}
impl Viewable for PatternManager {
    type Message = PatternManagerMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("nothing {}", 42))).into()
    }
}

impl Viewable for Reverb {
    type Message = ReverbMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!(
            "attenuation: {} seconds: {}",
            self.attenuation().value(),
            self.seconds()
        )))
        .into()
    }
}
impl Viewable for Sampler {
    type Message = SamplerMessage;
}
impl Viewable for Sequencer {
    type Message = SequencerMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("BPM: {}", self.bpm()))).into()
    }
}
impl Viewable for SignalPassthroughController {
    type Message = SignalPassthroughControllerMessage;
}
impl Viewable for Timer {
    type Message = TimerMessage;
}
impl Viewable for ToyAudioSource {
    type Message = ToyAudioSourceMessage;
}
impl<M: MessageBounds> Viewable for ToyController<M> {
    type Message = ToyControllerMessage;
}
impl Viewable for ToyEffect {
    type Message = ToyEffectMessage;
}
impl Viewable for ToyInstrument {
    type Message = ToyInstrumentMessage;
}
impl Viewable for ToySynth {
    type Message = ToySynthMessage;
}
impl Viewable for Trigger {
    type Message = TriggerMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("Value: {}", self.value()))).into()
    }
}
impl Viewable for WelshSynth {
    type Message = WelshSynthMessage;

    fn view(&self) -> Element<Self::Message> {
        let _options = vec!["Acid Bass".to_string(), "Piano".to_string()];
        let pan_knob: Element<WelshSynthMessage> = Knob::new(
            // TODO: toil. make it easier to go from bipolar normal to normal
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.pan()).value_as_f32()),
                default: IcedNormal::from_clipped(0.5),
            },
            |n| WelshSynthMessage::Pan(BipolarNormal::from(n.as_f32())),
        )
        .into();
        let envelope = GuiStuff::envelope_view(self.envelope().clone());
        let filter_envelope = GuiStuff::envelope_view(self.filter_envelope().clone());
        let column = Column::new()
            .push(GuiStuff::<WelshSynthMessage>::container_text(
                "Welsh coming soon",
            ))
            //                column.push(  pick_list(options, None, |s| {WelshSynthMessage::Pan}).font(SMALL_FONT));
            .push(pan_knob)
            .push(envelope)
            .push(filter_envelope);
        container(column).into()
    }
}

#[derive(Clone, Debug)]
pub enum ViewMessage {
    NextView,
    OtherEntityMessage(usize, OtherEntityMessage),
    MouseIn(usize),
    MouseOut(usize),
    MouseDown(usize),
    MouseUp(usize),

    /// The receiver should add a control link.
    AddControlLink(ControlLink),

    /// The receiver should remove a control link.
    RemoveControlLink(ControlLink),
}

#[derive(Clone, Copy, Debug, Default, EnumCountMacro, FromRepr, IntoStaticStr)]
pub enum ViewView {
    AudioLanes,
    #[default]
    Automation,
    Everything,
}

#[derive(Debug)]
pub struct View {
    current_view: ViewView,
    is_dragging: bool,
    source_uid: usize,
    target_uid: usize,

    // controller_uids: Vec<usize>,
    // controllable_uids: Vec<usize>,
    // controllable_uids_to_control_names: FxHashMap<usize, Vec<String>>,
    // connections: FxHashSet<ControlLink>,
    lanes: Vec<AudioLane>,
}

impl View {
    pub const LABEL_FONT_SIZE: u16 = 14;

    pub const ITEM_OUTER_PADDING: u16 = 16;
    pub const ITEM_PADDING: u16 = 8;
    pub const ITEM_WIDTH: Length = Length::Fixed(48.0);

    pub fn new() -> Self {
        Self {
            current_view: Default::default(),
            //  entity_store: Default::default(),
            is_dragging: false,
            source_uid: 0,
            target_uid: 0,

            // controller_uids: Default::default(),
            // controllable_uids: Default::default(),
            // controllable_uids_to_control_names: Default::default(),
            // connections: Default::default(),
            lanes: Default::default(),
        }
    }

    pub fn clear(&mut self) {
        eprintln!("Clearing...");
        // self.controller_uids.clear();
        // self.controllable_uids.clear();
        // self.controllable_uids_to_control_names.clear();
        // self.connections.clear();
    }

    pub fn view<'a>(&self, orchestrator: &'a Orchestrator) -> Element<'a, ViewMessage> {
        Card::new(
            Text::new(<&str>::from(self.current_view)),
            match self.current_view {
                // ViewView::AudioLanes => self.audio_lane_view(orchestrator),
                ViewView::AudioLanes => self.everything_view(orchestrator), // hack hack
                ViewView::Automation => self.automation_view(orchestrator),
                ViewView::Everything => self.everything_view(orchestrator),
            },
        )
        .into()
    }

    fn automation_view<'a>(&self, orchestrator: &'a Orchestrator) -> Element<'a, ViewMessage> {
        let controller_views = orchestrator
            .entity_iter()
            .filter(|(entity_uid, e)| e.is_controller())
            .fold(Vec::default(), |mut v, (controller_uid, controller)| {
                if let Some(view) = self.automation_controller_view(orchestrator, *controller_uid) {
                    v.push(view);
                }
                v
            });

        let controllable_views = orchestrator
            .entity_iter()
            .filter(|(entity_uid, e)| e.is_controllable())
            .fold(Vec::default(), |mut v, (controllable_uid, controllable)| {
                if let Some(view) =
                    self.automation_controllable_view(orchestrator, *controllable_uid)
                {
                    v.push(view);
                }
                v
            });

        let connection_views =
            orchestrator
                .connections()
                .iter()
                .fold(Vec::default(), |mut v, link| {
                    if let Some(view) = self.automation_connection_view(orchestrator, link) {
                        v.push(view);
                    }
                    v
                });

        Container::new(
            Column::new()
                .push(
                    controller_views
                        .into_iter()
                        .fold(Row::new(), |mut row, item| {
                            row = row.push(item);
                            row
                        }),
                )
                .push(
                    controllable_views
                        .into_iter()
                        .fold(Row::new(), |mut row, item| {
                            row = row.push(item);
                            row
                        }),
                )
                .push(
                    connection_views
                        .into_iter()
                        .fold(Row::new(), |mut row, item| {
                            row = row.push(item);
                            row
                        }),
                ),
        )
        .into()
    }

    fn automation_controller_view<'a>(
        &self,
        orchestrator: &'a Orchestrator,
        controller_uid: usize,
    ) -> Option<Element<'a, ViewMessage>> {
        if let Some(controller) = orchestrator.get(controller_uid) {
            let style = if self.is_dragging {
                if controller_uid == self.source_uid {
                    BadgeStyles::Primary
                } else {
                    BadgeStyles::Default
                }
            } else {
                BadgeStyles::Default
            };
            Some(
                Badge::new(ControlTargetWidget::<ViewMessage>::new(
                    Text::new(controller.name()),
                    if self.is_dragging && controller_uid != self.source_uid {
                        // entering the bounds of a potential target.
                        Some(ViewMessage::MouseIn(controller_uid))
                    } else {
                        None
                    },
                    if self.is_dragging && controller_uid == self.target_uid {
                        // leaving the bounds of a potential target
                        Some(ViewMessage::MouseOut(controller_uid))
                    } else {
                        None
                    },
                    if !self.is_dragging {
                        // starting a drag operation
                        Some(ViewMessage::MouseDown(controller_uid))
                    } else {
                        None
                    },
                    if self.is_dragging {
                        if controller_uid == self.source_uid {
                            // user pressed and released on source card
                            Some(ViewMessage::MouseUp(0))
                        } else {
                            // ending the drag on a target
                            Some(ViewMessage::MouseUp(controller_uid))
                        }
                    } else {
                        None
                    },
                    if self.is_dragging && controller_uid == self.source_uid {
                        // ending the drag somewhere that's not the source... but it could be a target!
                        // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                        Some(ViewMessage::MouseUp(0))
                    } else {
                        None
                    },
                ))
                .style(style)
                .into(),
            )
        } else {
            None
        }
    }

    fn automation_controllable_view<'a>(
        &self,
        orchestrator: &'a Orchestrator,
        controllable_uid: usize,
    ) -> Option<Element<'a, ViewMessage>> {
        if let Some(entity) = orchestrator.get(controllable_uid) {
            if let Some(controllable) = entity.as_controllable() {
                let mut column = Column::new();
                for index in 0..controllable.control_index_count() {
                    if let Some(name) = controllable.control_name_for_index(index) {
                        column = column.push(self.automation_control_point_view(
                            orchestrator,
                            controllable_uid,
                            index,
                            name.to_string(),
                        ));
                    }
                }
                let card_style = if self.is_dragging {
                    if controllable_uid == self.source_uid {
                        CardStyles::Primary
                    } else if controllable_uid == self.target_uid {
                        CardStyles::Danger
                    } else {
                        CardStyles::Default
                    }
                } else {
                    CardStyles::Default
                };
                return Some(
                    Card::new(
                        ControlTargetWidget::<ViewMessage>::new(
                            Text::new(entity.name()),
                            // Sources aren't targets.
                            None,
                            // Don't care.
                            None,
                            // starting a drag operation
                            Some(ViewMessage::MouseDown(controllable_uid)),
                            if self.is_dragging && controllable_uid != self.source_uid {
                                // ending the drag on a target
                                Some(ViewMessage::MouseUp(controllable_uid))
                            } else {
                                None
                            },
                            if self.is_dragging && controllable_uid == self.source_uid {
                                // ending the drag somewhere that's not the source... but it could be a target!
                                // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                                Some(ViewMessage::MouseUp(0))
                            } else {
                                None
                            },
                        ),
                        column,
                    )
                    .style(card_style)
                    .into(),
                );
            }
        }
        None
    }

    // TODO: orchestrator is here to satisfy borrow checker. Is there a better way?
    fn automation_control_point_view<'a>(
        &self,
        _orchestrator: &'a Orchestrator,
        controllable_uid: usize,
        control_index: usize,
        name: String,
    ) -> ControlTargetWidget<'a, ViewMessage> {
        let control_app_uid = controllable_uid * 10000 + control_index;
        let badge_style = if self.is_dragging {
            if control_app_uid == self.source_uid {
                BadgeStyles::Danger // This shouldn't happen (I think) because it's a source, not a target
            } else if control_app_uid == self.target_uid {
                BadgeStyles::Success // Hovering over target, so highlight it specially
            } else {
                BadgeStyles::Info // Indicate that it's a potential target
            }
        } else {
            BadgeStyles::Default // Regular state
        };
        ControlTargetWidget::<ViewMessage>::new(
            Badge::new(Text::new(name)).style(badge_style),
            if self.is_dragging && control_app_uid != self.source_uid {
                // entering the bounds of a potential target.
                Some(ViewMessage::MouseIn(control_app_uid))
            } else {
                None
            },
            if self.is_dragging && control_app_uid == self.target_uid {
                // leaving the bounds of a potential target
                Some(ViewMessage::MouseOut(control_app_uid))
            } else {
                None
            },
            if !self.is_dragging {
                // starting a drag operation
                Some(ViewMessage::MouseDown(control_app_uid))
            } else {
                None
            },
            if self.is_dragging && control_app_uid != self.source_uid {
                // ending the drag on a target
                Some(ViewMessage::AddControlLink(ControlLink {
                    source_uid: self.source_uid,
                    target_uid: controllable_uid,
                    control_index,
                }))
            } else {
                None
            },
            if self.is_dragging && control_app_uid == self.source_uid {
                // ending the drag somewhere that's not the source... but it could be a target!
                // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                Some(ViewMessage::MouseUp(0))
            } else {
                None
            },
            // Other cases to handle
            // - leaving the window entirely
            // - keyboard stuff
        )
    }

    fn automation_connection_view<'a>(
        &self,
        orchestrator: &'a Orchestrator,
        link: &ControlLink,
    ) -> Option<Element<'a, ViewMessage>> {
        if let Some(controller) = orchestrator.get(link.source_uid) {
            if let Some(controllable_entity) = orchestrator.get(link.target_uid) {
                if let Some(controllable) = controllable_entity.as_controllable() {
                    if let Some(name) = controllable.control_name_for_index(link.control_index) {
                        let remove_button = Button::new(Text::new("X"))
                            .on_press(ViewMessage::RemoveControlLink(link.clone()));
                        return Some(
                            Row::new()
                                .push(Badge::new(Text::new(format!(
                                    "{} controls {}'s {}",
                                    controller.name(),
                                    controllable_entity.name(),
                                    name
                                ))))
                                .push(remove_button)
                                .into(),
                        );
                    }
                }
            }
        }
        None
    }

    fn everything_view<'a>(&self, orchestrator: &'a Orchestrator) -> Element<'a, ViewMessage> {
        let boxes: Vec<Element<ViewMessage>> = orchestrator
            .entity_iter()
            .map(|(uid, entity)| {
                Card::new(Text::new(entity.name()), self.entity_view(*uid, entity)).into()
            })
            .collect();
        let column = boxes.into_iter().fold(Column::new(), |c, e| c.push(e));
        container(column).into()
    }

    // fn audio_lane_view<'a, 'b: 'a>(
    //     &'a self,
    //     orchestrator: &Orchestrator,
    // ) -> Element<'a, ViewMessage> {
    //     let lane_views = self
    //         .lanes
    //         .iter()
    //         .enumerate()
    //         .fold(Vec::default(), |mut v, (i, lane)| {
    //             let lane_row = Card::new(
    //                 text(&format!("Lane #{}: {}", i, lane.name)),
    //                 lane.items
    //                     .iter()
    //                     .enumerate()
    //                     .fold(Row::new(), |r, (item_index, uid)| {
    //                         if let Some(item) = self.entity_store.get(uid) {
    //                             let name: &'static str = "no idea 2345983495";
    //                             let view = self.entity_view(*uid, item.as_ref());
    //                             r.push(
    //                                 Card::new(
    //                                     text(&format!("#{}: {}", item_index, name)),
    //                                     container(view).height(Length::Fill),
    //                                 )
    //                                 .width(Length::FillPortion(1))
    //                                 .height(Length::FillPortion(1)),
    //                             )
    //                         } else {
    //                             r
    //                         }
    //                     }),
    //             );
    //             v.push(lane_row);
    //             v
    //         });
    //     let view_column = lane_views
    //         .into_iter()
    //         .fold(Column::new(), |c, lane_view| {
    //             c.push(lane_view.height(Length::FillPortion(1)))
    //         })
    //         .width(Length::Fill)
    //         .height(Length::Fill);
    //     let mixer = Card::new(text("Mixer"), text("coming soon").height(Length::Fill))
    //         .width(Length::Fixed(96.0));
    //     let overall_view = Row::new().height(Length::Fill);
    //     container(overall_view.push(view_column).push(mixer)).into()
    // }

    pub fn update(
        &mut self,
        orchestrator: &mut Orchestrator,
        message: ViewMessage,
    ) -> Option<ViewMessage> {
        match message {
            ViewMessage::NextView => {
                self.current_view =
                    ViewView::from_repr((self.current_view as usize + 1) % ViewView::COUNT)
                        .unwrap_or_default();
                None
            }
            ViewMessage::OtherEntityMessage(uid, message) => {
                if let Some(entity) = orchestrator.get_mut(uid) {
                    entity.update(message);
                }
                // //                    self.entity_update(uid, message)},
                // if let Some(entity) = self.entity_store.get_mut(&uid) {
                //     entity.update(message);
                // } else {
                //     self.entity_create(uid, message);
                // }
                None
            }
            ViewMessage::MouseDown(id) => {
                self.is_dragging = true;
                self.source_uid = id;
                self.target_uid = 0;
                eprintln!("Start dragging on {}", id);
                None
            }
            ViewMessage::MouseIn(id) => {
                // if dragging, highlight potential target
                self.target_uid = id;
                None
            }
            ViewMessage::MouseOut(_uid) => {
                // if dragging, un-highlight potential target
                self.target_uid = 0;
                None
            }
            ViewMessage::MouseUp(id) => {
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
                    self.target_uid = id;
                    eprintln!(
                        "Drag completed from {} to {}",
                        self.source_uid, self.target_uid
                    );
                }
                None
            }
            ViewMessage::AddControlLink(link) => {
                self.target_uid = link.target_uid;
                eprintln!(
                    "Drag completed from {} to {}",
                    &self.source_uid, &self.target_uid
                );
                return Some(ViewMessage::AddControlLink(link));
            }
            ViewMessage::RemoveControlLink(link) => {
                eprintln!("Widget asked to remove link {:?}", link);
                return Some(ViewMessage::RemoveControlLink(link));
            }
        }
    }

    // fn add_entity(&mut self, uid: usize, item: EntityNano) {
    //     if item.is_controller() {
    //         self.controller_uids.push(uid);
    //     }
    //     if let Some(controllable) = item.as_controllable() {
    //         self.controllable_uids.push(uid);

    //         let mut params = Vec::default();
    //         for i in 0..controllable.control_index_count() {
    //             if let Some(name) = controllable.control_name_for_index(i) {
    //                 params.push(name.to_string());
    //             }
    //         }
    //         if params.is_empty() {
    //             eprintln!(
    //                 "Warning: entity {} claims to be controllable but reports no controls",
    //                 uid
    //             );
    //         }
    //         self.controllable_uids_to_control_names.insert(uid, params);
    //     }

    //     // TODO: do we care about displaced items that had the same key?
    //     self.entity_store.entities.insert(uid, Box::new(item));
    // }

    // pub fn add_control_link(&mut self, link: ControlLink) {
    //     self.connections.insert(link);
    // }

    // pub fn remove_control_link(&mut self, link: ControlLink) {
    //     self.connections.remove(&link);
    // }
}

/// The #[derive(Views)] macro uses [ViewableEntities] to generate scaffolding.
/// The enum itself is otherwise unused.
#[allow(dead_code)]
#[derive(Views)]
enum ViewableEntities {
    #[views(controller, midi, controllable)]
    Arpeggiator(Arpeggiator),

    #[views(effect, controllable)]
    BiQuadFilterAllPass(BiQuadFilterAllPass),

    #[views(effect, controllable)]
    BiQuadFilterBandPass(BiQuadFilterBandPass),

    #[views(effect, controllable)]
    BiQuadFilterBandStop(BiQuadFilterBandStop),

    #[views(effect, controllable)]
    BiQuadFilterHighPass(BiQuadFilterHighPass),

    #[views(effect, controllable)]
    BiQuadFilterHighShelf(BiQuadFilterHighShelf),

    #[views(effect, controllable)]
    BiQuadFilterLowPass12db(BiQuadFilterLowPass12db),

    #[views(effect, controllable)]
    BiQuadFilterLowPass24db(BiQuadFilterLowPass24db),

    #[views(effect, controllable)]
    BiQuadFilterLowShelf(BiQuadFilterLowShelf),

    #[views(effect, controllable)]
    BiQuadFilterNone(BiQuadFilterNone),

    #[views(effect, controllable)]
    BiQuadFilterPeakingEq(BiQuadFilterPeakingEq),

    #[views(effect, controllable)]
    Bitcrusher(Bitcrusher),

    #[views(effect, controllable)]
    Chorus(Chorus),

    #[views(effect, controllable)]
    Clock(Clock),

    #[views(effect, controllable)]
    Compressor(Compressor),

    #[views(controller, midi)]
    ControlTrip(ControlTrip),

    #[views(instrument, midi, controllable)]
    DebugSynth(DebugSynth),

    #[views(effect, controllable)]
    Delay(Delay),

    #[views(instrument, midi)]
    Drumkit(Drumkit),

    #[views(instrument, midi, controllable)]
    FmSynth(FmSynth),

    #[views(effect, controllable)]
    Gain(Gain),

    #[views(controller, midi)]
    LfoController(LfoController),

    #[views(effect, controllable)]
    Limiter(Limiter),

    #[views(controllable, instrument)]
    Metronome(Metronome),

    #[views(controller, midi)]
    MidiTickSequencer(MidiTickSequencer),

    #[views(effect)]
    Mixer(Mixer),

    #[views(controller, midi)]
    PatternManager(PatternManager),

    #[views(effect, controllable)]
    Reverb(Reverb),

    #[views(instrument, midi)]
    Sampler(Sampler),

    #[views(controller, midi)]
    Sequencer(Sequencer),

    #[views(controller, effect, midi)]
    SignalPassthroughController(SignalPassthroughController),

    #[views(controller, midi)]
    Timer(Timer),

    #[views(instrument, midi)]
    ToyAudioSource(ToyAudioSource),

    #[views(controller, midi)]
    ToyController(ToyController<EntityMessage>),

    #[views(effect, controllable)]
    ToyEffect(ToyEffect),

    #[views(instrument, midi, controllable)]
    ToyInstrument(ToyInstrument),

    #[views(instrument, midi, controllable)]
    ToySynth(ToySynth),

    #[views(controllable)]
    Trigger(Trigger),

    #[views(instrument, midi, controllable)]
    WelshSynth(WelshSynth),
}