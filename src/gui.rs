use std::{any::type_name, fmt::Debug, rc::Weak};

use iced::text_input::State;
use iced::{container, Column, Container, Element, Font, Text};
use iced::{Row, TextInput};
use iced_audio::knob;
use iced_audio::text_marks;
use iced_audio::tick_marks;
use iced_audio::FloatRange;
use iced_audio::Knob;

use crate::{
    common::Ww,
    effects::{
        arpeggiator::Arpeggiator, bitcrusher::Bitcrusher, filter::Filter, gain::Gain,
        limiter::Limiter, mixer::Mixer,
    },
    synthesizers::{drumkit_sampler::Sampler as DrumkitSampler, sampler::Sampler, welsh::Synth},
    traits::{MakesIsViewable, SinksAudio},
};

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

#[derive(Clone, Debug)]
pub enum GrooveMessage {
    Null,
    Something,
    GainMessage(GainMessage), // TODO: this might be too specific
    GainLevelChangedAsNormal(iced_audio::Normal),
    GainLevelChangedAsString(String),
}

#[derive(Default)]
pub struct BorderedContainer {}

impl container::StyleSheet for BorderedContainer {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb8(
                224, 224, 224,
            ))),
            text_color: Some(iced::Color::from_rgb8(0, 0, 0)),
            border_color: iced::Color::BLACK,
            border_width: 1.0,
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct TitleContainer {}

impl container::StyleSheet for TitleContainer {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(iced::Background::Color(iced::Color::BLACK)),
            text_color: Some(iced::Color::from_rgb8(255, 255, 0)),
            border_color: iced::Color::BLACK,
            border_width: 1.0,
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct GuiStuff {}

impl<'a> GuiStuff {
    pub fn titled_container(
        title: &str,
        contents: Element<'a, GrooveMessage>,
    ) -> Element<'a, GrooveMessage> {
        Container::new(
            Column::new()
                .push(Self::titled_container_title(title))
                .push(Container::new(contents).padding(2)),
        )
        .padding(0)
        .style(BorderedContainer::default())
        .into()
    }

    pub fn titled_container_title(title: &str) -> Container<'a, GrooveMessage> {
        Container::new(
            Text::new(title.to_string())
                .font(SMALL_FONT)
                .size(SMALL_FONT_SIZE)
                .horizontal_alignment(iced::alignment::Horizontal::Left)
                .vertical_alignment(iced::alignment::Vertical::Center),
        )
        .width(iced::Length::Fill)
        .padding(1)
        .style(TitleContainer::default())
    }

    pub fn container_text(label: &str) -> Text {
        Text::new(label.to_string())
            .font(LARGE_FONT)
            .size(LARGE_FONT_SIZE)
            .horizontal_alignment(iced::alignment::Horizontal::Left)
            .vertical_alignment(iced::alignment::Vertical::Center)
    }
}

pub trait IsViewable: Debug {
    fn view(&mut self) -> Element<GrooveMessage> {
        Container::new(
            Text::new(format!("{}: under construction", self.name()))
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .vertical_alignment(iced::alignment::Vertical::Center),
        )
        .padding(4)
        .style(BorderedContainer::default())
        .into()
    }

    fn name(&mut self) -> String {
        type_name::<Self>().to_string()
    }

    fn update(&mut self, message: GrooveMessage) {
        dbg!(message);
    }
}

#[derive(Debug)]
pub struct MixerViewableResponder {
    target: Ww<Mixer>,
}
impl IsViewable for MixerViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Mixer>();
            let contents = format!("sources: {}", target.borrow().sources().len());
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()).into())
                .into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        dbg!(message);
    }
}

impl MakesIsViewable for Mixer {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(MixerViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct SamplerViewableResponder {
    target: Ww<Sampler>,
}
impl IsViewable for SamplerViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Sampler>();
            let contents = format!("name: {}", target.borrow().filename);
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()).into())
                .into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        dbg!(message);
    }
}

impl MakesIsViewable for Sampler {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(SamplerViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct DrumkitSamplerViewableResponder {
    target: Ww<DrumkitSampler>,
}
impl IsViewable for DrumkitSamplerViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<DrumkitSampler>();
            let contents = format!("kit name: {}", target.borrow().kit_name);
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()).into())
                .into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        dbg!(message);
    }
}

impl MakesIsViewable for DrumkitSampler {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(DrumkitSamplerViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct SynthViewableResponder {
    target: Ww<Synth>,
}
impl IsViewable for SynthViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Synth>();
            let contents = format!("name: {}", target.borrow().preset.name);
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()).into())
                .into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        dbg!(message);
    }
}

impl MakesIsViewable for Synth {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(SynthViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct GainViewableResponder {
    target: Ww<Gain>,
    knob_state: knob::State,
    text_input_state: State,
    tick_marks: tick_marks::Group,
    text_marks: text_marks::Group,
}
impl GainViewableResponder {
    fn new(me: Ww<Gain>) -> Self {
        Self {
            target: Weak::clone(&me),
            knob_state: knob::State::new(FloatRange::new(0.0, 1.0).normal_param(1.0, 1.0)),
            text_input_state: State::new(),
            tick_marks: tick_marks::Group::subdivided(1, 1, 1, Some(tick_marks::Tier::Two)),
            text_marks: text_marks::Group::min_max_and_center("0%", "100%", "50%"),
        }
    }
}
impl IsViewable for GainViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            let level = target.borrow().level();
            self.knob_state.normal_param.value = level.into();
            let title = "Gain";
            let contents = Container::new(
                Row::new()
                    .push(
                        Container::new(
                            Knob::new(
                                &mut self.knob_state,
                                GrooveMessage::GainLevelChangedAsNormal,
                                || None,
                                || None,
                            )
                            .tick_marks(&self.tick_marks)
                            .text_marks(&self.text_marks),
                        )
                        .width(iced::Length::FillPortion(1)),
                    )
                    .push(
                        TextInput::new(
                            &mut self.text_input_state,
                            "%",
                            level.to_string().as_str(),
                            GrooveMessage::GainLevelChangedAsString,
                        )
                        .width(iced::Length::FillPortion(1)),
                    ),
            )
            .padding(20);
            GuiStuff::titled_container(title, contents.into()).into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        if let Some(target) = self.target.upgrade() {
            match message {
                GrooveMessage::GainMessage(message) => match message {
                    GainMessage::Level(level) => {
                        if let Ok(level) = level.parse() {
                            target.borrow_mut().set_level(level);
                        }
                    }
                },
                GrooveMessage::GainLevelChangedAsNormal(new_level) => {
                    target.borrow_mut().set_level(new_level.as_f32());
                }
                GrooveMessage::GainLevelChangedAsString(new_level) => {
                    if let Ok(level) = new_level.parse() {
                        target.borrow_mut().set_level(level);
                    }
                }
                _ => todo!(),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum GainMessage {
    Level(String),
}
impl MakesIsViewable for Gain {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(GainViewableResponder::new(Weak::clone(&self.me))))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct BitcrusherViewableResponder {
    target: Ww<Bitcrusher>,
}
impl IsViewable for BitcrusherViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Bitcrusher>();
            let contents = format!("bits to crush: {}", target.borrow().bits_to_crush());
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()).into())
                .into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        match message {
            GrooveMessage::GainMessage(message) => match message {
                GainMessage::Level(level) => {
                    if let Some(target) = self.target.upgrade() {
                        if let Ok(level) = level.parse() {
                            target.borrow_mut().set_bits_to_crush(level);
                        }
                    }
                }
            },
            _ => {}
        }
    }
}

impl MakesIsViewable for Bitcrusher {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(BitcrusherViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct LimiterViewableResponder {
    target: Ww<Limiter>,
}
impl IsViewable for LimiterViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Limiter>();
            let contents = format!(
                "min: {} max: {}",
                target.borrow().min(),
                target.borrow().max()
            );
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()).into())
                .into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        match message {
            GrooveMessage::GainMessage(message) => match message {
                GainMessage::Level(level) => {
                    if let Some(target) = self.target.upgrade() {
                        if let Ok(level) = level.parse() {
                            target.borrow_mut().set_min(level);
                        }
                    }
                }
            },
            _ => {}
        }
    }
}

impl MakesIsViewable for Limiter {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(LimiterViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct FilterViewableResponder {
    target: Ww<Filter>,
}
impl IsViewable for FilterViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Filter>();
            let contents = format!("cutoff: {}", target.borrow().cutoff());
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()).into())
                .into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        match message {
            GrooveMessage::GainMessage(message) => match message {
                GainMessage::Level(level) => {
                    if let Some(target) = self.target.upgrade() {
                        if let Ok(level) = level.parse() {
                            target.borrow_mut().set_cutoff(level);
                        }
                    }
                }
            },
            _ => {}
        }
    }
}

impl MakesIsViewable for Filter {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(FilterViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct ArpeggiatorViewableResponder {
    target: Ww<Arpeggiator>,
}
impl IsViewable for ArpeggiatorViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Arpeggiator>();
            let contents = format!("cutoff: {}", target.borrow().nothing());
            GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()).into())
                .into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        match message {
            GrooveMessage::GainMessage(message) => match message {
                GainMessage::Level(level) => {
                    if let Some(target) = self.target.upgrade() {
                        if let Ok(level) = level.parse() {
                            target.borrow_mut().set_nothing(level);
                        }
                    }
                }
            },
            _ => {}
        }
    }
}

impl MakesIsViewable for Arpeggiator {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(ArpeggiatorViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::Rrc,
        effects::{
            arpeggiator::Arpeggiator, bitcrusher::Bitcrusher, filter::Filter, gain::Gain,
            limiter::Limiter, mixer::Mixer,
        },
        settings::patches::SynthPatch,
        synthesizers::{
            drumkit_sampler::Sampler as DrumkitSampler,
            sampler::Sampler,
            welsh::{PatchName, Synth},
        },
        traits::MakesIsViewable,
    };

    use super::GrooveMessage;

    // There aren't many assertions in this method, but we know it'll panic or spit out debug
    // messages if something's wrong.
    fn test_one_viewable(factory: Rrc<dyn MakesIsViewable>, message: Option<GrooveMessage>) {
        let is_viewable = factory.borrow_mut().make_is_viewable();
        if let Some(mut viewable) = is_viewable {
            let _ = viewable.view();
            if let Some(message) = message {
                viewable.update(message);
            }
        } else {
            assert!(false, "factory failed {:?}", factory);
        }
    }

    #[test]
    fn test_viewables() {
        test_one_viewable(
            Synth::new_wrapped_with(0, 44100, SynthPatch::by_name(&PatchName::Trombone)),
            None,
        );
        test_one_viewable(DrumkitSampler::new_wrapped_from_files(0), None);
        test_one_viewable(Sampler::new_wrapped_with(0, 1024), None);
        test_one_viewable(Mixer::new_wrapped(), None);
        test_one_viewable(
            Gain::new_wrapped(),
            Some(GrooveMessage::GainMessage(super::GainMessage::Level(
                "0.5".to_string(),
            ))),
        );
        test_one_viewable(
            Bitcrusher::new_wrapped_with(7),
            Some(GrooveMessage::GainMessage(super::GainMessage::Level(
                "4".to_string(),
            ))), // TODO: better messages
        );
        test_one_viewable(
            Filter::new_wrapped_with(&crate::effects::filter::FilterType::AllPass {
                sample_rate: 44100,
                cutoff: 1000.0,
                q: 2.0,
            }),
            Some(GrooveMessage::GainMessage(super::GainMessage::Level(
                "0.5".to_string(),
            ))), // TODO: better messages
        );
        test_one_viewable(
            Limiter::new_wrapped_with(0.0, 1.0),
            Some(GrooveMessage::GainMessage(super::GainMessage::Level(
                "0.5".to_string(),
            ))), // TODO: better messages
        );
        test_one_viewable(
            Arpeggiator::new_wrapped_with(0, 1),
            Some(GrooveMessage::GainMessage(super::GainMessage::Level(
                "0.5".to_string(),
            ))), // TODO: better messages
        );
    }
}
