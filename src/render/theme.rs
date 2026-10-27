//! Themes: what each visual role is coloured with, and nothing else.
//!
//! A theme never collects data, never reads the environment and never formats a
//! value. It is a table of colours the renderer consults, so adding a theme is
//! a data change rather than a second rendering path.
//!
//! A role left unset means "keep what the layout would do anyway": the title
//! and labels fall back to the accent, everything else to the terminal's own
//! foreground. A role set to `default` asks for the terminal's own colour
//! explicitly, which for the title and labels is not the same thing: unset
//! follows the accent, `default` refuses it.

use super::colour::Colour;
use anstyle::{AnsiColor, Style};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Stable identifier, as `--theme` and `[theme] name` accept it.
    pub name: String,
    /// One line for `justfetch themes`.
    pub description: String,
    /// Primary artwork colour. `None` keeps each logo's own colour, which is
    /// how the built-in themes stay recognisable across distributions.
    pub logo: Option<Colour>,
    /// Optional second artwork colour. Where both are set, the artwork is
    /// shaded top to bottom between them.
    pub logo_secondary: Option<Colour>,
    pub title: Option<Colour>,
    pub label: Option<Colour>,
    pub value: Option<Colour>,
    pub divider: Option<Colour>,
    /// The `~` that marks an estimated value.
    pub emphasis: Option<Colour>,
    /// The colour chips under the information.
    pub swatches: Vec<Colour>,
}

impl Theme {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            logo: None,
            logo_secondary: None,
            title: None,
            label: None,
            value: None,
            divider: None,
            emphasis: None,
            swatches: Vec::new(),
        }
    }

    /// The style to emit for one role.
    ///
    /// `accent` is the user's `color.accent`: when set it overrides the title
    /// and labels, which is what the configuration says it does. `fallback` is
    /// the effective logo colour, used only when neither the user nor the theme
    /// named one.
    pub fn style(
        &self,
        role: Role,
        accent: Option<Colour>,
        fallback: Option<Colour>,
        depth: super::colour::Depth,
    ) -> Style {
        let chosen = match role {
            Role::Title => accent.or(self.title).or(fallback),
            Role::Label => accent.or(self.label).or(fallback),
            Role::Value => self.value,
            Role::Divider => self.divider,
            Role::Emphasis => self.emphasis,
        };
        match chosen.and_then(|colour| colour.resolve(depth)) {
            // Bold is a structural signal: it marks the title, the label column
            // and the estimate marker. Values and rules stay regular weight so a
            // themed run is not heavier than an unthemed one.
            Some(color) => match role {
                Role::Value | Role::Divider => Style::new().fg_color(Some(color)),
                _ => Style::new().fg_color(Some(color)).bold(),
            },
            None => Style::new(),
        }
    }

    /// The artwork colours for a logo `lines` tall. Two-tone artwork shades
    /// from the primary to the secondary; one colour stays uniform.
    pub fn logo_colours(&self, fallback: Option<Colour>, lines: usize) -> Vec<Option<Colour>> {
        let primary = self.logo.or(fallback);
        match (primary, self.logo_secondary, lines) {
            (Some(primary), Some(secondary), 2..) => (0..lines)
                .map(|index| Some(mix(primary, secondary, index, lines)))
                .collect(),
            _ => vec![primary; lines],
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        default_theme()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Title,
    Label,
    Value,
    Divider,
    Emphasis,
}

/// Linear blend between two colours. Only meaningful in true colour; a terminal
/// that receives 16 or 256 colours quantises the result, and the caller may
/// pass the same colour twice to keep the artwork uniform.
fn mix(from: Colour, to: Colour, index: usize, lines: usize) -> Colour {
    let (Colour::Rgb(r0, g0, b0), Colour::Rgb(r1, g1, b1)) = (from, to) else {
        // Named or indexed colours have no defined midpoint; the first line's
        // colour is the honest answer rather than an invented shade.
        return if index + 1 >= lines { to } else { from };
    };
    let step = |start: u8, end: u8| {
        let start = u32::from(start);
        let end = u32::from(end);
        let position = (index as u32 * 256) / (lines as u32 - 1);
        ((start * (256 - position) + end * position) / 256) as u8
    };
    Colour::Rgb(step(r0, r1), step(g0, g1), step(b0, b1))
}

const NAMES: [(&str, &str); 6] = [
    ("default", "Default"),
    ("catppuccin-mocha", "Catppuccin Mocha"),
    ("nord", "Nord"),
    ("tokyo-night", "Tokyo Night"),
    ("gruvbox", "Gruvbox"),
    ("monochrome", "Monochrome"),
];

/// The identifiers `--theme` and `[theme] name` accept, in listing order.
pub fn names() -> Vec<&'static str> {
    NAMES.iter().map(|(id, _)| *id).collect()
}

/// Look up a built-in theme. Case-insensitive, and `catppuccin` is accepted for
/// the one Catppuccin flavour this project ships.
pub fn built_in(name: &str) -> Option<Theme> {
    let name = name.trim().to_ascii_lowercase();
    let name = match name.as_str() {
        "catppuccin" => "catppuccin-mocha",
        "tokyonight" => "tokyo-night",
        other => other,
    };
    Some(match name {
        "default" => default_theme(),
        "catppuccin-mocha" => catppuccin_mocha(),
        "nord" => nord(),
        "tokyo-night" => tokyo_night(),
        "gruvbox" => gruvbox(),
        "monochrome" => monochrome(),
        _ => return None,
    })
}

fn default_theme() -> Theme {
    let mut theme = Theme::new(
        "default",
        "The terminal's own sixteen colours; artwork keeps its distribution colour.",
    );
    // Role colours stay unset on purpose: the title and labels follow the
    // artwork, and everything else is the terminal's foreground.
    theme.swatches = vec![
        Colour::Ansi(AnsiColor::Black),
        Colour::Ansi(AnsiColor::Red),
        Colour::Ansi(AnsiColor::Green),
        Colour::Ansi(AnsiColor::Yellow),
        Colour::Ansi(AnsiColor::Blue),
        Colour::Ansi(AnsiColor::Magenta),
        Colour::Ansi(AnsiColor::Cyan),
        Colour::Ansi(AnsiColor::White),
    ];
    theme
}

fn catppuccin_mocha() -> Theme {
    let mut theme = Theme::new(
        "catppuccin-mocha",
        "Catppuccin's darkest flavour: soft pastels on a muted base.",
    );
    theme.logo = Some(Colour::Rgb(0xcb, 0xa6, 0xf7));
    theme.logo_secondary = Some(Colour::Rgb(0x89, 0xb4, 0xfa));
    theme.title = Some(Colour::Rgb(0xb4, 0xbe, 0xfe));
    theme.label = Some(Colour::Rgb(0x89, 0xb4, 0xfa));
    theme.value = Some(Colour::Rgb(0xcd, 0xd6, 0xf4));
    theme.divider = Some(Colour::Rgb(0x6c, 0x70, 0x86));
    theme.emphasis = Some(Colour::Rgb(0xfa, 0xb3, 0x87));
    theme.swatches = vec![
        Colour::Rgb(0xf5, 0xc2, 0xe7),
        Colour::Rgb(0xcb, 0xa6, 0xf7),
        Colour::Rgb(0x89, 0xb4, 0xfa),
        Colour::Rgb(0x94, 0xe2, 0xd5),
        Colour::Rgb(0xa6, 0xe3, 0xa1),
        Colour::Rgb(0xf9, 0xe2, 0xaf),
        Colour::Rgb(0xfa, 0xb3, 0x87),
        Colour::Rgb(0xf3, 0x8b, 0xa8),
    ];
    theme
}

fn nord() -> Theme {
    let mut theme = Theme::new(
        "nord",
        "Nord's arctic blues and greys, with a single warm accent.",
    );
    theme.logo = Some(Colour::Rgb(0x88, 0xc0, 0xd0));
    theme.logo_secondary = Some(Colour::Rgb(0x81, 0xa1, 0xc1));
    theme.title = Some(Colour::Rgb(0xec, 0xef, 0xf4));
    theme.label = Some(Colour::Rgb(0x8f, 0xbc, 0xbb));
    theme.value = Some(Colour::Rgb(0xd8, 0xde, 0xe9));
    theme.divider = Some(Colour::Rgb(0x4c, 0x56, 0x6a));
    theme.emphasis = Some(Colour::Rgb(0xeb, 0xcb, 0x8b));
    theme.swatches = vec![
        Colour::Rgb(0x8f, 0xbc, 0xbb),
        Colour::Rgb(0x88, 0xc0, 0xd0),
        Colour::Rgb(0x81, 0xa1, 0xc1),
        Colour::Rgb(0x5e, 0x81, 0xac),
        Colour::Rgb(0xa3, 0xbe, 0x8c),
        Colour::Rgb(0xeb, 0xcb, 0x8b),
        Colour::Rgb(0xd0, 0x87, 0x70),
        Colour::Rgb(0xb4, 0x8e, 0xad),
    ];
    theme
}

fn tokyo_night() -> Theme {
    let mut theme = Theme::new(
        "tokyo-night",
        "Tokyo Night's deep blue base with neon accents.",
    );
    theme.logo = Some(Colour::Rgb(0x7a, 0xa2, 0xf7));
    theme.logo_secondary = Some(Colour::Rgb(0xbb, 0x9a, 0xf7));
    theme.title = Some(Colour::Rgb(0xc0, 0xca, 0xf5));
    theme.label = Some(Colour::Rgb(0x7d, 0xcf, 0xff));
    theme.value = Some(Colour::Rgb(0xa9, 0xb1, 0xd6));
    theme.divider = Some(Colour::Rgb(0x56, 0x5f, 0x89));
    theme.emphasis = Some(Colour::Rgb(0xe0, 0xaf, 0x68));
    theme.swatches = vec![
        Colour::Rgb(0x7a, 0xa2, 0xf7),
        Colour::Rgb(0xbb, 0x9a, 0xf7),
        Colour::Rgb(0x7d, 0xcf, 0xff),
        Colour::Rgb(0x9e, 0xce, 0x6a),
        Colour::Rgb(0xe0, 0xaf, 0x68),
        Colour::Rgb(0xff, 0x9e, 0x64),
        Colour::Rgb(0xf7, 0x76, 0x8e),
        Colour::Rgb(0x73, 0x7a, 0xa2),
    ];
    theme
}

fn gruvbox() -> Theme {
    let mut theme = Theme::new(
        "gruvbox",
        "Gruvbox's warm retro palette: amber, clay and moss.",
    );
    theme.logo = Some(Colour::Rgb(0xfa, 0xbd, 0x2f));
    theme.logo_secondary = Some(Colour::Rgb(0xfe, 0x80, 0x19));
    theme.title = Some(Colour::Rgb(0xeb, 0xdb, 0xb2));
    theme.label = Some(Colour::Rgb(0x83, 0xa5, 0x98));
    theme.value = Some(Colour::Rgb(0xeb, 0xdb, 0xb2));
    theme.divider = Some(Colour::Rgb(0x92, 0x83, 0x74));
    theme.emphasis = Some(Colour::Rgb(0xb8, 0xbb, 0x26));
    theme.swatches = vec![
        Colour::Rgb(0xfb, 0x49, 0x34),
        Colour::Rgb(0xfe, 0x80, 0x19),
        Colour::Rgb(0xfa, 0xbd, 0x2f),
        Colour::Rgb(0xb8, 0xbb, 0x26),
        Colour::Rgb(0x8e, 0xc0, 0x7c),
        Colour::Rgb(0x83, 0xa5, 0x98),
        Colour::Rgb(0xd3, 0x86, 0x9b),
        Colour::Rgb(0xeb, 0xdb, 0xb2),
    ];
    theme
}

fn monochrome() -> Theme {
    let mut theme = Theme::new(
        "monochrome",
        "Greys only, for terminals with one accent and no room for a palette.",
    );
    theme.logo = Some(Colour::Ansi(AnsiColor::White));
    theme.title = Some(Colour::Ansi(AnsiColor::BrightWhite));
    theme.label = Some(Colour::Ansi(AnsiColor::White));
    theme.value = Some(Colour::Default);
    theme.divider = Some(Colour::Ansi(AnsiColor::BrightBlack));
    theme.emphasis = Some(Colour::Ansi(AnsiColor::BrightWhite));
    theme.swatches = vec![
        Colour::Ansi(AnsiColor::BrightBlack),
        Colour::Ansi(AnsiColor::White),
        Colour::Ansi(AnsiColor::BrightWhite),
    ];
    theme
}

/// A theme file, validated. Unknown keys are errors, as in the main config.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileTheme {
    name: String,
    description: Option<String>,
    logo: Option<String>,
    logo_secondary: Option<String>,
    title: Option<String>,
    label: Option<String>,
    value: Option<String>,
    divider: Option<String>,
    emphasis: Option<String>,
    swatches: Option<Vec<String>>,
}

/// Parse a custom theme. Pure: the caller reads the file, so the renderer
/// never touches the filesystem.
pub fn parse(text: &str, origin: &str) -> Result<Theme, String> {
    let file: FileTheme = toml::from_str(text).map_err(|error| format!("{origin}: {error}"))?;
    let name = file.name.trim();
    if name.is_empty() {
        return Err(format!("{origin}: theme name is empty"));
    }
    if name.chars().count() > 64 {
        return Err(format!("{origin}: theme name is longer than 64 characters"));
    }
    // The name is printed by `doctor`. It is user input, so it gets the same
    // treatment as a label: anything the renderer would strip is refused
    // rather than shown or silently dropped.
    if super::layout::sanitize(name) != name {
        return Err(format!("{origin}: theme name contains control characters"));
    }
    let colour = |key: &str, value: Option<String>| -> Result<Option<Colour>, String> {
        match value {
            None => Ok(None),
            Some(text) => Colour::parse(&text).map(Some).ok_or_else(|| {
                format!(
                    "{origin}: {key} is `{text}`; expected `#rrggbb`, a basic colour name, `bright-` plus one, or `default`"
                )
            }),
        }
    };
    let swatches = match file.swatches {
        None => Vec::new(),
        Some(list) => {
            if list.is_empty() || list.len() > 8 {
                return Err(format!(
                    "{origin}: swatches must list between 1 and 8 colours"
                ));
            }
            list.iter()
                .enumerate()
                .map(|(index, text)| {
                    Colour::parse(text)
                        .ok_or_else(|| format!("{origin}: swatches[{index}] is not a colour"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    Ok(Theme {
        name: name.to_owned(),
        description: file
            .description
            .map(|description| super::layout::sanitize(&description))
            .unwrap_or_else(|| "Custom theme".to_owned()),
        logo: colour("logo", file.logo)?,
        logo_secondary: colour("logo_secondary", file.logo_secondary)?,
        title: colour("title", file.title)?,
        label: colour("label", file.label)?,
        value: colour("value", file.value)?,
        divider: colour("divider", file.divider)?,
        emphasis: colour("emphasis", file.emphasis)?,
        swatches,
    })
}

/// Apply a style only when colour is on and there is text. Padding rows must
/// not emit an empty styled span.
pub fn styled(text: &str, style: Style) -> String {
    if text.is_empty() || style == Style::new() {
        return text.to_owned();
    }
    format!("{style}{text}{style:#}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use anstyle::Color;

    #[test]
    fn every_built_in_theme_is_complete_and_distinct() {
        for name in names() {
            let theme = built_in(name).unwrap_or_else(|| panic!("{name} is listed but missing"));
            assert_eq!(theme.name, name);
            assert!(!theme.description.is_empty(), "{name} has no description");
            assert!(!theme.swatches.is_empty(), "{name} has no swatches");
            assert!(theme.swatches.len() <= 8, "{name} has too many swatches");
        }
        // Two themes that render identically would be a maintenance trap.
        let signatures: Vec<_> = names()
            .iter()
            .map(|name| {
                let theme = built_in(name).unwrap();
                (
                    theme.logo,
                    theme.title,
                    theme.label,
                    theme.value,
                    theme.divider,
                )
            })
            .collect();
        for (index, signature) in signatures.iter().enumerate() {
            assert!(
                !signatures[..index].contains(signature),
                "{} duplicates an earlier theme",
                names()[index]
            );
        }
    }

    #[test]
    fn lookup_is_case_insensitive_and_has_aliases() {
        assert!(built_in("Catppuccin-Mocha").is_some());
        assert!(built_in("catppuccin").is_some());
        assert!(built_in("tokyonight").is_some());
        assert!(built_in("nonexistent").is_none());
        assert!(built_in("").is_none());
    }

    #[test]
    fn the_user_accent_overrides_the_theme_which_falls_back_to_the_logo() {
        use super::super::colour::Depth;
        let fallback = Some(Colour::Ansi(AnsiColor::Cyan));
        let default = default_theme();
        let title = default.style(Role::Title, None, fallback, Depth::Ansi16);
        assert_eq!(title.get_fg_color(), Some(Color::Ansi(AnsiColor::Cyan)));
        let value = default.style(Role::Value, None, fallback, Depth::Ansi16);
        assert_eq!(value.get_fg_color(), None);
        // With colour off nothing is styled at all.
        assert_eq!(
            default
                .style(Role::Title, None, fallback, Depth::None)
                .get_fg_color(),
            None
        );

        // A theme that names the role still yields to an explicit accent: the
        // configuration documents `accent` as an override, not a fallback.
        let nord = built_in("nord").expect("built in");
        let overridden = nord.style(
            Role::Title,
            Some(Colour::Ansi(AnsiColor::Red)),
            fallback,
            Depth::Ansi16,
        );
        assert_eq!(overridden.get_fg_color(), Some(Color::Ansi(AnsiColor::Red)));
        let themed = nord.style(Role::Title, None, fallback, Depth::Ansi16);
        assert_ne!(themed.get_fg_color(), Some(Color::Ansi(AnsiColor::Red)));
    }

    #[test]
    fn a_custom_theme_is_validated_key_by_key() {
        let good = parse(
            "name = \"mine\"\ntitle = \"#aabbcc\"\nlabel = \"red\"\n",
            "t.toml",
        );
        assert_eq!(good.unwrap().title, Some(Colour::Rgb(0xaa, 0xbb, 0xcc)));

        for (text, expected) in [
            ("title = \"red\"", "name"),
            ("name = \"\"", "empty"),
            ("name = \"x\"\ntitle = \"chartreuse\"", "title"),
            ("name = \"x\"\nunknown = \"red\"", "unknown field"),
            // TOML escapes, so the parser hands us the real characters.
            ("name = \"evil\\u001B[31m\"", "control characters"),
            ("name = \"evil\\u202E\"", "control characters"),
            ("name = \"x\"\nswatches = []", "swatches"),
            ("name = \"x\"\nswatches = [\"nope\"]", "swatches[0]"),
        ] {
            let error = parse(text, "t.toml").unwrap_err();
            assert!(error.contains(expected), "{text}: {error}");
        }
    }

    #[test]
    fn two_tone_artwork_shades_and_one_tone_does_not() {
        let mut theme = default_theme();
        theme.logo = Some(Colour::Rgb(0, 0, 0));
        theme.logo_secondary = Some(Colour::Rgb(255, 255, 255));
        let colours = theme.logo_colours(None, 3);
        assert_eq!(
            colours,
            vec![
                Some(Colour::Rgb(0, 0, 0)),
                Some(Colour::Rgb(127, 127, 127)),
                Some(Colour::Rgb(255, 255, 255)),
            ]
        );
        let uniform = default_theme().logo_colours(Some(Colour::Ansi(AnsiColor::Cyan)), 4);
        assert_eq!(uniform, vec![Some(Colour::Ansi(AnsiColor::Cyan)); 4]);
    }

    #[test]
    fn a_named_pair_is_not_blended_into_invented_shades() {
        let mut theme = default_theme();
        theme.logo = Some(Colour::Ansi(AnsiColor::Red));
        theme.logo_secondary = Some(Colour::Ansi(AnsiColor::Blue));
        let colours = theme.logo_colours(None, 3);
        assert_eq!(
            colours,
            vec![
                Some(Colour::Ansi(AnsiColor::Red)),
                Some(Colour::Ansi(AnsiColor::Red)),
                Some(Colour::Ansi(AnsiColor::Blue)),
            ]
        );
    }
}
