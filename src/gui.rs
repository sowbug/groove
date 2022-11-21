use crate::{
    common::Ww,
    effects::{
        arpeggiator::Arpeggiator, bitcrusher::Bitcrusher, filter::BiQuadFilter, gain::Gain,
        limiter::Limiter, mixer::Mixer,
    },
    instruments::{drumkit_sampler::Sampler as DrumkitSampler, sampler::Sampler, welsh::Synth},
    messages::GrooveMessage,
    midi::{
        patterns::{Note, Pattern, PatternManager},
        sequencers::BeatSequencer,
    },
    traits::{MakesIsViewable, MessageBounds},
    GrooveOrchestrator,
};
use iced::{
    alignment::{Horizontal, Vertical},
    theme,
    widget::{button, column, container, row, slider, text},
    Color, Command, Element, Font, Theme,
};
use std::{any::type_name, fmt::Debug, marker::PhantomData};

pub const SMALL_FONT_SIZE: u16 = 16;
pub const SMALL_FONT: Font = Font::External {
    name: "Small Font",
    bytes: include_bytes!("../resources/fonts/SourceSansPro-Regular.ttf"),
};

pub const LARGE_FONT_SIZE: u16 = 20;
pub const LARGE_FONT: Font = Font::External {
    name: "Large Font",
    bytes: include_bytes!("../resources/fonts/SourceSansPro-Regular.ttf"),
};

pub const NUMBERS_FONT_SIZE: u16 = 32;
pub const NUMBERS_FONT: Font = Font::External {
    name: "Numbers Font",
    bytes: include_bytes!("../resources/fonts/NotoSansMono-Regular.ttf"),
};

#[derive(Clone, Debug)]
pub enum ViewableMessage {
    MutePressed(bool),
    EnablePressed(bool),
    ArpeggiatorChanged(u8),
    BitcrusherValueChanged(u8),
    FilterCutoffChangedAsF32(f32),
    FilterCutoffChangedAsU8Percentage(u8),
    GainLevelChangedAsString(String),
    GainLevelChangedAsU8Percentage(u8),
    LimiterMinChanged(f32),
    LimiterMaxChanged(f32),
    PatternMessage(usize, PatternMessage),
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

    fn missing_target_container() -> Element<'a, Message> {
        container(text("missing target!")).into()
    }
}

pub trait IsViewable: Debug {
    type Message;

    fn view(&self) -> Element<'_, Self::Message, iced::Renderer> {
        GuiStuff::titled_container(
            "Untitled",
            text("under construction")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center)
                .into(),
        )
    }

    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }

    #[allow(unused_variables)]
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }
}

impl<M: MessageBounds> IsViewable for Mixer<M> {
    type Message = ViewableMessage;

    fn view(&self) -> Element<ViewableMessage> {
        let title = "MIXER";
        let contents = format!("sources: {}", 227);
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            ViewableMessage::MutePressed(is_muted) => {
                //                self.set_muted(is_muted);
            }
            ViewableMessage::EnablePressed(is_enabled) => {
                //             self.set_enabled(is_enabled);
            }
            _ => todo!(),
        };
        Command::none()
    }
}

impl<M: MessageBounds> MakesIsViewable for Mixer<M> {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
    }
}

#[derive(Debug)]
pub struct SamplerViewableResponder {
    target: Ww<Sampler>,
}
impl IsViewable for SamplerViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Sampler>();
            let contents = format!("name: {}", target.borrow().filename);
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
        } else {
            GuiStuff::missing_target_container()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    // target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    //    target.borrow_mut().set_enabled(is_enabled);
                }
                _ => todo!(),
            };
        };
        Command::none()
    }
}

impl MakesIsViewable for Sampler {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
    }
}

#[derive(Debug)]
pub struct DrumkitSamplerViewableResponder {
    target: Ww<DrumkitSampler>,
}
impl IsViewable for DrumkitSamplerViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<DrumkitSampler>();
            let contents = format!("kit name: {}", target.borrow().kit_name);
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
        } else {
            GuiStuff::missing_target_container()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    //      target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    //    target.borrow_mut().set_enabled(is_enabled);
                }
                _ => todo!(),
            };
        };
        Command::none()
    }
}

impl MakesIsViewable for DrumkitSampler {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
    }
}

#[derive(Debug)]
pub struct SynthViewableResponder {
    target: Ww<Synth>,
}
impl IsViewable for SynthViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Synth>();
            let contents = format!("name: {}", target.borrow().preset.name);
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
        } else {
            GuiStuff::missing_target_container()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    //     target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    //   target.borrow_mut().set_enabled(is_enabled);
                }
                _ => todo!(),
            };
        };
        Command::none()
    }
}

impl MakesIsViewable for Synth {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
    }
}

#[derive(Debug)]
pub struct GainViewableResponder {}
impl GainViewableResponder {
    fn new(me: Ww<Gain>) -> Self {
        Self {}
    }
}
impl IsViewable for GainViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        // if let Some(target) = self.target.upgrade() {
        //     let level = target.borrow().ceiling();
        //     let level_percent: u8 = (level * 100.0) as u8;
        //     let title = "Gain";
        //     let contents = container(row![
        //         container(slider(
        //             0..=100,
        //             level_percent,
        //             Self::Message::GainLevelChangedAsU8Percentage
        //         ))
        //         .width(iced::Length::FillPortion(1)),
        //         text_input(
        //             "%",
        //             level_percent.to_string().as_str(),
        //             Self::Message::GainLevelChangedAsString,
        //         )
        //         .width(iced::Length::FillPortion(1)),
        //     ])
        //     .padding(20);
        //     GuiStuff::titled_container(title, contents.into())
        // } else {
        GuiStuff::missing_target_container()
        // }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        // if let Some(target) = self.target.upgrade() {
        //     match message {
        //         ViewableMessage::MutePressed(is_muted) => {
        //             //    target.borrow_mut().set_muted(is_muted);
        //         }
        //         ViewableMessage::EnablePressed(is_enabled) => {
        //             //  target.borrow_mut().set_enabled(is_enabled);
        //         }
        //         Self::Message::GainLevelChangedAsU8Percentage(ceiling) => {
        //             // TODO: we need input sanitizers 0..=100 0.0..=1.0
        //             // -1.0..=1.0
        //             target.borrow_mut().set_ceiling((ceiling as f32) / 100.0);
        //         }
        //         Self::Message::GainLevelChangedAsString(ceiling) => {
        //             if let Ok(ceiling) = ceiling.parse() {
        //                 target.borrow_mut().set_ceiling(ceiling);
        //             }
        //         }
        //         _ => todo!(),
        //     }
        // }
        Command::none()
    }
}

#[derive(Clone, Debug)]
pub enum GainMessage {
    Level(String),
}
impl MakesIsViewable for Gain {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
    }
}

#[derive(Debug)]
pub struct BitcrusherViewableResponder {
    target: Ww<Bitcrusher>,
}
impl IsViewable for BitcrusherViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Bitcrusher>();
            let contents = format!("bits to crush: {}", target.borrow().bits_to_crush());
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
        } else {
            GuiStuff::missing_target_container()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    //    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    //      target.borrow_mut().set_enabled(is_enabled);
                }

                Self::Message::BitcrusherValueChanged(new_value) => {
                    target.borrow_mut().set_bits_to_crush(new_value);
                }
                _ => todo!(),
            }
        }
        Command::none()
    }
}

impl MakesIsViewable for Bitcrusher {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
    }
}

#[derive(Debug)]
pub struct LimiterViewableResponder {
    target: Ww<Limiter>,
}
impl IsViewable for LimiterViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Limiter>();
            let contents = format!(
                "min: {} max: {}",
                target.borrow().min(),
                target.borrow().max()
            );
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
        } else {
            GuiStuff::missing_target_container()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    //      target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    //        target.borrow_mut().set_enabled(is_enabled);
                }
                ViewableMessage::LimiterMinChanged(new_value) => {
                    if let Some(target) = self.target.upgrade() {
                        target.borrow_mut().set_min(new_value);
                    }
                }
                ViewableMessage::LimiterMaxChanged(new_value) => {
                    target.borrow_mut().set_max(new_value);
                }
                _ => todo!(),
            }
        }
        Command::none()
    }
}

impl MakesIsViewable for Limiter {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
    }
}

#[derive(Debug)]
pub struct FilterViewableResponder {
    target: Ww<BiQuadFilter<GrooveMessage>>,
}
impl IsViewable for FilterViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<BiQuadFilter<GrooveMessage>>();
            let contents = row![
                container(slider(
                    0..=100,
                    (target.borrow().cutoff_pct() * 100.0) as u8,
                    Self::Message::FilterCutoffChangedAsU8Percentage
                ))
                .width(iced::Length::FillPortion(1)),
                container(GuiStuff::container_text(
                    format!("cutoff: {}Hz", target.borrow().cutoff_hz()).as_str()
                ))
                .width(iced::Length::FillPortion(1))
            ];
            GuiStuff::titled_container(title, contents.into())
        } else {
            GuiStuff::missing_target_container()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    //           target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    //          target.borrow_mut().set_enabled(is_enabled);
                }
                ViewableMessage::FilterCutoffChangedAsF32(new_value) => {
                    if let Some(target) = self.target.upgrade() {
                        target.borrow_mut().set_cutoff_hz(new_value);
                    }
                }
                ViewableMessage::FilterCutoffChangedAsU8Percentage(new_value) => {
                    target
                        .borrow_mut()
                        .set_cutoff_pct((new_value as f32) / 100.0);
                }
                _ => todo!(),
            }
        }
        Command::none()
    }
}

impl MakesIsViewable for BiQuadFilter<GrooveMessage> {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
    }
}

#[derive(Debug)]
pub struct ArpeggiatorViewableResponder {
    target: Ww<Arpeggiator>,
}
impl IsViewable for ArpeggiatorViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Arpeggiator>();
            let contents = format!("cutoff: {}", "Foo TODO");
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
        } else {
            GuiStuff::missing_target_container()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    //            target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    //          target.borrow_mut().set_enabled(is_enabled);
                }
                ViewableMessage::ArpeggiatorChanged(new_value) => {
//                    target.borrow_mut().set_nothing(new_value as f32);
                }
                _ => todo!(),
            }
        }
        Command::none()
    }
}

impl MakesIsViewable for Arpeggiator {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
    }
}

#[derive(Debug)]
pub struct BeatSequencerViewableResponder<M: MessageBounds> {
    target: Ww<BeatSequencer<M>>,
}
impl<M: MessageBounds> IsViewable for BeatSequencerViewableResponder<M> {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<BeatSequencer<GrooveMessage>>();
            let contents = format!("cursor point: {}", "tOdO");
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
        } else {
            GuiStuff::missing_target_container()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    //          target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    //          target.borrow_mut().set_enabled(is_enabled);
                }
                _ => todo!(),
            };
        };
        Command::none()
    }
}

impl<M: MessageBounds> MakesIsViewable for BeatSequencer<M> {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        None
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

    fn update(&mut self, message: PatternMessage) {
        match message {
            _ => {
                dbg!(&message);
            }
        }
    }
}

impl IsViewable for PatternManager {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        let title = type_name::<PatternManager>();
        let contents = {
            let pattern_views = self.patterns().iter().enumerate().map(|(i, item)| {
                item.view()
                    .map(move |message| ViewableMessage::PatternMessage(i, message))
            });
            column(pattern_views.collect())
        };
        GuiStuff::titled_container(title, contents.into())
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            ViewableMessage::PatternMessage(i, message) => {
                self.patterns_mut()[i].update(message);
            }
            _ => {
                dbg!(&message);
            }
        }
        Command::none()
    }
}

impl IsViewable for GrooveOrchestrator {
    type Message = ViewableMessage;

    fn view(&self) -> Element<ViewableMessage> {
        column(vec![self.pattern_manager().view()].into()).into()
    }

    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        dbg!(&message);
        Command::none()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        effects::{
            arpeggiator::Arpeggiator,
            bitcrusher::Bitcrusher,
            filter::{BiQuadFilter, FilterParams},
            gain::Gain,
            limiter::Limiter,
        },
        instruments::{
            drumkit_sampler::Sampler as DrumkitSampler,
            sampler::Sampler,
            welsh::{PatchName, Synth},
        },
        messages::tests::TestMessage,
        midi::sequencers::BeatSequencer,
        settings::patches::SynthPatch,
        traits::MakesIsViewable,
    };

    use super::ViewableMessage;

    // There aren't many assertions in this method, but we know it'll panic or
    // spit out debug messages if something's wrong.
    fn test_one_viewable(factory: Box<dyn MakesIsViewable>, message: Option<ViewableMessage>) {
        let is_viewable = factory.make_is_viewable();
        if let Some(mut viewable) = is_viewable {
            let _ = viewable.view();
            if let Some(message) = message {
                viewable.update(message);
            }
        } else {
            panic!("factory failed {factory:?}");
        }
    }

    #[ignore]
    #[test]
    fn test_viewables() {
        test_one_viewable(
            Box::new(Synth::new_with(
                0,
                44100,
                SynthPatch::by_name(&PatchName::Trombone),
            )),
            None,
        );
        Box::new(test_one_viewable(
            Box::new(DrumkitSampler::new_from_files(0)),
            None,
        ));
        Box::new(test_one_viewable(
            Box::new(Sampler::new_with(0, 1024)),
            None,
        ));
        // TODO - test it! test_one_viewable(Mixer::new_wrapped(), None);
        test_one_viewable(
            Box::new(Gain::new()),
            Some(ViewableMessage::GainLevelChangedAsU8Percentage(28)),
        );
        test_one_viewable(
            Box::new(Bitcrusher::new_with(7)),
            Some(ViewableMessage::BitcrusherValueChanged(4)),
        );
        test_one_viewable(
            Box::new(BiQuadFilter::new_with(
                &FilterParams::AllPass {
                    cutoff: 1000.0,
                    q: 2.0,
                },
                44100,
            )),
            Some(ViewableMessage::FilterCutoffChangedAsF32(500.0)),
        );
        test_one_viewable(
            Box::new(Limiter::new_with(0.0, 1.0)),
            Some(ViewableMessage::LimiterMinChanged(0.5)),
        );
        test_one_viewable(
            Box::new(Arpeggiator::new_with(0, 1)),
            Some(ViewableMessage::ArpeggiatorChanged(42)),
        );
        test_one_viewable(
            Box::new(BeatSequencer::<TestMessage>::new()),
            Some(ViewableMessage::EnablePressed(false)),
        );
    }
}
