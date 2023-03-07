// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub mod persistence;

use iced::widget::Text;
use iced::{
    alignment::{self, Vertical},
    theme,
    widget::{self, button, column, container, row, text},
    Color, Element, Font, Length, Renderer, Theme,
};
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
        disclosure_triangle_message: Message,
    ) -> Element<'a, Message> {
        let disclosure = button(
            if is_expanded {
                expand_less_icon()
            } else {
                expand_more_icon()
            }
            .size(8),
        )
        .on_press(disclosure_triangle_message);

        container(row![
            disclosure,
            text(title.to_string())
                .font(SMALL_FONT)
                .size(SMALL_FONT_SIZE)
                .horizontal_alignment(iced::alignment::Horizontal::Left)
                .vertical_alignment(Vertical::Center),
        ])
        .width(iced::Length::Fill)
        .padding(1)
        .style(theme::Container::Custom(
            Self::titled_container_title_style(&Theme::Dark),
        ))
        .into()
    }

    pub fn collapsed_container(
        title: &str,
        disclosure_triangle_message: Message,
    ) -> Element<'a, Message> {
        container(Self::container_title_bar(
            title,
            false,
            disclosure_triangle_message,
        ))
        .width(iced::Length::Fill)
        .padding(0)
        .style(theme::Container::Box)
        .into()
    }

    pub fn expanded_container(
        title: &str,
        disclosure_triangle_message: Message,
        contents: Element<'a, Message>,
    ) -> Element<'a, Message> {
        container(column![
            Self::container_title_bar(title, true, disclosure_triangle_message),
            contents,
        ])
        .width(iced::Length::Fill)
        .padding(0)
        .style(theme::Container::Box)
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
pub fn play_icon<'a>() -> Text<'a> {
    icon('\u{e037}')
}
#[allow(dead_code)]
pub fn pause_icon<'a>() -> Text<'a> {
    icon('\u{e034}')
}
pub fn stop_icon<'a>() -> Text<'a> {
    icon('\u{e047}')
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
pub fn skip_to_prev_icon<'a>() -> Text<'a> {
    icon('\u{e045}')
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
