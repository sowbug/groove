use iced::{button, container, Color, Vector};

pub enum Button {
    Icon,
}

impl button::StyleSheet for Button {
    fn active(&self) -> button::Style {
        match self {
            Button::Icon => button::Style {
                text_color: Color::from_rgb(0.5, 0.5, 0.5),
                ..button::Style::default()
            },
        }
    }

    fn hovered(&self) -> button::Style {
        let active = self.active();

        button::Style {
            text_color: match self {
                Button::Icon => Color::from_rgb(0.2, 0.2, 0.7),
            },
            shadow_offset: active.shadow_offset + Vector::new(0.0, 1.0),
            ..active
        }
    }
}

#[derive(Default)]
pub struct BorderedContainer {}

impl container::StyleSheet for BorderedContainer {
    fn style(&self) -> container::Style {
        container::Style {
            border_color: iced::Color::BLACK,
            border_width: 1.0,
            ..Default::default()
        }
    }
}
