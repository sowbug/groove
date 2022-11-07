use crate::{
    common::{wrc_clone, Rrc, Ww},
    effects::{
        arpeggiator::Arpeggiator, bitcrusher::Bitcrusher, filter::BiQuadFilter, gain::Gain,
        limiter::Limiter, mixer::Mixer,
    },
    patterns::PatternSequencer,
    synthesizers::{drumkit_sampler::Sampler as DrumkitSampler, sampler::Sampler, welsh::Synth},
    traits::{HasEnable, HasMute, HasOverhead, MakesIsViewable, SinksAudio},
};
use iced::{
    alignment::{Horizontal, Vertical},
    theme,
    widget::{checkbox, column, container, row, slider, text, text_input},
    Color, Element, Font,
};
use std::{any::type_name, fmt::Debug};

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
}

#[derive(Default)]
pub struct GuiStuff {}

impl<'a> GuiStuff {
    pub fn titled_container(
        device: Option<Rrc<dyn HasOverhead>>,
        title: &str,
        contents: Element<'a, ViewableMessage>,
    ) -> Element<'a, ViewableMessage> {
        container(column![
            Self::titled_container_title(device, title),
            container(contents).padding(2)
        ])
        .padding(0)
        .style(theme::Container::Box)
        .into()
    }

    pub fn titled_container_title(
        device: Option<Rrc<dyn HasOverhead>>,
        title: &str,
    ) -> Element<'a, ViewableMessage> {
        let checkboxes = container(if let Some(device) = device {
            row![
                checkbox(
                    "Enabled".to_string(),
                    device.borrow().is_enabled(),
                    ViewableMessage::EnablePressed
                ),
                checkbox(
                    "Muted".to_string(),
                    device.borrow().is_muted(),
                    ViewableMessage::MutePressed
                )
            ]
        } else {
            row![text("".to_string())]
        });
        container(row![
            text(title.to_string())
                .font(SMALL_FONT)
                .size(SMALL_FONT_SIZE)
                .horizontal_alignment(iced::alignment::Horizontal::Left)
                .vertical_alignment(Vertical::Center),
            checkboxes
        ])
        .width(iced::Length::Fill)
        .padding(1)
        .style(theme::Container::Custom(Self::titled_container_title_style))
        .into()
    }

    pub fn container_text(label: &str) -> Element<'a, ViewableMessage> {
        text(label.to_string())
            .font(LARGE_FONT)
            .size(LARGE_FONT_SIZE)
            .horizontal_alignment(iced::alignment::Horizontal::Left)
            .vertical_alignment(Vertical::Center)
            .into()
    }

    fn titled_container_title_style(theme: &iced::Theme) -> container::Appearance {
        let palette = theme.extended_palette();
        container::Appearance {
            text_color: Some(palette.background.strong.text),
            background: Some(palette.background.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn number_box_style(_theme: &iced::Theme) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb8(255, 255, 0)),
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        }
    }
}

pub trait IsViewable: Debug {
    type Message;

    fn view(&self) -> Element<ViewableMessage> {
        GuiStuff::titled_container(
            None,
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

    fn update(&mut self, message: ViewableMessage) {
        dbg!(message);
    }
}

#[derive(Debug)]
pub struct MixerViewableResponder {
    target: Ww<Mixer>,
}
impl IsViewable for MixerViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Mixer>();
            let contents = format!("sources: {}", target.borrow().sources().len());
            GuiStuff::titled_container(
                Some(target),
                title,
                GuiStuff::container_text(contents.as_str()),
            )
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
                }
                _ => todo!(),
            };
        };
    }
}

impl MakesIsViewable for Mixer {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(MixerViewableResponder {
                target: wrc_clone(&self.me),
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
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Sampler>();
            let contents = format!("name: {}", target.borrow().filename);
            GuiStuff::titled_container(
                Some(target),
                title,
                GuiStuff::container_text(contents.as_str()),
            )
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
                }
                _ => todo!(),
            };
        };
    }
}

impl MakesIsViewable for Sampler {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(SamplerViewableResponder {
                target: wrc_clone(&self.me),
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
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<DrumkitSampler>();
            let contents = format!("kit name: {}", target.borrow().kit_name);
            GuiStuff::titled_container(
                Some(target),
                title,
                GuiStuff::container_text(contents.as_str()),
            )
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
                }
                _ => todo!(),
            };
        };
    }
}

impl MakesIsViewable for DrumkitSampler {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(DrumkitSamplerViewableResponder {
                target: wrc_clone(&self.me),
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
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Synth>();
            let contents = format!("name: {}", target.borrow().preset.name);
            GuiStuff::titled_container(
                Some(target),
                title,
                GuiStuff::container_text(contents.as_str()),
            )
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
                }
                _ => todo!(),
            };
        };
    }
}

impl MakesIsViewable for Synth {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(SynthViewableResponder {
                target: wrc_clone(&self.me),
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
}
impl GainViewableResponder {
    fn new(me: Ww<Gain>) -> Self {
        Self {
            target: wrc_clone(&me),
        }
    }
}
impl IsViewable for GainViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let level = target.borrow().ceiling();
            let level_percent: u8 = (level * 100.0) as u8;
            let title = "Gain";
            let contents = container(row![
                container(slider(
                    0..=100,
                    level_percent,
                    Self::Message::GainLevelChangedAsU8Percentage
                ))
                .width(iced::Length::FillPortion(1)),
                text_input(
                    "%",
                    level_percent.to_string().as_str(),
                    Self::Message::GainLevelChangedAsString,
                )
                .width(iced::Length::FillPortion(1)),
            ])
            .padding(20);
            GuiStuff::titled_container(Some(target), title, contents.into())
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
                }
                Self::Message::GainLevelChangedAsU8Percentage(ceiling) => {
                    // TODO: we need input sanitizers
                    // 0..=100
                    // 0.0..=1.0
                    // -1.0..=1.0
                    target.borrow_mut().set_ceiling((ceiling as f32) / 100.0);
                }
                Self::Message::GainLevelChangedAsString(ceiling) => {
                    if let Ok(ceiling) = ceiling.parse() {
                        target.borrow_mut().set_ceiling(ceiling);
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
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(GainViewableResponder::new(wrc_clone(&self.me))))
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
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Bitcrusher>();
            let contents = format!("bits to crush: {}", target.borrow().bits_to_crush());
            GuiStuff::titled_container(
                Some(target),
                title,
                GuiStuff::container_text(contents.as_str()),
            )
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
                }

                Self::Message::BitcrusherValueChanged(new_value) => {
                    target.borrow_mut().set_bits_to_crush(new_value);
                }
                _ => todo!(),
            }
        }
    }
}

impl MakesIsViewable for Bitcrusher {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(BitcrusherViewableResponder {
                target: wrc_clone(&self.me),
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
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Limiter>();
            let contents = format!(
                "min: {} max: {}",
                target.borrow().min(),
                target.borrow().max()
            );
            GuiStuff::titled_container(
                Some(target),
                title,
                GuiStuff::container_text(contents.as_str()),
            )
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
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
    }
}

impl MakesIsViewable for Limiter {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(LimiterViewableResponder {
                target: wrc_clone(&self.me),
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
    target: Ww<BiQuadFilter>,
}
impl IsViewable for FilterViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<BiQuadFilter>();
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
            GuiStuff::titled_container(Some(target), title, contents.into())
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
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
    }
}

impl MakesIsViewable for BiQuadFilter {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(FilterViewableResponder {
                target: wrc_clone(&self.me),
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
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<Arpeggiator>();
            let contents = format!("cutoff: {}", target.borrow().nothing());
            GuiStuff::titled_container(
                Some(target),
                title,
                GuiStuff::container_text(contents.as_str()),
            )
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
                }
                ViewableMessage::ArpeggiatorChanged(new_value) => {
                    target.borrow_mut().set_nothing(new_value as f32);
                }
                _ => todo!(),
            }
        }
    }
}

impl MakesIsViewable for Arpeggiator {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(ArpeggiatorViewableResponder {
                target: wrc_clone(&self.me),
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
pub struct PatternSequencerNewViewableResponder {
    target: Ww<PatternSequencer>,
}
impl IsViewable for PatternSequencerNewViewableResponder {
    type Message = ViewableMessage;

    fn view(&self) -> Element<Self::Message> {
        if let Some(target) = self.target.upgrade() {
            let title = type_name::<PatternSequencer>();
            let contents = format!("cursor point: {}", target.borrow().cursor());
            GuiStuff::titled_container(
                Some(target),
                title,
                GuiStuff::container_text(contents.as_str()),
            )
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: Self::Message) {
        if let Some(target) = self.target.upgrade() {
            match message {
                ViewableMessage::MutePressed(is_muted) => {
                    target.borrow_mut().set_muted(is_muted);
                }
                ViewableMessage::EnablePressed(is_enabled) => {
                    target.borrow_mut().set_enabled(is_enabled);
                }
                _ => todo!(),
            };
        };
    }
}

impl MakesIsViewable for PatternSequencer {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(PatternSequencerNewViewableResponder {
                target: wrc_clone(&self.me),
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
            arpeggiator::Arpeggiator,
            bitcrusher::Bitcrusher,
            filter::{BiQuadFilter, FilterParams},
            gain::Gain,
            limiter::Limiter,
            mixer::Mixer,
        },
        patterns::PatternSequencer,
        settings::patches::SynthPatch,
        synthesizers::{
            drumkit_sampler::Sampler as DrumkitSampler,
            sampler::Sampler,
            welsh::{PatchName, Synth},
        },
        traits::MakesIsViewable,
    };

    use super::ViewableMessage;

    // There aren't many assertions in this method, but we know it'll panic or spit out debug
    // messages if something's wrong.
    fn test_one_viewable(factory: Rrc<dyn MakesIsViewable>, message: Option<ViewableMessage>) {
        let is_viewable = factory.borrow_mut().make_is_viewable();
        if let Some(mut viewable) = is_viewable {
            let _ = viewable.view();
            if let Some(message) = message {
                viewable.update(message);
            }
        } else {
            panic!("factory failed {factory:?}");
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
            Some(ViewableMessage::GainLevelChangedAsU8Percentage(28)),
        );
        test_one_viewable(
            Bitcrusher::new_wrapped_with(7),
            Some(ViewableMessage::BitcrusherValueChanged(4)),
        );
        test_one_viewable(
            BiQuadFilter::new_wrapped_with(
                &FilterParams::AllPass {
                    cutoff: 1000.0,
                    q: 2.0,
                },
                44100,
            ),
            Some(ViewableMessage::FilterCutoffChangedAsF32(500.0)),
        );
        test_one_viewable(
            Limiter::new_wrapped_with(0.0, 1.0),
            Some(ViewableMessage::LimiterMinChanged(0.5)),
        );
        test_one_viewable(
            Arpeggiator::new_wrapped_with(0, 1),
            Some(ViewableMessage::ArpeggiatorChanged(42)),
        );
        test_one_viewable(
            PatternSequencer::new_wrapped_with(&crate::TimeSignature::default()),
            Some(ViewableMessage::EnablePressed(false)),
        );
    }
}
