// SPDX-License-Identifier: GPL-3.0-or-later
//! A theme overrides colors, sizes, and fonts from a base style. You can
//! edit the TOML file without compiling a new style library.
//!
//! ```toml
//! name = "Sage"            # what to call it, for people
//! style = "ninetynine"     # the style it is a way of wearing
//!
//! [colours]                # the style's own, by the names it gives them
//! tint = "#ccddcc"
//!
//! [palette]                # what the engine draws words and marks in
//! ink = "#223322"
//!
//! [sizes]                  # any of the sizes a style decides
//! row_height = 18
//!
//! [type]                   # interface, small, small_bold, reading, fixed, title, frame
//! interface = { family = "DejaVu Sans", size = 12 }
//! ```
//!
//! Select a theme by its filename without the extension. For example,
//! set `PRESS_STYLE=sage` to select `sage.toml`.

use press_core::{Metrics, Palette, Type};

/// Overrides to apply to a base style.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Theme {
    /// Display name. The engine selects the theme by filename.
    pub name: Option<String>,
    /// Base style name or path to a style library.
    pub style: String,
    /// The style's own colours, by the names the style gives them.
    pub colours: Vec<(String, u32)>,
    pub palette: Vec<(String, u32)>,
    pub sizes: Vec<(String, i32)>,
    /// Typefaces, by the number a style library knows each use by.
    pub types: Vec<(u32, Type)>,
    /// Diagnostics for entries the parser skipped.
    pub doubts: Vec<String>,
}

/// The uses of type a theme may speak of, in the order a style library
/// numbers them.
pub const TYPES: [&str; 7] = ["interface", "small", "small_bold", "reading", "fixed", "title", "frame"];

macro_rules! by_name {
    ($list:ident, $find:ident, $of:ty, $kind:ty: $($field:ident),* $(,)?) => {
        /// The names a theme may use here.
        pub const $list: &[&str] = &[$(stringify!($field)),*];

        fn $find<'a>(of: &'a mut $of, name: &str) -> Option<&'a mut $kind> {
            match name {
                $(stringify!($field) => Some(&mut of.$field),)*
                _ => None,
            }
        }
    };
}

by_name!(SIZES, size, Metrics, i32:
    button_height, button_min_width, button_pad, default_ring, check_size, check_gap, radio_size, radio_gap,
    field_inset_x, field_inset_y, progress_height, slider_thick, scroll_bar, slider_knob, tab_height, popup_height,
    row_height, menu_bar_height, menu_item_height, menu_shadow, frame_title, frame_border, frame_button_width,
    frame_button_height, frame_button_gap, frame_button_inset, frame_button_rise, frame_left, frame_right,
    menu_in_window, frame_tab, shortcut_form, frame_foot, frame_double, scroll_arrows, tab_align, menu_border,
    frame_shadow, ok_leads,
);

by_name!(PALETTE, colour, Palette, u32:
    window, text, paper, ink, menu, menu_text, list, list_ink, pane, tip, tip_text, pointer, pointer_edge,
);

/// A colour as a theme writes it: "#rrggbb", or "#rgb" for short, or the
/// number 0xRRGGBB. It is solid whatever is said.
fn colour_of(value: &toml::Value) -> Option<u32> {
    let rgb = match value {
        toml::Value::Integer(n) => u32::try_from(*n).ok().filter(|n| *n <= 0xff_ffff)?,
        toml::Value::String(s) => {
            let hex = s.trim().strip_prefix('#')?;
            if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            let n = u32::from_str_radix(hex, 16).ok()?;
            match hex.len() {
                6 => n,
                3 => ((n >> 8 & 15) * 0x11) << 16 | ((n >> 4 & 15) * 0x11) << 8 | (n & 15) * 0x11,
                _ => return None,
            }
        }
        _ => return None,
    };
    Some(0xff00_0000 | rgb)
}

/// Whether a family may be asked of the machine: its name goes to
/// fontconfig as it is, so nothing in it may be taken for the rest of a
/// pattern. The engine's own are named press:...
pub fn family_is_plain(family: &str) -> bool {
    !family.is_empty() && (family.starts_with("press:") || family.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | ',' | '-' | '_' | '.')))
}

impl Theme {
    /// Read a theme. What is not understood in it is passed over and noted
    /// in `doubts`, so that a theme written for a later engine is still worn.
    pub fn parse(text: &str) -> Result<Theme, String> {
        let file: toml::Table = text.parse().map_err(|why: toml::de::Error| why.message().to_string())?;
        let mut theme = Theme::default();
        let Some(toml::Value::String(style)) = file.get("style") else {
            return Err("it does not say which style it is of: style = \"...\"".into());
        };
        theme.style = style.trim().to_string();
        if theme.style.is_empty() {
            return Err("it names no style".into());
        }
        for (key, value) in &file {
            match (key.as_str(), value) {
                ("style", _) => {}
                ("name", toml::Value::String(name)) => theme.name = Some(name.clone()),
                ("colours" | "colors", toml::Value::Table(table)) => {
                    for (name, value) in table {
                        match colour_of(value) {
                            Some(c) => theme.colours.push((name.clone(), c)),
                            None => theme.doubts.push(format!("the colour {name} is not written \"#rrggbb\"")),
                        }
                    }
                }
                ("palette", toml::Value::Table(table)) => {
                    for (name, value) in table {
                        match colour_of(value) {
                            Some(c) if PALETTE.contains(&name.as_str()) => theme.palette.push((name.clone(), c)),
                            Some(_) => theme.doubts.push(format!("there is no {name} in the palette, which has: {}", PALETTE.join(", "))),
                            None => theme.doubts.push(format!("the colour {name} is not written \"#rrggbb\"")),
                        }
                    }
                }
                ("sizes", toml::Value::Table(table)) => {
                    for (name, value) in table {
                        match value.as_integer().and_then(|n| i32::try_from(n).ok()) {
                            Some(n) if SIZES.contains(&name.as_str()) => theme.sizes.push((name.clone(), n)),
                            Some(_) => theme.doubts.push(format!("there is no size named {name}")),
                            None => theme.doubts.push(format!("the size {name} is not a whole number")),
                        }
                    }
                }
                ("type", toml::Value::Table(table)) => {
                    for (name, value) in table {
                        let Some(number) = TYPES.iter().position(|t| t == name) else {
                            theme.doubts.push(format!("there is no use of type named {name}; there are: {}", TYPES.join(", ")));
                            continue;
                        };
                        let family = value.get("family").and_then(|f| f.as_str()).unwrap_or("").trim().to_string();
                        let size = value.get("size").and_then(|s| s.as_integer()).unwrap_or(0).clamp(0, 200) as i32;
                        let own = family.starts_with("press:");
                        if !family_is_plain(&family) || (!own && size == 0) {
                            theme.doubts.push(format!("the typeface for {name} wants a family and a size: {{ family = \"...\", size = 12 }}"));
                            continue;
                        }
                        let flag = |key: &str| value.get(key).and_then(|b| b.as_bool()).unwrap_or(false);
                        theme.types.push((number as u32, Type { family, size, bold: flag("bold"), crisp: flag("crisp") }));
                    }
                }
                (other, _) => theme.doubts.push(format!("{other} is not something a theme says")),
            }
        }
        Ok(theme)
    }

    /// Change the sizes this theme speaks of.
    pub fn resize(&self, metrics: &mut Metrics) {
        for (name, value) in &self.sizes {
            if let Some(size) = size(metrics, name) {
                *size = *value;
            }
        }
    }

    /// Change the colours of the palette this theme speaks of.
    pub fn repaint(&self, palette: &mut Palette) {
        for (name, value) in &self.palette {
            if let Some(colour) = colour(palette, name) {
                *colour = *value;
            }
        }
    }

    /// The typeface this theme would have for a numbered use, if it says.
    pub fn typeface(&self, number: u32) -> Option<Type> {
        self.types.iter().find(|(n, _)| *n == number).map(|(_, t)| t.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_is_read_from_its_few_lines() {
        let theme = Theme::parse(
            r##"
            name = "Sage"
            style = "ninetynine"
            [colours]
            tint = "#ccddcc"
            royal = "#363"
            [palette]
            ink = 0x223322
            [sizes]
            row_height = 18
            [type]
            interface = { family = "DejaVu Sans", size = 12, bold = true }
            fixed = { family = "press:pica" }
            "##,
        )
        .unwrap();
        assert_eq!(theme.name.as_deref(), Some("Sage"));
        assert_eq!(theme.style, "ninetynine");
        assert!(theme.colours.contains(&("tint".into(), 0xffccddcc)));
        assert!(theme.colours.contains(&("royal".into(), 0xff336633)));
        assert_eq!(theme.palette, [("ink".to_string(), 0xff223322)]);
        assert_eq!(theme.sizes, [("row_height".to_string(), 18)]);
        assert_eq!(theme.typeface(0), Some(Type { family: "DejaVu Sans".into(), size: 12, bold: true, crisp: false }));
        assert_eq!(theme.typeface(4).map(|t| t.family), Some("press:pica".into()));
        assert_eq!(theme.typeface(1), None);
        assert!(theme.doubts.is_empty(), "{:?}", theme.doubts);

        let mut metrics = press_core::EightyFour::METRICS;
        theme.resize(&mut metrics);
        assert_eq!(metrics.row_height, 18);
        let mut palette = press_core::EightyFour::PALETTE;
        theme.repaint(&mut palette);
        assert_eq!(palette.ink, 0xff223322);
    }

    #[test]
    fn what_a_theme_gets_wrong_is_passed_over_and_noted() {
        let theme = Theme::parse(
            r##"
            style = "ninetyfive"
            flavour = "mint"
            [colors]
            face = "teal"
            navy = "#008080"
            [palette]
            sky = "#ffffff"
            [sizes]
            row_height = "tall"
            rows = 3
            [type]
            interface = { family = "Sans:bold", size = 12 }
            heading = { family = "Sans", size = 12 }
            small = { family = "Sans" }
            "##,
        )
        .unwrap();
        assert_eq!(theme.colours, [("navy".to_string(), 0xff008080)]);
        assert!(theme.palette.is_empty() && theme.sizes.is_empty() && theme.types.is_empty());
        assert_eq!(theme.doubts.len(), 8, "{:?}", theme.doubts);
    }

    #[test]
    fn a_theme_must_say_which_style_it_is_of() {
        assert!(Theme::parse("[colours]\nface = \"#000000\"").is_err());
        assert!(Theme::parse("style = \"\"").is_err());
        assert!(Theme::parse("style = ").is_err());
    }

    #[test]
    fn every_size_and_colour_can_be_named() {
        // the lists above are written by hand: a field added to the engine
        // and not to them would be one no theme could reach
        assert_eq!(SIZES.len() * 4, std::mem::size_of::<Metrics>());
        assert_eq!(PALETTE.len() * 4, std::mem::size_of::<Palette>());
    }
}
