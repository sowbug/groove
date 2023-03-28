// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    GuiStuff, IconType, Icons, LARGE_FONT, LARGE_FONT_SIZE, NUMBERS_FONT, NUMBERS_FONT_SIZE,
    SMALL_FONT, SMALL_FONT_SIZE,
};
use groove::{app_version, Entity};
use groove_core::{
    time::{Clock, TimeSignature},
    traits::HasUid,
    BipolarNormal, Normal, ParameterType, StereoSample,
};
use groove_entities::{
    controllers::{
        Arpeggiator, ArpeggiatorParams, ArpeggiatorParamsMessage, ControlTrip, ControlTripParams,
        ControlTripParamsMessage, LfoController, LfoControllerParams, LfoControllerParamsMessage,
        MidiTickSequencer, MidiTickSequencerParams, MidiTickSequencerParamsMessage, Note, Pattern,
        PatternManager, PatternManagerParams, PatternManagerParamsMessage, PatternMessage,
        Sequencer, SequencerParams, SequencerParamsMessage, SignalPassthroughController,
        SignalPassthroughControllerParams, SignalPassthroughControllerParamsMessage, Timer,
        TimerParams, TimerParamsMessage,
    },
    effects::{
        BiQuadFilter, BiQuadFilterParams, BiQuadFilterParamsMessage, Bitcrusher, BitcrusherParams,
        BitcrusherParamsMessage, Chorus, ChorusParams, ChorusParamsMessage, Compressor,
        CompressorParams, CompressorParamsMessage, Delay, DelayParams, DelayParamsMessage, Gain,
        GainParams, GainParamsMessage, Limiter, LimiterParams, LimiterParamsMessage, Mixer,
        MixerParams, MixerParamsMessage, Reverb, ReverbParams, ReverbParamsMessage,
    },
    instruments::{
        Drumkit, DrumkitParams, DrumkitParamsMessage, FmSynth, FmSynthParams, FmSynthParamsMessage,
        Sampler, SamplerParams, SamplerParamsMessage, WelshSynth, WelshSynthParams,
        WelshSynthParamsMessage,
    },
    EntityMessage,
};
use groove_orchestration::{EntityParams, OtherEntityMessage};
use groove_toys::{
    ToyAudioSource, ToyAudioSourceParams, ToyAudioSourceParamsMessage, ToyController,
    ToyControllerParams, ToyControllerParamsMessage, ToyEffect, ToyEffectParams,
    ToyEffectParamsMessage, ToyInstrument, ToyInstrumentParams, ToyInstrumentParamsMessage,
    ToySynth, ToySynthParams, ToySynthParamsMessage,
};
use iced::{
    alignment, theme,
    widget::{
        button, column, container, pick_list, row, text, text_input, Column, Container, Row, Text,
    },
    Alignment, Element, Length, Renderer, Theme,
};
use iced_audio::{FloatRange, HSlider, IntRange, Knob, Normal as IcedNormal, NormalParam};
use iced_aw::{
    style::{BadgeStyles, CardStyles},
    Badge, Card,
};
use iced_native::{mouse, widget::Tree, Event, Widget};
use rustc_hash::FxHashMap;
use std::any::type_name;
use strum::EnumCount;
use strum_macros::{EnumCount as EnumCountMacro, FromRepr};

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
            Entity::FmSynth(e) => self.fm_synth_view(e),
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
                    value: IcedNormal::from_clipped(e.cutoff_pct().value_as_f32()),
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
                    value: IcedNormal::from_clipped(e.params().threshold().value_as_f32()),
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

    fn fm_synth_view(&self, e: &FmSynth) -> Element<EntityMessage> {
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
                format!("Fake value: {}", e.fake_value().value()).as_str(),
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
                format!("Runtime: {}", e.seconds_to_run()).as_str(),
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

#[derive(Debug)]
pub(crate) struct AudioLane {
    pub name: String,
    pub items: Vec<usize>,
}

trait Viewable {
    type Message;

    fn view(&self) -> Element<Self::Message> {
        container(text("coming soon")).into()
    }
}

impl Viewable for ArpeggiatorParams {
    type Message = ArpeggiatorParamsMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("bpm: {}", self.bpm()))).into()
    }
}

impl Viewable for BitcrusherParams {
    type Message = BitcrusherParamsMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("bits: {}", self.bits()))).into()
    }
}

impl Viewable for GainParams {
    type Message = GainParamsMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("ceiling: {}", self.ceiling().value()))).into()
    }
}

impl Viewable for LfoControllerParams {
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

impl Viewable for MixerParams {
    type Message = MixerParamsMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("I'm a mixer! {}", 261))).into()
    }
}

impl Viewable for PatternManagerParams {
    type Message = PatternManagerParamsMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("nothing {}", 42))).into()
    }
}

impl Viewable for ReverbParams {
    type Message = ReverbParamsMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!(
            "attenuation: {}",
            self.attenuation().value()
        )))
        .into()
    }
}

impl Viewable for SequencerParams {
    type Message = SequencerParamsMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("BPM: {}", self.bpm()))).into()
    }
}

impl Viewable for WelshSynthParams {
    type Message = WelshSynthParamsMessage;

    fn view(&self) -> Element<Self::Message> {
        let options = vec!["Acid Bass".to_string(), "Piano".to_string()];
        let pan_knob: Element<WelshSynthParamsMessage> = Knob::new(
            // TODO: toil. make it easier to go from bipolar normal to normal
            NormalParam {
                value: IcedNormal::from_clipped(Normal::from(self.pan()).value_as_f32()),
                default: IcedNormal::from_clipped(0.5),
            },
            |n| WelshSynthParamsMessage::Pan(BipolarNormal::from(n.as_f32())),
        )
        .into();
        let column = Column::new()
            .push(GuiStuff::<WelshSynthParamsMessage>::container_text(
                "Welsh coming soon",
            ))
            //                column.push(  pick_list(options, None, |s| {WelshSynthParamsMessage::Pan}).font(SMALL_FONT));
            .push(pan_knob);
        container(column).into()
    }
}

impl Viewable for ChorusParams {
    type Message = ChorusParamsMessage;

    fn view(&self) -> Element<Self::Message> {
        container(text(&format!("delay factor: {}", self.delay_factor()))).into()
    }
}

impl Viewable for ControlTripParams {
    type Message = ControlTripParamsMessage;
}

impl Viewable for MidiTickSequencerParams {
    type Message = MidiTickSequencerParamsMessage;
}

impl Viewable for SignalPassthroughControllerParams {
    type Message = SignalPassthroughControllerParamsMessage;
}

impl Viewable for TimerParams {
    type Message = TimerParamsMessage;
}

impl Viewable for ToyInstrumentParams {
    type Message = ToyInstrumentParamsMessage;
}

impl Viewable for ToyEffectParams {
    type Message = ToyEffectParamsMessage;
}

impl Viewable for ToySynthParams {
    type Message = ToySynthParamsMessage;
}

impl Viewable for ToyAudioSourceParams {
    type Message = ToyAudioSourceParamsMessage;
}

impl Viewable for BiQuadFilterParams {
    type Message = BiQuadFilterParamsMessage;
}

impl Viewable for ToyControllerParams {
    type Message = ToyControllerParamsMessage;
}

impl Viewable for CompressorParams {
    type Message = CompressorParamsMessage;
}

impl Viewable for DelayParams {
    type Message = DelayParamsMessage;
}

impl Viewable for LimiterParams {
    type Message = LimiterParamsMessage;
}
impl Viewable for SamplerParams {
    type Message = SamplerParamsMessage;
}
impl Viewable for FmSynthParams {
    type Message = FmSynthParamsMessage;
}
impl Viewable for DrumkitParams {
    type Message = DrumkitParamsMessage;
}

#[derive(Clone, Debug)]
pub(crate) enum ViewMessage {
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
pub(crate) enum ViewView {
    AudioLanes,
    #[default]
    Automation,
    Everything,
}

#[derive(Debug)]
pub(crate) struct View {
    current_view: ViewView,
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

macro_rules! build_view_dispatcher {
        ($($entity:ident; $params:tt; $params_message:tt ,)*) => {
            fn entity_view<'a>(&self, uid: usize, entity: &'a EntityParams) -> Element<'a, ViewMessage> {
                match entity {
                $(
                    EntityParams::$entity(e) => {
                        e.view().map(move |message| {
                            ViewMessage::OtherEntityMessage(
                                uid,
                                OtherEntityMessage::$params(
                                    message,
                                ),
                            )
                        })
                    } ),*
                }
            }

            fn entity_create(
                &mut self,
                uid: usize,
                message: OtherEntityMessage,
            )  {
                match message {
                    $(
                        OtherEntityMessage::$params($params_message::$params(params)) => {
                            self.add_entity(uid, EntityParams::$entity(Box::new(params)));
                        }
                        ),*
                        _ => {todo!();}
                }
            }
        }
    }

impl View {
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
        eprintln!("Clearing...");
        self.controller_uids.clear();
        self.controllable_uids.clear();
        self.controllable_uids_to_control_names.clear();
        self.connections.clear(); // TODO: this shouldn't exist, or else should be like the Params things (synchronized)
    }

    pub(crate) fn view(&self) -> Element<ViewMessage> {
        match self.current_view {
            ViewView::AudioLanes => self.audio_lane_view(),
            ViewView::Automation => self.automation_view(),
            ViewView::Everything => self.everything_view(),
        }
    }

    fn automation_view(&self) -> Element<ViewMessage> {
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
                let controller = self.entity_store.get(controller_uid).unwrap(); // TODO no unwraps!
                let card = Card::new(
                    ControlTargetWidget::<ViewMessage>::new(
                        Text::new("TBD"),
                        if self.is_dragging && controller_id != self.source_id {
                            // entering the bounds of a potential target.
                            Some(ViewMessage::MouseIn(controller_id))
                        } else {
                            None
                        },
                        if self.is_dragging && controller_id == self.target_id {
                            // leaving the bounds of a potential target
                            Some(ViewMessage::MouseOut(controller_id))
                        } else {
                            None
                        },
                        if !self.is_dragging {
                            // starting a drag operation
                            Some(ViewMessage::MouseDown(controller_id))
                        } else {
                            None
                        },
                        if self.is_dragging {
                            if controller_id == self.source_id {
                                // user pressed and released on source card
                                Some(ViewMessage::MouseUp(0))
                            } else {
                                // ending the drag on a target
                                Some(ViewMessage::MouseUp(controller_id))
                            }
                        } else {
                            None
                        },
                        if self.is_dragging && controller_id == self.source_id {
                            // ending the drag somewhere that's not the source... but it could be a target!
                            // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                            Some(ViewMessage::MouseUp(0))
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
                if let Some(controllable) = self.entity_store.get(controllable_uid) {
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
                            let child = ControlTargetWidget::<ViewMessage>::new(
                                Badge::new(Text::new(
                                    controllable
                                        .control_name_for_index(param_id)
                                        .unwrap_or_default()
                                        .to_string(),
                                ))
                                .style(badge_style),
                                if self.is_dragging && param_app_id != self.source_id {
                                    // entering the bounds of a potential target.
                                    Some(ViewMessage::MouseIn(param_app_id))
                                } else {
                                    None
                                },
                                if self.is_dragging && param_app_id == self.target_id {
                                    // leaving the bounds of a potential target
                                    Some(ViewMessage::MouseOut(param_app_id))
                                } else {
                                    None
                                },
                                if !self.is_dragging {
                                    // starting a drag operation
                                    Some(ViewMessage::MouseDown(param_app_id))
                                } else {
                                    None
                                },
                                if self.is_dragging && param_app_id != self.source_id {
                                    // ending the drag on a target
                                    Some(ViewMessage::Connect(
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
                                    Some(ViewMessage::MouseUp(0))
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
                            ControlTargetWidget::<ViewMessage>::new(
                                Text::new(
                                    format!("I don't know my name yet! {}", controllable_uid)
                                        .to_string(),
                                ),
                                // Sources aren't targets.
                                None,
                                // Don't care.
                                None,
                                // starting a drag operation
                                Some(ViewMessage::MouseDown(controllable_id)),
                                if self.is_dragging && controllable_id != self.source_id {
                                    // ending the drag on a target
                                    Some(ViewMessage::MouseUp(controllable_id))
                                } else {
                                    None
                                },
                                if self.is_dragging && controllable_id == self.source_id {
                                    // ending the drag somewhere that's not the source... but it could be a target!
                                    // we have to catch this case because nobody otherwise reports a mouseup outside their bounds.
                                    Some(ViewMessage::MouseUp(0))
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

    build_view_dispatcher! {
        // struct; params; message; is_controller; is_controllable,

        // Controllers
        Arpeggiator; ArpeggiatorParams; ArpeggiatorParamsMessage,
        ControlTrip; ControlTripParams; ControlTripParamsMessage,
        LfoController; LfoControllerParams; LfoControllerParamsMessage,
        MidiTickSequencer; MidiTickSequencerParams; MidiTickSequencerParamsMessage,
        PatternManager; PatternManagerParams; PatternManagerParamsMessage,
        Sequencer; SequencerParams; SequencerParamsMessage,
        SignalPassthroughController; SignalPassthroughControllerParams; SignalPassthroughControllerParamsMessage,
        Timer; TimerParams; TimerParamsMessage,
        ToyController; ToyControllerParams; ToyControllerParamsMessage,

        // Effects
        BiQuadFilter; BiQuadFilterParams; BiQuadFilterParamsMessage,
        Bitcrusher; BitcrusherParams; BitcrusherParamsMessage,
        Chorus; ChorusParams; ChorusParamsMessage,
        Compressor; CompressorParams; CompressorParamsMessage,
        Delay; DelayParams; DelayParamsMessage,
        Gain; GainParams; GainParamsMessage,
        Limiter; LimiterParams; LimiterParamsMessage,
        Mixer; MixerParams; MixerParamsMessage,
        Reverb; ReverbParams; ReverbParamsMessage,
    // both controller and effect...    SignalPassthroughController; SignalPassthroughControllerParams; SignalPassthroughControllerParamsMessage,
        ToyEffect; ToyEffectParams; ToyEffectParamsMessage,

        // Instruments
        Drumkit; DrumkitParams; DrumkitParamsMessage,
        FmSynth; FmSynthParams; FmSynthParamsMessage,
        Sampler; SamplerParams; SamplerParamsMessage,
        ToyAudioSource; ToyAudioSourceParams; ToyAudioSourceParamsMessage,
        ToyInstrument; ToyInstrumentParams; ToyInstrumentParamsMessage,
        ToySynth; ToySynthParams; ToySynthParamsMessage,
        WelshSynth; WelshSynthParams; WelshSynthParamsMessage,
    }

    fn everything_view(&self) -> Element<ViewMessage> {
        let boxes: Vec<Element<ViewMessage>> = self
            .entity_store
            .entities
            .iter()
            .map(|(uid, entity)| self.entity_view(*uid, entity))
            .collect();
        let column = boxes.into_iter().fold(Column::new(), |c, e| c.push(e));
        container(column).into()
    }

    fn audio_lane_view<'a, 'b: 'a>(&'a self) -> Element<'a, ViewMessage> {
        let lane_views = self
            .lanes
            .iter()
            .enumerate()
            .fold(Vec::default(), |mut v, (i, lane)| {
                let lane_row = Card::new(
                    text(&format!("Lane #{}: {}", i, lane.name)),
                    lane.items
                        .iter()
                        .enumerate()
                        .fold(Row::new(), |r, (item_index, uid)| {
                            if let Some(item) = self.entity_store.get(uid) {
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
                        }),
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

    pub(crate) fn update(&mut self, message: ViewMessage) -> Option<ViewMessage> {
        match message {
            ViewMessage::NextView => {
                self.current_view =
                    ViewView::from_repr((self.current_view as usize + 1) % ViewView::COUNT)
                        .unwrap_or_default();
                None
            }
            ViewMessage::OtherEntityMessage(uid, message) => {
                //                    self.entity_update(uid, message)},
                if let Some(entity) = self.entity_store.get_mut(&uid) {
                    entity.update(message);
                } else {
                    self.entity_create(uid, message);
                }
                None
            }
            ViewMessage::MouseDown(id) => {
                self.is_dragging = true;
                self.source_id = id;
                self.target_id = 0;
                eprintln!("Start dragging on {}", id);
                None
            }
            ViewMessage::MouseIn(id) => {
                // if dragging, highlight potential target
                self.target_id = id;
                None
            }
            ViewMessage::MouseOut(_id) => {
                // if dragging, un-highlight potential target
                self.target_id = 0;
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
                    self.target_id = id;
                    eprintln!(
                        "Drag completed from {} to {}",
                        self.source_id, self.target_id
                    );
                }
                None
            }
            ViewMessage::Connect(_controller_id, controllable_id, _control_index) => {
                self.target_id = controllable_id;
                eprintln!(
                    "Drag completed from {} to {}",
                    self.source_id, self.target_id
                );
                return Some(message);
            }
        }
    }

    fn add_entity(&mut self, uid: usize, item: EntityParams) {
        eprintln!("Adding item... {:?}", item);
        if item.is_controller() {
            self.controller_uids.push(uid);
        }
        if let Some(controllable) = item.as_controllable_ref() {
            self.controllable_uids.push(uid);

            let mut params = Vec::default();
            for i in 0..controllable.control_index_count() {
                if let Some(name) = controllable.control_name_for_index(i) {
                    params.push(name.to_string());
                }
            }
            if params.is_empty() {
                eprintln!(
                    "Warning: entity {} claims to be controllable but reports no controls",
                    uid
                );
            }
            self.controllable_uids_to_control_names.insert(uid, params);
        }

        // TODO: do we care about displaced items that had the same key?
        self.entity_store.entities.insert(uid, Box::new(item));
    }
}
