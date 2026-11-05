//! Colour tokens, light and dark, carried over from the old `app.css`.

use gpui::{rgb, App, Global, Hsla, SharedString, WindowAppearance};

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Hsla,
    pub surface: Hsla,
    pub surface_2: Hsla,
    pub surface_hover: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub border: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub accent_text: Hsla,
    pub green: Hsla,
    pub green_bg: Hsla,
    pub amber: Hsla,
    pub amber_bg: Hsla,
    pub red: Hsla,
    pub red_bg: Hsla,
    pub blue: Hsla,
    pub blue_bg: Hsla,
    pub grey: Hsla,
    pub grey_bg: Hsla,
    /// Backdrop behind the cluster form.
    pub scrim: Hsla,
    pub mono: SharedString,
}

impl Global for Theme {}

fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn mono_font() -> SharedString {
    if cfg!(target_os = "macos") {
        "Menlo".into()
    } else if cfg!(target_os = "windows") {
        "Consolas".into()
    } else {
        "DejaVu Sans Mono".into()
    }
}

impl Theme {
    pub fn light() -> Self {
        Self {
            bg: c(0xf4f5f7),
            surface: c(0xffffff),
            surface_2: c(0xeceef2),
            surface_hover: c(0xf1f3f6),
            text: c(0x1b1f27),
            muted: c(0x6b7280),
            border: c(0xe1e4ea),
            accent: c(0x2f6fed),
            accent_hover: c(0x2560d6),
            accent_text: c(0xffffff),
            green: c(0x15803d),
            green_bg: c(0xdcfce7),
            amber: c(0xb45309),
            amber_bg: c(0xfef3c7),
            red: c(0xb91c1c),
            red_bg: c(0xfee2e2),
            blue: c(0x1d4ed8),
            blue_bg: c(0xdbeafe),
            grey: c(0x4b5563),
            grey_bg: c(0xe5e7eb),
            scrim: gpui::black().opacity(0.35),
            mono: mono_font(),
        }
    }

    pub fn dark() -> Self {
        Self {
            bg: c(0x111318),
            surface: c(0x181b22),
            surface_2: c(0x20242d),
            surface_hover: c(0x1e222b),
            text: c(0xe6e8ee),
            muted: c(0x9aa1ad),
            border: c(0x2a2f3a),
            accent: c(0x5b8def),
            accent_hover: c(0x6f9cf3),
            accent_text: c(0xffffff),
            green: c(0x4ade80),
            green_bg: c(0x14532d),
            amber: c(0xfbbf24),
            amber_bg: c(0x78350f),
            red: c(0xf87171),
            red_bg: c(0x7f1d1d),
            blue: c(0x93c5fd),
            blue_bg: c(0x1e3a8a),
            grey: c(0xcbd5e1),
            grey_bg: c(0x374151),
            scrim: gpui::black().opacity(0.5),
            mono: mono_font(),
        }
    }

    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
        }
    }
}

pub fn theme(cx: &App) -> &Theme {
    cx.global::<Theme>()
}
