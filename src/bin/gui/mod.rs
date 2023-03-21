// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub(crate) mod persistence;
pub(crate) mod views;

use iced::{
    alignment::{self, Vertical},
    theme::{self, palette},
    widget::{self, button, checkbox, column, container, row, svg, text, Button, Text},
    Color, Element, Font, Length, Renderer, Theme,
};
use iced_native::{svg::Handle, widget::Svg};
use std::marker::PhantomData;

pub const SMALL_FONT_SIZE: u16 = 16;
pub const SMALL_FONT: Font = Font::External {
    name: "Small Font",
    bytes: include_bytes!("../../../res/fonts/heebo/static/Heebo-Regular.ttf"),
};

pub const LARGE_FONT_SIZE: u16 = 20;
pub const LARGE_FONT: Font = Font::External {
    name: "Large Font",
    bytes: include_bytes!("../../../res/fonts/heebo/static/Heebo-Regular.ttf"),
};

pub const NUMBERS_FONT_SIZE: u16 = 24;
pub const NUMBERS_FONT: Font = Font::External {
    name: "Numbers Font",
    bytes: include_bytes!("../../../res/fonts/noto-sans-mono/NotoSansMono-Regular.ttf"),
};

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

pub(crate) struct NumberContainerStyle {
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

pub(crate) struct CollapsingBoxStyle {
    _theme: iced::Theme,
    enabled: bool,
}

impl container::StyleSheet for CollapsingBoxStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let palette = &palette::EXTENDED_DARK;

        if self.enabled {
            container::Appearance {
                text_color: None,
                background: palette.background.weak.color.into(),
                border_radius: 2.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        } else {
            container::Appearance {
                text_color: palette.secondary.weak.color.into(),
                background: palette.background.weak.color.into(),
                border_radius: 2.0,
                border_width: 0.0,
                border_color: palette.secondary.weak.color.into(),
            }
        }
    }
}

#[derive(Default)]
pub struct GuiStuff<'a, Message> {
    phantom: PhantomData<&'a Message>,
}

impl<'a, Message: 'a + Clone> GuiStuff<'a, Message> {
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
        container(row![text(title.to_string())
            .font(SMALL_FONT)
            .size(SMALL_FONT_SIZE)
            .horizontal_alignment(iced::alignment::Horizontal::Left)
            .vertical_alignment(Vertical::Center),])
        .width(iced::Length::Fill)
        .padding(1)
        .style(theme::Container::Custom(
            Self::titled_container_title_style(&Theme::Dark),
        ))
        .into()
    }

    pub fn container_text(label: &str) -> widget::Text<'a, Renderer> {
        text(label.to_string())
            .font(LARGE_FONT)
            .size(LARGE_FONT_SIZE)
            .horizontal_alignment(iced::alignment::Horizontal::Left)
            .vertical_alignment(Vertical::Center)
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

    fn container_title_bar(
        title: &str,
        is_expanded: bool,
        is_enabled: bool,
        on_expand: Message,
        on_enable: impl Fn(bool) -> Message + 'a,
    ) -> Element<'a, Message> {
        let disclosure = button(
            if is_expanded {
                expand_less_icon()
            } else {
                expand_more_icon()
            }
            .size(8),
        )
        .on_press(on_expand);
        let enable_checkbox = checkbox("", is_enabled, on_enable);

        let title_text = container(
            text(title.to_string())
                .font(SMALL_FONT)
                .size(SMALL_FONT_SIZE)
                .horizontal_alignment(iced::alignment::Horizontal::Left)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fill);

        container(row![title_text, enable_checkbox, disclosure,])
            .width(iced::Length::Fill)
            .padding(1)
            .style(theme::Container::Custom(
                Self::titled_container_title_style(&Theme::Dark),
            ))
            .into()
    }

    pub fn collapsing_box_style(
        theme: &iced::Theme,
        enabled: bool,
    ) -> Box<(dyn iced::widget::container::StyleSheet<Style = Theme>)> {
        Box::new(CollapsingBoxStyle {
            _theme: theme.clone(),
            enabled,
        })
    }

    pub fn collapsed_container(
        title: &str,
        on_expand: Message,
        on_enable: impl Fn(bool) -> Message + 'a,
        enabled: bool,
    ) -> Element<'a, Message> {
        container(Self::container_title_bar(
            title, false, enabled, on_expand, on_enable,
        ))
        .width(iced::Length::Fill)
        .padding(0)
        .style(theme::Container::Custom(Self::collapsing_box_style(
            &Theme::Dark,
            enabled,
        )))
        .into()
    }

    pub fn expanded_container(
        title: &str,
        on_expand: Message,
        on_enable: impl Fn(bool) -> Message + 'a,
        enabled: bool,
        contents: Element<'a, Message>,
    ) -> Element<'a, Message> {
        container(column![
            Self::container_title_bar(title, true, enabled, on_expand, on_enable),
            contents,
        ])
        .width(iced::Length::Fill)
        .padding(0)
        .style(theme::Container::Custom(Self::collapsing_box_style(
            &Theme::Dark,
            enabled,
        )))
        .into()
    }
}

const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../../../res/fonts/material-icons/MaterialIcons-Regular.ttf"),
};

fn icon<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .font(ICONS)
        .width(Length::Fixed(20.0))
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}

pub(crate) enum IconType {
    Start,
    Play,
    Pause,
    Stop,
    OpenProject,
    ExportWav,
    ExportMp3,
}

pub(crate) struct Icons;
impl Icons {
    pub const SIZE: f32 = 40.0;

    pub fn button_icon<T>(icon_type: IconType) -> Button<'static, T> {
        button(Self::icon(icon_type))
            .width(Length::Fixed(Self::SIZE))
            .height(Length::Fixed(Self::SIZE))
    }

    pub fn icon(icon_type: IconType) -> Svg<Renderer> {
        let resource_name: &[u8] = match icon_type {
            IconType::Start => {
                include_bytes!("../../../res/bootstrap-icons-1.10.3/skip-start-fill.svg")
            }
            IconType::Play => include_bytes!("../../../res/bootstrap-icons-1.10.3/play-fill.svg"),
            IconType::Pause => include_bytes!("../../../res/bootstrap-icons-1.10.3/pause-fill.svg"),
            IconType::Stop => include_bytes!("../../../res/bootstrap-icons-1.10.3/stop-fill.svg"),
            IconType::OpenProject => {
                include_bytes!("../../../res/bootstrap-icons-1.10.3/folder2-open.svg")
            }
            IconType::ExportWav => {
                include_bytes!("../../../res/bootstrap-icons-1.10.3/filetype-wav.svg")
            }
            IconType::ExportMp3 => {
                include_bytes!("../../../res/bootstrap-icons-1.10.3/filetype-mp3.svg")
            }
        };
        Self::styled_svg_from_memory(resource_name)
    }

    fn styled_svg_from_memory(bytes: &'static [u8]) -> Svg<Renderer> {
        Svg::new(Handle::from_memory(bytes)).style(theme::Svg::custom_fn(|_theme| {
            svg::Appearance {
                color: Some(Color::WHITE),
            }
        }))
    }
}

// https://fonts.google.com/icons?selected=Material+Icons
pub fn expand_more_icon<'a>() -> Text<'a> {
    icon('\u{e5cf}')
}
pub fn expand_less_icon<'a>() -> Text<'a> {
    icon('\u{e5ce}')
}
#[allow(dead_code)]
pub fn edit_icon<'a>() -> Text<'a> {
    icon('\u{e254}')
}
#[allow(dead_code)]
pub fn delete_icon<'a>() -> Text<'a> {
    icon('\u{e872}')
}
#[allow(dead_code)]
pub fn settings_icon<'a>() -> Text<'a> {
    icon('\u{e8b8}')
}
#[allow(dead_code)]
pub fn rewind_icon<'a>() -> Text<'a> {
    icon('\u{e020}')
}
#[allow(dead_code)]
pub fn fast_forward_icon<'a>() -> Text<'a> {
    icon('\u{e01f}')
}
#[allow(dead_code)]
pub fn mute_icon<'a>(is_muted: bool) -> Text<'a> {
    if is_muted {
        icon('\u{e04f}')
    } else {
        icon('\u{e050}')
    }
}
#[allow(dead_code)]
pub fn muted_music_icon<'a>() -> Text<'a> {
    icon('\u{e440}')
}
#[allow(dead_code)]
pub fn unmuted_music_icon<'a>() -> Text<'a> {
    icon('\u{e405}')
}
#[allow(dead_code)]
pub fn skip_to_next_icon<'a>() -> Text<'a> {
    icon('\u{e044}')
}
#[allow(dead_code)]
pub fn home_icon<'a>() -> Text<'a> {
    icon('\u{e88a}')
}
#[allow(dead_code)]
pub fn clock_icon<'a>() -> Text<'a> {
    icon('\u{e8b5}')
}
