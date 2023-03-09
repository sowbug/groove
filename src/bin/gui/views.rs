use super::{GuiStuff, LARGE_FONT, LARGE_FONT_SIZE, SMALL_FONT};
use groove::Entity;
use groove_core::traits::HasUid;
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
    widget::{button, column, container, pick_list, row, text},
    Element,
};
use iced_audio::{HSlider, IntRange, Knob, Normal as IcedNormal, NormalParam};
use rustc_hash::FxHashMap;
use std::any::type_name;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum EntityViewState {
    #[default]
    Collapsed,
    Expanded,
}

#[derive(Debug, Default)]
pub(crate) struct EntityViewGenerator {
    entity_view_states: FxHashMap<usize, EntityViewState>,
}

impl EntityViewGenerator {
    pub(crate) fn set_entity_view_state(&mut self, uid: usize, new_state: EntityViewState) {
        self.entity_view_states.insert(uid, new_state);
    }

    pub(crate) fn reset(&mut self) {
        self.entity_view_states.clear();
    }

    pub(crate) fn entity_view(&self, entity: &Entity) -> Element<EntityMessage> {
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
        let title = type_name::<Arpeggiator>();
        let contents = format!("Coming soon: {}", e.uid());
        GuiStuff::titled_container(
            title,
            GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
        )
    }

    fn audio_source_view(&self, e: &ToyAudioSource) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<ToyAudioSource>(),
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into(),
        )
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
            IntRange::new(0, 15).normal_param(e.bits_to_crush().into(), 8),
            EntityMessage::HSliderInt
        )])
        .padding(20);
        GuiStuff::titled_container(&title, contents.into())
    }

    fn chorus_view(&self, e: &Chorus) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<Chorus>(),
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into(),
        )
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

    fn control_trip_view(&self, e: &ControlTrip) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<ControlTrip>(),
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into(),
        )
    }

    fn delay_view(&self, e: &Delay) -> Element<EntityMessage> {
        let title = type_name::<Delay>();
        let contents = format!("delay in seconds: {}", e.seconds());
        GuiStuff::titled_container(
            title,
            GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
        )
    }

    fn drumkit_view(&self, e: &Drumkit) -> Element<EntityMessage> {
        let title = type_name::<Drumkit>();
        let contents = format!("kit name: {}", e.kit_name());
        GuiStuff::titled_container(
            title,
            GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
        )
    }

    fn entity_view_state(&self, uid: usize) -> EntityViewState {
        if let Some(state) = self.entity_view_states.get(&uid) {
            state.clone()
        } else {
            EntityViewState::default()
        }
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

    fn limiter_view(&self, e: &Limiter) -> Element<EntityMessage> {
        let title = type_name::<Limiter>();
        let contents = format!("min: {} max: {}", e.min(), e.max());
        GuiStuff::titled_container(
            title,
            GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
        )
    }

    fn midi_tick_sequencer_view(&self, e: &MidiTickSequencer) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<MidiTickSequencer>(),
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into(),
        )
    }

    fn mixer_view(&self, e: &Mixer) -> Element<EntityMessage> {
        self.collapsing_box("Mixer", e.uid(), || {
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
        GuiStuff::titled_container(
            type_name::<Reverb>(),
            GuiStuff::<EntityMessage>::container_text(format!("Coming soon: {}", e.uid()).as_str())
                .into(),
        )
    }

    fn sampler_view(&self, e: &Sampler) -> Element<EntityMessage> {
        let title = type_name::<Sampler>();
        let contents = format!("Coming soon: {}", e.uid());
        GuiStuff::titled_container(
            title,
            GuiStuff::<EntityMessage>::container_text(contents.as_str()).into(),
        )
    }

    fn sequencer_view(&self, e: &Sequencer) -> Element<EntityMessage> {
        self.collapsing_box("Sequencer", e.uid(), || {
            let contents = format!("{}", e.next_instant());
            GuiStuff::<EntityMessage>::container_text(contents.as_str()).into()
        })
    }

    fn signal_controller_view(&self, _: &SignalPassthroughController) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<SignalPassthroughController>(),
            GuiStuff::<EntityMessage>::container_text("nothing").into(),
        )
    }

    fn test_instrument_view(&self, e: &ToyInstrument) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<ToyInstrument>(),
            GuiStuff::<EntityMessage>::container_text(
                format!("Fake value: {}", e.fake_value()).as_str(),
            )
            .into(),
        )
    }

    fn test_synth_view(&self, _: &ToySynth) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<ToySynth>(),
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

    fn toy_controller_view(&self, e: &ToyController<EntityMessage>) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<ToyController<EntityMessage>>(),
            GuiStuff::<EntityMessage>::container_text(format!("Tempo: {}", e.tempo).as_str())
                .into(),
        )
    }

    fn toy_effect_view(&self, e: &ToyEffect) -> Element<EntityMessage> {
        GuiStuff::titled_container(
            type_name::<ToyEffect>(),
            GuiStuff::<EntityMessage>::container_text(format!("Value: {}", e.my_value()).as_str())
                .into(),
        )
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
                EntityMessage::IcedKnob,
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
}
