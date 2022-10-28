pub mod persistence;
pub mod style;

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
pub fn edit_icon<'a>() -> Text<'a> {
    icon('\u{e254}')
}
pub fn delete_icon<'a>() -> Text<'a> {
    icon('\u{e872}')
}
pub fn settings_icon<'a>() -> Text<'a> {
    icon('\u{e8b8}')
}
pub fn play_icon<'a>() -> Text<'a> {
    icon('\u{e037}')
}
pub fn pause_icon<'a>() -> Text<'a> {
    icon('\u{e034}')
}
pub fn stop_icon<'a>() -> Text<'a> {
    icon('\u{e047}')
}
pub fn rewind_icon<'a>() -> Text<'a> {
    icon('\u{e020}')
}
pub fn fast_forward_icon<'a>() -> Text<'a> {
    icon('\u{e01f}')
}
pub fn mute_icon<'a>(is_muted: bool) -> Text<'a> {
    if is_muted {
        icon('\u{e04f}')
    } else {
        icon('\u{e050}')
    }
}
pub fn muted_music_icon<'a>() -> Text<'a> {
    icon('\u{e440}')
}
pub fn unmuted_music_icon<'a>() -> Text<'a> {
    icon('\u{e405}')
}
pub fn skip_to_prev_icon<'a>() -> Text<'a> {
    icon('\u{e045}')
}
pub fn skip_to_next_icon<'a>() -> Text<'a> {
    icon('\u{e044}')
}
pub fn home_icon<'a>() -> Text<'a> {
    icon('\u{e88a}')
}
pub fn clock_icon<'a>() -> Text<'a> {
    icon('\u{e8b5}')
}
