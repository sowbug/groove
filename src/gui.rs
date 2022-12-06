use crate::{
    controllers::{
        arpeggiator::Arpeggiator,
        sequencers::{BeatSequencer, MidiTickSequencer},
        ControlTrip,
    },
    effects::{
        bitcrusher::Bitcrusher, delay::Delay, filter::BiQuadFilter, gain::Gain, limiter::Limiter,
        mixer::Mixer,
    },
    instruments::{
        drumkit_sampler::Sampler as DrumkitSampler, envelopes::AdsrEnvelope,
        oscillators::Oscillator, sampler::Sampler, welsh::WelshSynth,
    },
    messages::{EntityMessage, GrooveMessage, MessageBounds},
    midi::{
        patterns::{Note, Pattern, PatternManager},
        MidiOutputHandler,
    },
    traits::{TestController, TestEffect, TestInstrument},
    utils::{AudioSource, Timer, Trigger},
    MidiHandler, Orchestrator,
};
use iced::{
    alignment::{Horizontal, Vertical},
    theme,
    widget::{button, column, container, row, slider, text, text_input},
    Color, Element, Font, Theme,
};
use iced_audio::{HSlider, Normal, NormalParam};
use std::{any::type_name, fmt::Debug, marker::PhantomData};

pub const SMALL_FONT_SIZE: u16 = 16;
pub const SMALL_FONT: Font = Font::External {
    name: "Small Font",
    bytes: include_bytes!("../res/fonts/SourceSansPro-Regular.ttf"),
};

pub const LARGE_FONT_SIZE: u16 = 20;
pub const LARGE_FONT: Font = Font::External {
    name: "Large Font",
    bytes: include_bytes!("../res/fonts/SourceSansPro-Regular.ttf"),
};

pub const NUMBERS_FONT_SIZE: u16 = 32;
pub const NUMBERS_FONT: Font = Font::External {
    name: "Numbers Font",
    bytes: include_bytes!("../res/fonts/NotoSansMono-Regular.ttf"),
};

pub trait Viewable {
    type ViewMessage;

    fn view(&self) -> Element<'_, Self::ViewMessage, iced::Renderer> {
        GuiStuff::titled_container(
            "Untitled",
            text("under construction")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center)
                .into(),
        )
    }
}

struct TitledContainerTitleStyle {
    theme: iced::Theme,
}

impl container::StyleSheet for TitledContainerTitleStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let palette = self.theme.extended_palette();
        container::Appearance {
            text_color: Some(palette.background.strong.text),
            background: Some(palette.background.strong.color.into()),
            ..Default::default()
        }
    }
}

struct NumberContainerStyle {
    _theme: iced::Theme,
}

impl container::StyleSheet for NumberContainerStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb8(255, 255, 0)),
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct GuiStuff<'a, Message> {
    phantom: PhantomData<&'a Message>,
}

impl<'a, Message: 'a> GuiStuff<'a, Message> {
    pub fn titled_container(title: &str, contents: Element<'a, Message>) -> Element<'a, Message> {
        container(column![
            Self::titled_container_title(title),
            container(contents).padding(2)
        ])
        .padding(0)
        .style(theme::Container::Box)
        .into()
    }

    #[allow(unused_variables)]
    pub fn titled_container_title(title: &str) -> Element<'a, Message> {
        // let checkboxes = container(if let Some(device) = device {
        //     row![
        //         checkbox(
        //             "Enabled".to_string(),
        //             device.borrow().is_enabled(),
        //             ViewableMessage::EnablePressed
        //         ),
        //         checkbox(
        //             "Muted".to_string(),
        //             device.borrow().is_muted(),
        //             ViewableMessage::MutePressed
        //         )
        //     ]
        // } else {
        //     row![text("".to_string())]
        // });
        container(row![
            text(title.to_string())
                .font(SMALL_FONT)
                .size(SMALL_FONT_SIZE)
                .horizontal_alignment(iced::alignment::Horizontal::Left)
                .vertical_alignment(Vertical::Center),
            // checkboxes
        ])
        .width(iced::Length::Fill)
        .padding(1)
        .style(theme::Container::Custom(
            Self::titled_container_title_style(&Theme::Dark),
        ))
        .into()
    }

    pub fn container_text(label: &str) -> Element<'a, Message> {
        text(label.to_string())
            .font(LARGE_FONT)
            .size(LARGE_FONT_SIZE)
            .horizontal_alignment(iced::alignment::Horizontal::Left)
            .vertical_alignment(Vertical::Center)
            .into()
    }

    fn titled_container_title_style(
        theme: &iced::Theme,
    ) -> Box<(dyn iced::widget::container::StyleSheet<Style = Theme>)> {
        Box::new(TitledContainerTitleStyle {
            theme: theme.clone(),
        })
    }

    pub fn number_box_style(
        theme: &iced::Theme,
    ) -> Box<(dyn iced::widget::container::StyleSheet<Style = Theme>)> {
        Box::new(NumberContainerStyle {
            _theme: theme.clone(),
        })
    }
}

impl<M: MessageBounds> Viewable for Mixer<M> {
    type ViewMessage = M;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = "MIXER";
        let contents = format!("sources: {}", 227);
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for Sampler {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<Sampler>();
        let contents = format!("name: {}", self.filename);
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for DrumkitSampler {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<DrumkitSampler>();
        let contents = format!("kit name: {}", self.kit_name);
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for WelshSynth {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<WelshSynth>();
        let contents = format!("name: {}", self.preset.name);
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl<M: MessageBounds> Viewable for Gain<M> {
    default type ViewMessage = M;

    default fn view(&self) -> Element<'_, Self::ViewMessage, iced::Renderer> {
        GuiStuff::titled_container(
            "Untitled",
            text("under construction")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center)
                .into(),
        )
    }
}
impl Viewable for Gain<EntityMessage> {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = format!("Gain: {}", self.ceiling()).to_string();
        let contents = container(row![HSlider::new(
            NormalParam {
                value: Normal::new(self.ceiling()),
                default: Normal::new(1.0)
            },
            Self::ViewMessage::HSliderInt
        )])
        .padding(20);
        GuiStuff::titled_container(&title, contents.into())
    }
}

impl Viewable for Bitcrusher {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<Bitcrusher>();
        let contents = format!("bits to crush: {}", self.bits_to_crush());
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for Limiter {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<Limiter>();
        let contents = format!("min: {} max: {}", self.min(), self.max());
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for BiQuadFilter<EntityMessage> {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<BiQuadFilter<Self::ViewMessage>>();
        let contents = row![
            container(slider(
                0..=100,
                (self.cutoff_pct() * 100.0) as u8,
                Self::ViewMessage::UpdateParam1U8 // CutoffPct
            ))
            .width(iced::Length::FillPortion(1)),
            container(GuiStuff::container_text(
                format!("cutoff: {}Hz", self.cutoff_hz()).as_str()
            ))
            .width(iced::Length::FillPortion(1))
        ];
        GuiStuff::titled_container(title, contents.into())
    }
}

impl Viewable for Delay {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = "dElAy";
        let contents = format!("delay in seconds: {}", self.delay_seconds());
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for Arpeggiator {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<Arpeggiator>();
        let contents = format!("cutoff: {}", "Foo TODO");
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl<M: MessageBounds> Viewable for BeatSequencer<M> {
    default type ViewMessage = M;

    default fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<BeatSequencer<M>>();
        let contents = format!("cursor point: {}", "tOdO");
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for BeatSequencer<EntityMessage> {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<BeatSequencer<EntityMessage>>();
        let contents = format!("cursor point: {}", "tOdO");
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

#[derive(Clone, Debug)]
pub enum PatternMessage {
    SomethingHappened,
    ButtonPressed,
}

impl Pattern<Note> {
    fn view<'a>(&self) -> Element<'a, PatternMessage> {
        let mut note_rows = Vec::new();
        for track in self.notes.iter() {
            let mut note_row = Vec::new();
            for note in track {
                let cell = text(format!("{:02} ", note.key).to_string());
                note_row.push(cell.into());
            }
            let row_note_row = row(note_row).into();
            note_rows.push(row_note_row);
        }
        column(vec![
            button(text(format!("{:?}", self.note_value)))
                .on_press(PatternMessage::ButtonPressed)
                .into(),
            column(note_rows).into(),
        ])
        .into()
    }
}

impl Viewable for PatternManager {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<PatternManager>();
        let contents = {
            let pattern_views = self.patterns().iter().enumerate().map(|(i, item)| {
                item.view()
                    .map(move |message| Self::ViewMessage::PatternMessage(i, message))
            });
            column(pattern_views.collect())
        };
        GuiStuff::titled_container(title, contents.into())
    }
}

impl<M: MessageBounds> Viewable for Orchestrator<M> {
    default type ViewMessage = M;

    default fn view(&self) -> Element<Self::ViewMessage> {
        container(text("not implemented")).into()
    }
}

impl Viewable for Orchestrator<GrooveMessage> {
    type ViewMessage = GrooveMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let views = self
            .store()
            .iter()
            .fold(Vec::new(), |mut v, (uid, e)| match e {
                crate::traits::BoxedEntity::Controller(entity) => {
                    v.push(
                        entity
                            .view()
                            .map(move |message| GrooveMessage::EntityMessage(*uid, message)),
                    );
                    v
                }
                crate::traits::BoxedEntity::Effect(entity) => {
                    v.push(
                        entity
                            .view()
                            .map(move |message| GrooveMessage::EntityMessage(*uid, message)),
                    );
                    v
                }
                crate::traits::BoxedEntity::Instrument(entity) => {
                    v.push(
                        entity
                            .view()
                            .map(move |message| GrooveMessage::EntityMessage(*uid, message)),
                    );
                    v
                }
            });
        //        let pattern_view = self.pattern_manager().view();
        column(views.into()).into()
    }
}

impl<M: MessageBounds> Viewable for ControlTrip<M> {
    type ViewMessage = M;
}
impl Viewable for Oscillator {
    type ViewMessage = EntityMessage;
}
impl<M: MessageBounds> Viewable for TestController<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for TestEffect<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for TestInstrument<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for Timer<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for Trigger<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for AudioSource<M> {
    type ViewMessage = M;
}
impl Viewable for MidiOutputHandler {
    type ViewMessage = EntityMessage;
}
impl Viewable for MidiHandler {
    type ViewMessage = EntityMessage;
}
impl<M: MessageBounds> Viewable for MidiTickSequencer<M> {
    type ViewMessage = M;
}
impl<M: MessageBounds> Viewable for BiQuadFilter<M> {
    default type ViewMessage = M;

    default fn view(&self) -> Element<'_, Self::ViewMessage, iced::Renderer> {
        GuiStuff::titled_container(
            "Untitled",
            text("under construction")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center)
                .into(),
        )
    }
}
impl Viewable for AdsrEnvelope {
    type ViewMessage = EntityMessage;
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    use iced::{
        widget::{container, text},
        Element,
    };

    use super::{GuiStuff, Viewable};
    use crate::utils::tests::{TestControlSourceContinuous, TestLfo, TestMixer, TestSynth};
    use crate::{
        controllers::sequencers::BeatSequencer,
        effects::{
            filter::{BiQuadFilter, FilterParams},
            gain::Gain,
        },
        messages::{tests::TestMessage, EntityMessage, MessageBounds},
    };

    impl<M: MessageBounds> Viewable for TestSynth<M> {
        type ViewMessage = M;
    }
    impl<M: MessageBounds> Viewable for TestMixer<M> {
        type ViewMessage = M;
    }
    impl<M: MessageBounds> Viewable for TestLfo<M> {
        type ViewMessage = M;
    }
    impl<M: MessageBounds> Viewable for TestControlSourceContinuous<M> {
        type ViewMessage = M;
    }
    impl Viewable for BiQuadFilter<TestMessage> {
        type ViewMessage = TestMessage;

        fn view(&self) -> Element<Self::ViewMessage> {
            container(text("not implemented")).into()
        }
    }
    impl Viewable for BeatSequencer<TestMessage> {
        type ViewMessage = TestMessage;

        fn view(&self) -> Element<Self::ViewMessage> {
            let title = type_name::<Self>();
            let contents = format!("cursor point: {}", "tOdO");
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
        }
    }

    // impl Viewable for PatternManager {
    //     type ViewMessage = GrooveMessage;

    //     fn view(&self) -> Element<Self::ViewMessage> {
    //         let title = type_name::<PatternManager>();
    //         let contents = {
    //             let pattern_views = self.patterns().iter().enumerate().map(|(i, item)| {
    //                 item.view()
    //                     .map(move |message| Self::ViewMessage::PatternMessage(i, message))
    //             });
    //             column(pattern_views.collect())
    //         };
    //         GuiStuff::titled_container(title, contents.into())
    //     }
    // }

    // There aren't many assertions in this method, but we know it'll panic or
    // spit out debug messages if something's wrong.
    fn test_one_viewable(
        viewable: Box<dyn Viewable<ViewMessage = EntityMessage>>,
        message: Option<EntityMessage>,
    ) {
        let _ = viewable.view();
        if let Some(_message) = message {
            //viewable.update(message);
        }
    }

    #[ignore]
    #[test]
    fn test_viewables_of_generic_entities() {
        // TODO: some of these commented-out entities could be made generic, but
        // it's a maintenance cost, and I don't know for sure if they're even
        // useful being able to respond to TestMessaage. I think I know how to genericize entities pretty well now, so it's not

        // test_one_viewable(
        //     Box::new(WelshSynth::new_with(
        //         44100,
        //         SynthPatch::by_name(&PatchName::Trombone),
        //     )),
        //     None,
        // );
        // test_one_viewable(Box::new(DrumkitSampler::new_from_files()), None);
        // test_one_viewable(Box::new(Sampler::new_with(1024)), None);
        // TODO - test it! test_one_viewable(Mixer::new_wrapped(), None);
        test_one_viewable(
            Box::new(Gain::<EntityMessage>::default()),
            Some(EntityMessage::UpdateParam0U8(28)),
        );
        // test_one_viewable(
        //     Box::new(Bitcrusher::new_with(7)),
        //     Some(GrooveMessage::BitcrusherValueChanged(4)),
        // );
        test_one_viewable(
            Box::new(BiQuadFilter::<EntityMessage>::new_with(
                &FilterParams::AllPass {
                    cutoff: 1000.0,
                    q: 2.0,
                },
                44100,
            )),
            Some(EntityMessage::UpdateParam1F32(500.0)),
        );
        // test_one_viewable(
        //     Box::new(Limiter::new_with(0.0, 1.0)),
        //     Some(GrooveMessage::LimiterMinChanged(0.5)),
        // );
        // test_one_viewable(
        //     Box::new(Arpeggiator::new_with(1)),
        //     Some(GrooveMessage::ArpeggiatorChanged(42)),
        // );
        test_one_viewable(
            Box::new(BeatSequencer::<EntityMessage>::default()),
            Some(EntityMessage::EnablePressed(false)),
        );
    }
}
