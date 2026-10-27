//! Colour values and the terminal capability they are rendered at.
//!
//! A theme names the colour it wants — usually in true colour, because that is
//! how its design is published — and this module decides what the terminal is
//! actually told. Nothing here reads the environment: the caller supplies the
//! depth it detected, so the conversion is a pure function and testable.

use anstyle::{Ansi256Color, AnsiColor, Color};

/// What a theme asks for. `Default` means "leave the terminal's own colour",
/// which is how a role stays unstyled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Colour {
    #[default]
    Default,
    Ansi(AnsiColor),
    Rgb(u8, u8, u8),
}

impl Colour {
    /// Parse a colour as it appears in a theme file: a basic ANSI name,
    /// optionally prefixed with `bright-`, or `#rrggbb`.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.eq_ignore_ascii_case("default") {
            return Some(Colour::Default);
        }
        if let Some(hex) = text.strip_prefix('#')
            && hex.len() == 6
            && hex.chars().all(|c| c.is_ascii_hexdigit())
        {
            let channel =
                |range: std::ops::Range<usize>| u8::from_str_radix(hex.get(range)?, 16).ok();
            return Some(Colour::Rgb(channel(0..2)?, channel(2..4)?, channel(4..6)?));
        }
        let text = text.to_ascii_lowercase();
        let (bright, name) = match text.strip_prefix("bright-") {
            Some(name) => (true, name),
            None => (false, text.as_str()),
        };
        let pair = match name {
            "black" => (AnsiColor::Black, AnsiColor::BrightBlack),
            "red" => (AnsiColor::Red, AnsiColor::BrightRed),
            "green" => (AnsiColor::Green, AnsiColor::BrightGreen),
            "yellow" => (AnsiColor::Yellow, AnsiColor::BrightYellow),
            "blue" => (AnsiColor::Blue, AnsiColor::BrightBlue),
            "magenta" => (AnsiColor::Magenta, AnsiColor::BrightMagenta),
            "cyan" => (AnsiColor::Cyan, AnsiColor::BrightCyan),
            "white" => (AnsiColor::White, AnsiColor::BrightWhite),
            _ => return None,
        };
        Some(Colour::Ansi(if bright { pair.1 } else { pair.0 }))
    }

    /// The escape the terminal should receive, or `None` when colour is off.
    /// A colour the terminal cannot express is reduced rather than dropped:
    /// the shape of the design survives losing its exact shades.
    pub fn resolve(self, depth: Depth) -> Option<Color> {
        match (self, depth) {
            (_, Depth::None) => None,
            (Colour::Default, _) => None,
            (Colour::Ansi(colour), Depth::Ansi16 | Depth::Ansi256 | Depth::TrueColor) => {
                Some(Color::Ansi(colour))
            }
            (Colour::Rgb(r, g, b), Depth::Ansi16) => Some(Color::Ansi(nearest_ansi16((r, g, b)))),
            (Colour::Rgb(r, g, b), Depth::Ansi256) => {
                Some(Color::Ansi256(Ansi256Color(nearest_index((r, g, b)))))
            }
            (Colour::Rgb(r, g, b), Depth::TrueColor) => Some(Color::Rgb((r, g, b).into())),
        }
    }
}

impl std::fmt::Display for Colour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Colour::Default => f.write_str("default"),
            Colour::Rgb(r, g, b) => write!(f, "#{r:02x}{g:02x}{b:02x}"),
            Colour::Ansi(colour) => f.write_str(match colour {
                AnsiColor::Black => "black",
                AnsiColor::Red => "red",
                AnsiColor::Green => "green",
                AnsiColor::Yellow => "yellow",
                AnsiColor::Blue => "blue",
                AnsiColor::Magenta => "magenta",
                AnsiColor::Cyan => "cyan",
                AnsiColor::White => "white",
                AnsiColor::BrightBlack => "bright-black",
                AnsiColor::BrightRed => "bright-red",
                AnsiColor::BrightGreen => "bright-green",
                AnsiColor::BrightYellow => "bright-yellow",
                AnsiColor::BrightBlue => "bright-blue",
                AnsiColor::BrightMagenta => "bright-magenta",
                AnsiColor::BrightCyan => "bright-cyan",
                AnsiColor::BrightWhite => "bright-white",
            }),
        }
    }
}

/// How much colour the terminal can be trusted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

impl Depth {
    /// Decide the depth from the two environment values terminals use to
    /// advertise it. `None` for `colorterm` means unset; an empty value counts
    /// as unset, as with NO_COLOR.
    pub fn detect(colour_allowed: bool, colorterm: Option<&str>, term: Option<&str>) -> Self {
        if !colour_allowed {
            return Self::None;
        }
        if colorterm.is_some_and(|value| {
            let value = value.to_ascii_lowercase();
            value.contains("truecolor") || value.contains("24bit")
        }) {
            return Self::TrueColor;
        }
        if term.is_some_and(|value| value.to_ascii_lowercase().contains("256color")) {
            return Self::Ansi256;
        }
        Self::Ansi16
    }
}

/// The xterm palette, which is what the 256-colour cube and the 16 basic
/// colours are indexed against here. Terminals customise these, so the 16-colour
/// reduction is an approximation by definition.
fn indexed_rgb(index: u8) -> (u8, u8, u8) {
    const BASIC: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    match index {
        0..=15 => BASIC[index as usize],
        16..=231 => {
            let index = index - 16;
            let component = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
            (
                component(index / 36),
                component(index % 36 / 6),
                component(index % 6),
            )
        }
        _ => {
            let level = 8 + (index - 232) * 10;
            (level, level, level)
        }
    }
}

fn distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
    let channel = |x: u8, y: u8| {
        let x = i32::from(x);
        let y = i32::from(y);
        ((x - y) * (x - y)) as u32
    };
    channel(a.0, b.0) + channel(a.1, b.1) + channel(a.2, b.2)
}

fn nearest_ansi16(rgb: (u8, u8, u8)) -> AnsiColor {
    const BASIC: [AnsiColor; 16] = [
        AnsiColor::Black,
        AnsiColor::Red,
        AnsiColor::Green,
        AnsiColor::Yellow,
        AnsiColor::Blue,
        AnsiColor::Magenta,
        AnsiColor::Cyan,
        AnsiColor::White,
        AnsiColor::BrightBlack,
        AnsiColor::BrightRed,
        AnsiColor::BrightGreen,
        AnsiColor::BrightYellow,
        AnsiColor::BrightBlue,
        AnsiColor::BrightMagenta,
        AnsiColor::BrightCyan,
        AnsiColor::BrightWhite,
    ];
    // The nearest of a fixed sixteen, computed without an unwrap: the search
    // cannot be empty, but production code does not get to assume that.
    let mut best = (BASIC[0], u32::MAX);
    for (index, colour) in BASIC.iter().enumerate() {
        let candidate = distance(rgb, indexed_rgb(index as u8));
        if candidate < best.1 {
            best = (*colour, candidate);
        }
    }
    best.0
}

/// The classic 6×6×6 cube plus its grey ramp, with the greys preferred for
/// near-grey input so a shade does not pick up a colour cast.
fn nearest_index(rgb: (u8, u8, u8)) -> u8 {
    let level = |value: u8| match value {
        0..=47 => 0,
        48..=114 => 1,
        value => (value - 35) / 40,
    };
    let cube = 16 + 36 * level(rgb.0) + 6 * level(rgb.1) + level(rgb.2);
    // The grey ramp is only exact for greys, but it is also the closer choice
    // for near-greys, so both candidates are measured rather than guessed.
    let average = (u16::from(rgb.0) + u16::from(rgb.1) + u16::from(rgb.2)) / 3;
    let step = (average.saturating_sub(8) + 5) / 10;
    let grey = 232 + step.min(23) as u8;
    if distance(rgb, indexed_rgb(grey)) < distance(rgb, indexed_rgb(cube)) {
        grey
    } else {
        cube
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_hex_parse_and_anything_else_does_not() {
        assert_eq!(Colour::parse("cyan"), Some(Colour::Ansi(AnsiColor::Cyan)));
        assert_eq!(
            Colour::parse("Bright-Red"),
            Some(Colour::Ansi(AnsiColor::BrightRed))
        );
        assert_eq!(
            Colour::parse("#1E1E2E"),
            Some(Colour::Rgb(0x1e, 0x1e, 0x2e))
        );
        assert_eq!(Colour::parse("#12345"), None);
        assert_eq!(Colour::parse("#gggggg"), None);
        assert_eq!(Colour::parse("chartreuse"), None);
        assert_eq!(Colour::parse(""), None);
    }

    #[test]
    fn every_colour_written_out_is_read_back() {
        for text in [
            "default",
            "red",
            "bright-cyan",
            "#1e1e2e",
            "#ffffff",
            "black",
        ] {
            let colour = Colour::parse(text).expect(text);
            assert_eq!(colour.to_string(), text);
        }
    }

    #[test]
    fn colour_is_reduced_to_what_the_terminal_can_show() {
        let mocha = Colour::Rgb(0xcb, 0xa6, 0xf7);
        assert_eq!(
            mocha.resolve(Depth::TrueColor),
            Some(Color::Rgb((0xcb, 0xa6, 0xf7).into()))
        );
        // 256 is an index, never the raw triple.
        assert!(matches!(
            mocha.resolve(Depth::Ansi256),
            Some(Color::Ansi256(_))
        ));
        assert!(matches!(mocha.resolve(Depth::Ansi16), Some(Color::Ansi(_))));
        assert_eq!(mocha.resolve(Depth::None), None);
        assert_eq!(Colour::Default.resolve(Depth::TrueColor), None);
    }

    #[test]
    fn the_grey_ramp_round_trips_and_the_cube_is_indexed() {
        assert_eq!(nearest_index((0, 0, 0)), 16);
        assert_eq!(nearest_index((255, 255, 255)), 231);
        assert_eq!(nearest_index((0x80, 0x80, 0x80)), 244);
        assert_eq!(nearest_index((255, 0, 0)), 16 + 36 * 5);
        for index in 16..=255u8 {
            assert!(
                index == nearest_index(indexed_rgb(index))
                    || distance(
                        indexed_rgb(index),
                        indexed_rgb(nearest_index(indexed_rgb(index)))
                    ) <= 3,
                "index {index} did not survive the round trip"
            );
        }
    }

    #[test]
    fn a_reduced_colour_is_still_a_colour() {
        // Pure red is exactly a basic colour, so 16-colour mode must pick it.
        assert_eq!(nearest_ansi16((255, 0, 0)), AnsiColor::BrightRed);
        assert_eq!(nearest_ansi16((0, 0, 0)), AnsiColor::Black);
        assert_eq!(nearest_ansi16((255, 255, 255)), AnsiColor::BrightWhite);
    }

    #[test]
    fn depth_follows_the_environment() {
        assert_eq!(Depth::detect(false, None, None), Depth::None);
        assert_eq!(Depth::detect(true, None, None), Depth::Ansi16);
        assert_eq!(
            Depth::detect(true, None, Some("xterm-256color")),
            Depth::Ansi256
        );
        assert_eq!(
            Depth::detect(true, Some("truecolor"), Some("xterm")),
            Depth::TrueColor
        );
        assert_eq!(
            Depth::detect(true, Some("24bit"), Some("screen-256color")),
            Depth::TrueColor
        );
        assert_eq!(Depth::detect(true, Some(""), Some("dumb")), Depth::Ansi16);
    }
}
