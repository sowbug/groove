use super::reverb::Reverb;
use super::{
    bitcrusher::Bitcrusher, delay::Delay, filter::BiQuadFilter, gain::Gain, limiter::Limiter,
    mixer::Mixer,
};
use crate::gui::{GuiStuff, Viewable};
use crate::messages::{EntityMessage, MessageBounds};
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{container, row, slider, text},
    Element,
};
use iced_audio::{HSlider, Normal, NormalParam};
use std::any::type_name;

impl Viewable for Bitcrusher {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = format!("Bitcrusher: {}", self.bits_to_crush());
        let contents = container(row![HSlider::new(
            self.int_range.normal_param(self.bits_to_crush() as i32, 8),
            Self::ViewMessage::HSliderInt
        )])
        .padding(20);
        GuiStuff::titled_container(&title, contents.into())
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
        let title = format!("Gain: {}", self.ceiling());
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

impl Viewable for Limiter {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<Limiter>();
        let contents = format!("min: {} max: {}", self.min(), self.max());
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for Reverb {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<Reverb>();
        let contents = format!("TODO");
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

impl<M: MessageBounds> Viewable for Mixer<M> {
    type ViewMessage = M;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = "MIXER";
        let contents = format!("sources: {}", 227);
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
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
