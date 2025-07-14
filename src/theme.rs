use ratatui::style::Color;

#[derive(Debug)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub highlight_bd: Color,
    pub help_key: Color,
    pub help_text: Color,
    pub title_foreground: Color,
    pub tip_bg: Color,
    pub tip_fg: Color,

    pub name: String,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: Color::Black,
            foreground: Color::White,
            border: Color::DarkGray,
            highlight_bg: Color::Blue,
            highlight_fg: Color::White,
            highlight_bd: Color::Blue,
            help_key: Color::Green,
            help_text: Color::Gray,
            title_foreground: Color::Cyan,
            tip_bg: Color::DarkGray,
            tip_fg: Color::Yellow,
            name: String::from("dark"),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::White,
            foreground: Color::Black,
            border: Color::DarkGray,
            highlight_bg: Color::LightBlue,
            highlight_fg: Color::White,
            highlight_bd: Color::Blue,
            help_key: Color::Blue,
            help_text: Color::DarkGray,
            title_foreground: Color::Blue,
            tip_bg: Color::Gray,
            tip_fg: Color::Red,
            name: String::from("light"),
        }
    }
}

impl From<&str> for Theme {
    fn from(value: &str) -> Self {
        match value {
            "dark" => Self::dark(),
            "light" => Self::light(),
            _ => Self::dark(),
        }
    }
}
