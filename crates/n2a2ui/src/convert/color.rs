//! Notion `Color` → CSS color string mapping.
//!
//! Mirrors the palette used by `notion-to-jarkup`: foreground Notion
//! tokens map to a muted hex code, background tokens map to a paler
//! companion. Returning hex (rather than the raw Notion token) means
//! downstream renderers can drop the value straight into a CSS
//! property — see `ElmParagraph` in `@elmethis/qwik`, which assigns
//! `color` to `--elmethis-scoped-color`.

use notionrs::types::prelude::Color;

/// Map a Notion foreground color to its hex code, or `None` for
/// `Default` / any background variant.
pub(crate) fn map_color(color: Color) -> Option<String> {
    match color {
        Color::Blue => Some("#6987b8".into()),
        Color::Brown => Some("#8b4c3f".into()),
        Color::Gray => Some("#868e9c".into()),
        Color::Green => Some("#59b57c".into()),
        Color::Orange => Some("#bf7e71".into()),
        Color::Pink => Some("#c9699e".into()),
        Color::Purple => Some("#9771bd".into()),
        Color::Red => Some("#b36472".into()),
        Color::Yellow => Some("#b8a36e".into()),
        _ => None,
    }
}

/// Map a Notion background color to its hex code, or `None` for
/// `Default` / any foreground variant.
pub(crate) fn map_background_color(color: Color) -> Option<String> {
    match color {
        Color::BlueBackground => Some("#c0cce1".into()),
        Color::BrownBackground => Some("#d0bdac".into()),
        Color::GrayBackground => Some("#cccfd5".into()),
        Color::GreenBackground => Some("#b1dcc2".into()),
        Color::OrangeBackground => Some("#f1dbd2".into()),
        Color::PinkBackground => Some("#ebc7db".into()),
        Color::PurpleBackground => Some("#d7c8e5".into()),
        Color::RedBackground => Some("#e8c2c2".into()),
        Color::YellowBackground => Some("#f0e9d7".into()),
        _ => None,
    }
}
