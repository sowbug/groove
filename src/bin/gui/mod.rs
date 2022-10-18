pub mod persistence;
pub mod style;

use iced::{alignment, Font, Length, Text};

// Fonts
const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../../../resources/fonts/MaterialIcons-Regular.ttf"),
};

fn icon(unicode: char) -> Text {
    Text::new(unicode.to_string())
        .font(ICONS)
        .width(Length::Units(20))
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}

// https://fonts.google.com/icons?selected=Material+Icons
pub fn edit_icon() -> Text {
    icon('\u{e254}')
}
pub fn delete_icon() -> Text {
    icon('\u{e872}')
}
pub fn settings_icon() -> Text {
    icon('\u{e8b8}')
}
pub fn play_icon() -> Text {
    icon('\u{e037}')
}
pub fn pause_icon() -> Text {
    icon('\u{e034}')
}
pub fn stop_icon() -> Text {
    icon('\u{e047}')
}
pub fn rewind_icon() -> Text {
    icon('\u{e020}')
}
pub fn fast_forward_icon() -> Text {
    icon('\u{e01f}')
}
pub fn mute_icon(is_muted: bool) -> Text {
    if is_muted {
        icon('\u{e04f}')
    } else {
        icon('\u{e050}')
    }
}
pub fn muted_music_icon() -> Text {
    icon('\u{e440}')
}
pub fn unmuted_music_icon() -> Text {
    icon('\u{e405}')
}
pub fn skip_to_prev_icon() -> Text {
    icon('\u{e045}')
}
pub fn skip_to_next_icon() -> Text {
    icon('\u{e044}')
}
pub fn home_icon() -> Text {
    icon('\u{e88a}')
}
pub fn clock_icon() -> Text {
    icon('\u{e8b5}')
}
