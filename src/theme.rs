//! Theme module: provides a modern dark palette for the `rustm` TUI.
//!
//! Only the palette entries supported by Cursive 0.21 are customized:
//! `Background`, `Shadow`, `View`, `Primary`, `Secondary`, `Tertiary`,
//! `TitlePrimary`, `TitleSecondary`, `Highlight`, `HighlightInactive`, `HighlightText`.
//!
//! Goals:
//! - Dark, low-glare backgrounds (neutral blue‑gray range).
//! - High‑contrast accent (purple) for focus & selection.
//! - Clear hierarchy of text brightness (`Primary` > `Secondary` > `Tertiary`).
//! - Soft, readable titles (slightly brighter than body text).
//! - Avoid pure white to reduce eye strain.
//!
//! Public API:
//! - `apply_theme(&mut Cursive)` to set the theme on the root.
//! - `modern_theme()` returns the configured `Theme` (for further user tweaking).
//!
//! Future extensions (not implemented here):
//! - Light theme variant.
//! - Dynamically loading theme from a user config file.
//! - Allow runtime switching.
//!
//! This file is deliberately dependency‑light and UI‑agnostic.

use cursive::theme::{BorderStyle, Color, Palette, PaletteColor, Theme};

/// Apply the modern theme directly to a `Cursive` root.
pub fn apply_theme(siv: &mut cursive::Cursive) {
    siv.set_theme(modern_theme());
}

/// Construct and return the modern dark theme.
pub fn modern_theme() -> Theme {
    Theme {
        borders: BorderStyle::Simple,
        shadow: false, // Turn off drop shadows for a cleaner terminal look.
        palette: build_palette(),
    }
}

/// Build the palette with valid `PaletteColor` variants only.
fn build_palette() -> Palette {
    let mut p = Palette::default();

    // Base surfaces.
    p[PaletteColor::Background] = rgb(18, 20, 24); // Global background (near graphite).
    p[PaletteColor::Shadow] = rgb(10, 11, 13); // Subtle shadow (darker tone).
    p[PaletteColor::View] = rgb(28, 30, 34); // Panel / dialog background.

    // Text hierarchy.
    p[PaletteColor::Primary] = rgb(230, 232, 235); // Main text (soft off‑white).
    p[PaletteColor::Secondary] = rgb(168, 174, 186); // Muted gray‑blue.
    p[PaletteColor::Tertiary] = rgb(121, 127, 140); // Further subdued for hints / placeholders.

    // Titles.
    p[PaletteColor::TitlePrimary] = rgb(245, 246, 248); // Slightly brighter than Primary.
    p[PaletteColor::TitleSecondary] = rgb(194, 198, 206); // Dimmed title (inactive headers).

    // Accent colors.
    let accent_active = rgb(166, 104, 255); // Vibrant purple.
    let accent_inactive = rgb(115, 78, 185); // Dimmed counterpart.

    p[PaletteColor::Highlight] = accent_active;
    p[PaletteColor::HighlightInactive] = accent_inactive;
    p[PaletteColor::HighlightText] = rgb(255, 255, 255); // Text on highlighted background.

    p
}

/// Convenience: construct an RGB color.
const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

// ---------------------------
// Tests
// ---------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use cursive::theme::PaletteColor;

    #[test]
    fn highlight_is_expected_accent() {
        let theme = modern_theme();
        match theme.palette[PaletteColor::Highlight] {
            Color::Rgb(r, g, b) => assert_eq!((r, g, b), (166, 104, 255)),
            other => panic!("Unexpected highlight color variant: {:?}", other),
        }
    }

    #[test]
    fn contrast_primary_vs_background_reasonable() {
        let t = modern_theme();
        let primary = t.palette[PaletteColor::Primary];
        let bg = t.palette[PaletteColor::Background];

        let luminance = |c: Color| -> f32 {
            match c {
                Color::Rgb(r, g, b) => {
                    // Simple relative luminance approximation.
                    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
                }
                _ => 0.0,
            }
        };
        let contrast_ratio = (luminance(primary) + 0.05) / (luminance(bg) + 0.05);

        // Not aiming for WCAG perfection (terminal constraints), but ensure a baseline.
        assert!(
            contrast_ratio > 3.0,
            "Contrast ratio too low: {:.2} (expected > 3.0)",
            contrast_ratio
        );
    }
}
