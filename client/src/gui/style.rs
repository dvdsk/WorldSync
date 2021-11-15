use iced::{button, Background, Color, Vector};

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
        let active = self.active();
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
