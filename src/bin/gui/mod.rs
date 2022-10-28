pub mod persistence;

use iced::widget::text;
use iced::widget::Text;
use iced::{alignment, Font, Length};

// Fonts
const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../../../resources/fonts/MaterialIcons-Regular.ttf"),
};

fn icon<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .font(ICONS)
        .width(Length::Units(20))
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}

// https://fonts.google.com/icons?selected=Material+Icons
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
