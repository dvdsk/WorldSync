use iced::{button, text_input, Background, Color};

pub enum Button {
    Clickable,
    Blocked,
}

impl button::StyleSheet for Button {
    fn active(&self) -> button::Style {
        match self {
            Button::Clickable => button::Style::default(),
            Button::Blocked => button::Style {
                background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.7))),
                border_radius: 10.0,
                text_color: Color::WHITE,
                ..button::Style::default()
            },
        }
    }

    fn hovered(&self) -> button::Style {
        match self {
            Button::Clickable => button::Style::default(),
            Button::Blocked => button::Style {
                background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.7))),
                border_radius: 10.0,
                text_color: Color::WHITE,
                ..button::Style::default()
            },
        }
    }
}

#[derive(Clone, Copy)]
pub enum Input {
    Ok,
    Err,
}

impl Default for Input {
    fn default() -> Self {
        Self::Ok
    }
}

impl text_input::StyleSheet for Input {
    fn active(&self) -> text_input::Style {
        match self {
            Self::Ok => text_input::Style {
                background: Background::Color(Color::WHITE),
                border_radius: 0.0,
                border_width: 1.0,
                border_color: Color::from_rgb(0.7, 0.7, 0.7),
            },
            Self::Err => text_input::Style {
                background: Background::Color(Color::from_rgb(1., 0.7, 0.7)),
                ..text_input::Style::default()
            },
        }
    }
    fn focused(&self) -> text_input::Style {
        text_input::Style {
            background: Background::Color(Color::WHITE),
            border_radius: 0.0,
            border_width: 1.0,
            border_color: Color::from_rgb(0.7, 0.7, 0.7),
        }
    }
    fn placeholder_color(&self) -> Color {
        Color::from_rgb(0.7, 0.7, 0.7)
    }
    fn value_color(&self) -> Color {
        Color::from_rgb(0.3, 0.3, 0.3)
    }
    fn selection_color(&self) -> Color {
        Color::from_rgb(0.8, 0.8, 1.0)
    }
}
