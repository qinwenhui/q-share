//! q-share native GUI (iced) — terminal aesthetic, monospace, phosphor green.
//!
//! Design language: hacker terminal / cyberpunk console.
//! - Pure black background, phosphor-green (#00FF41) foreground.
//! - Monospace everywhere; ASCII prompts ("$ ", "> ", ">>").
//! - Blinking cursor in "starting" state; breathing status dot when running.
//! - Stop button lives in the header (always one click away while running).

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use iced::overlay::menu;
use iced::widget::{
    button, column, container, horizontal_space, mouse_area, pick_list, row, scrollable, svg, text,
    text_input, vertical_space, Space,
};
use iced::{Alignment, Color, Element, Length, Size, Subscription, Task, Theme};
use qshare_core::{Server, ServerConfig, StatsSnapshot};
use rfd::AsyncFileDialog;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(name = "qshare", version, about = "q-share native GUI")]
struct Args {
    #[arg(long)]
    root: Option<PathBuf>,
    #[arg(long, default_value_t = 8888)]
    port: u16,
    // Note: there is intentionally no `--bind` flag here. The GUI always
    // binds to 0.0.0.0 so both loopback and LAN clients can connect; the
    // displayed URL uses the auto-detected LAN IP. Letting the user pick a
    // bind address was a footgun: a stale/detached LAN IP would let the
    // server "start" but be unreachable from anywhere (including 127.0.0.1),
    // producing a black-console wall of "Connection refused" warnings.
}

// ─── Design tokens ─────────────────────────────────────────────────────────
//
// The GUI ships five switchable visual styles (cycled from a button in the
// header, `[◈ name]`). `CURRENT_STYLE` is a plain atomic the view reads on
// every redraw, and `mod t` is a set of accessors that forward to the
// active palette — so the rest of the code keeps writing `t::fg()` etc.
// no matter which style is live.
//
//   hacker  — phosphor green on black (the original terminal look)
//   tech    — cyan neon on deep blue-black ("hardcore tech")
//   retro   — Win95 teal desktop + grey windows ("retro win")
//   anime   — sakura pink on plum, rounded corners ("anime cartoon")
//   mac     — frosted-glass light chrome, like a macOS panel ("apple glass")

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum GuiStyle {
    Hacker,
    Tech,
    Retro,
    Anime,
    Mac,
}

impl GuiStyle {
    const ALL: [GuiStyle; 5] = [
        GuiStyle::Hacker,
        GuiStyle::Tech,
        GuiStyle::Retro,
        GuiStyle::Anime,
        GuiStyle::Mac,
    ];

    fn name(self) -> &'static str {
        match self {
            GuiStyle::Hacker => "hacker",
            GuiStyle::Tech => "tech",
            GuiStyle::Retro => "retro",
            GuiStyle::Anime => "anime",
            GuiStyle::Mac => "mac",
        }
    }

    fn next(self) -> GuiStyle {
        let i = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// Current visual style, read by `palette()` every redraw.
static CURRENT_STYLE: AtomicU8 = AtomicU8::new(0);

/// One complete colour story for the GUI.
#[derive(Debug, Clone, Copy)]
struct Palette {
    bg: Color,
    card: Color,
    card_hover: Color,
    input_bg: Color,
    border: Color,
    border_subtle: Color,
    border_bright: Color,
    fg: Color,
    fg_dim: Color,
    fg_faint: Color,
    accent: Color,
    success: Color,
    warning: Color,
    danger: Color,
    /// Corner radius for every panel — 0 for terminal/retro, larger for anime.
    radius: f32,
    /// Panel drop-shadow colour (offset varies per style, see `shadow_offset`).
    shadow: Color,
    shadow_offset: (f32, f32),
}

fn palette() -> Palette {
    match CURRENT_STYLE.load(Ordering::Relaxed) % 5 {
        1 => palette_tech(),
        2 => palette_retro(),
        3 => palette_anime(),
        4 => palette_mac(),
        _ => palette_hacker(),
    }
}

fn palette_hacker() -> Palette {
    Palette {
        bg: Color::from_rgb(0.012, 0.020, 0.016),         // #050508
        card: Color::from_rgb(0.027, 0.043, 0.035),       // #070B09
        card_hover: Color::from_rgb(0.047, 0.071, 0.059), // #0C120F
        input_bg: Color::from_rgb(0.020, 0.031, 0.027),   // #050807
        border: Color::from_rgb(0.000, 0.180, 0.094),     // #002E18
        border_subtle: Color::from_rgb(0.000, 0.110, 0.063), // #001C10
        border_bright: Color::from_rgb(0.000, 0.400, 0.220), // #006638
        fg: Color::from_rgb(0.000, 1.000, 0.255),         // #00FF41
        fg_dim: Color::from_rgb(0.000, 0.620, 0.196),     // #009E32
        fg_faint: Color::from_rgb(0.000, 0.349, 0.122),   // #00591F
        accent: Color::from_rgb(0.000, 1.000, 0.255),     // #00FF41
        success: Color::from_rgb(0.000, 1.000, 0.255),    // #00FF41
        warning: Color::from_rgb(1.000, 0.722, 0.000),    // #FFB800
        danger: Color::from_rgb(1.000, 0.196, 0.196),     // #FF3232
        radius: 0.0,
        shadow: Color::from_rgba(0.0, 0.5, 0.25, 0.18),
        shadow_offset: (0.0, 0.0),
    }
}

fn palette_tech() -> Palette {
    Palette {
        bg: Color::from_rgb(0.012, 0.020, 0.035),         // #03050D
        card: Color::from_rgb(0.024, 0.043, 0.078),       // #060B14
        card_hover: Color::from_rgb(0.047, 0.082, 0.129), // #0C1521
        input_bg: Color::from_rgb(0.016, 0.027, 0.051),   // #04070D
        border: Color::from_rgb(0.000, 0.337, 0.553),     // #00568D
        border_subtle: Color::from_rgb(0.000, 0.208, 0.349), // #003559
        border_bright: Color::from_rgb(0.000, 0.608, 0.808), // #009BCE
        fg: Color::from_rgb(0.000, 0.898, 1.000),         // #00E5FF
        fg_dim: Color::from_rgb(0.290, 0.616, 0.753),     // #4A9DC0
        fg_faint: Color::from_rgb(0.122, 0.314, 0.388),   // #1F5063
        accent: Color::from_rgb(0.000, 0.898, 1.000),     // #00E5FF
        success: Color::from_rgb(0.000, 0.898, 1.000),
        warning: Color::from_rgb(1.000, 0.702, 0.000), // #FFB300
        danger: Color::from_rgb(1.000, 0.302, 0.302),  // #FF4D4D
        radius: 0.0,
        shadow: Color::from_rgba(0.0, 0.6, 1.0, 0.18),
        shadow_offset: (0.0, 0.0),
    }
}

fn palette_retro() -> Palette {
    Palette {
        bg: Color::from_rgb(0.000, 0.502, 0.502), // #008080 teal desktop
        card: Color::from_rgb(0.753, 0.753, 0.753), // #C0C0C0 window grey
        card_hover: Color::from_rgb(0.831, 0.831, 0.831), // #D4D4D4
        input_bg: Color::from_rgb(1.000, 1.000, 1.000), // #FFFFFF
        border: Color::from_rgb(0.502, 0.502, 0.502), // #808080
        border_subtle: Color::from_rgb(0.627, 0.627, 0.627), // #A0A0A0
        border_bright: Color::from_rgb(1.000, 1.000, 1.000), // #FFFFFF highlight
        fg: Color::from_rgb(0.000, 0.000, 0.000), // black
        fg_dim: Color::from_rgb(0.251, 0.251, 0.251), // #404040
        fg_faint: Color::from_rgb(0.502, 0.502, 0.502), // #808080
        accent: Color::from_rgb(0.000, 0.000, 0.502), // #000080 navy
        success: Color::from_rgb(0.000, 0.502, 0.000), // #008000
        warning: Color::from_rgb(0.502, 0.502, 0.000), // #808000
        danger: Color::from_rgb(0.502, 0.000, 0.000), // #800000
        radius: 0.0,
        shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
        shadow_offset: (1.0, 1.0), // bevel reads as a hard bottom-right edge
    }
}

// `card_hover`'s blue channel 0.318 is an intentional design value that
// happens to sit near FRAC_1_PI — suppress clippy's approx_constant so the
// palette is left untouched.
#[allow(clippy::approx_constant)]
fn palette_anime() -> Palette {
    Palette {
        bg: Color::from_rgb(0.129, 0.075, 0.149), // #211326 deep plum
        card: Color::from_rgb(0.208, 0.114, 0.235), // #351D3C
        card_hover: Color::from_rgb(0.290, 0.157, 0.318), // #4A2851
        input_bg: Color::from_rgb(0.165, 0.086, 0.184), // #2A162F
        border: Color::from_rgb(0.541, 0.310, 0.533), // #8A4F88
        border_subtle: Color::from_rgb(0.416, 0.227, 0.408), // #6A3A68
        border_bright: Color::from_rgb(0.831, 0.533, 0.788), // #D488C9
        fg: Color::from_rgb(1.000, 0.890, 0.949), // #FFE3F2
        fg_dim: Color::from_rgb(0.878, 0.663, 0.800), // #E0A9CC
        fg_faint: Color::from_rgb(0.690, 0.498, 0.639), // #B07FA3
        accent: Color::from_rgb(1.000, 0.420, 0.710), // #FF6BB5
        success: Color::from_rgb(0.482, 0.878, 0.627), // #7BE0A0
        warning: Color::from_rgb(1.000, 0.706, 0.361), // #FFB45C
        danger: Color::from_rgb(1.000, 0.361, 0.478), // #FF5C7A
        radius: 10.0,
        shadow: Color::from_rgba(1.0, 0.42, 0.71, 0.22),
        shadow_offset: (0.0, 0.0),
    }
}

/// macOS-style frosted glass: a soft periwinkle "desktop" with translucent
/// white panels on top, so each card reads as a sheet of glass. Panels are
/// iced `Color`s with alpha — wgpu composites them over `bg`, which is what
/// produces the frosted tint. Radius and a soft drop shadow finish the
/// Big Sur look (there's no real backdrop blur available behind widgets).
fn palette_mac() -> Palette {
    Palette {
        bg: Color::from_rgb(0.663, 0.718, 0.839), // #A9B7D6 periwinkle desktop
        card: Color::from_rgba(0.980, 0.984, 0.992, 0.62), // frosted white glass
        card_hover: Color::from_rgba(1.000, 1.000, 1.000, 0.78),
        input_bg: Color::from_rgba(1.000, 1.000, 1.000, 0.45), // recessed glass field
        border: Color::from_rgba(1.000, 1.000, 1.000, 0.50),   // hairline
        border_subtle: Color::from_rgba(1.000, 1.000, 1.000, 0.35),
        border_bright: Color::from_rgba(1.000, 1.000, 1.000, 0.90), // top highlight
        fg: Color::from_rgb(0.114, 0.137, 0.200),                   // #1D2333 slate ink
        fg_dim: Color::from_rgb(0.333, 0.376, 0.478),               // #55607A
        fg_faint: Color::from_rgb(0.533, 0.576, 0.675),             // #8893AC
        accent: Color::from_rgb(0.039, 0.518, 1.000),               // #0A84FF Apple blue
        success: Color::from_rgb(0.184, 0.655, 0.404),              // #2FA767
        warning: Color::from_rgb(0.910, 0.604, 0.086),              // #E89A16
        danger: Color::from_rgb(0.898, 0.282, 0.302),               // #E5484D
        radius: 12.0,
        shadow: Color::from_rgba(0.06, 0.10, 0.20, 0.20), // soft diffuse
        shadow_offset: (0.0, 1.5), // settles toward the bottom, like a real panel
    }
}

/// Active design-token accessors. Tiny module of functions that forward to
/// the current `Palette`, so call sites keep reading `t::fg()`, `t::card()`,
/// `t::radius()`, … and don't need to know which style is active.
mod t {
    use super::{palette, Color};

    pub fn bg() -> Color {
        palette().bg
    }
    pub fn card() -> Color {
        palette().card
    }
    pub fn card_hover() -> Color {
        palette().card_hover
    }
    pub fn input_bg() -> Color {
        palette().input_bg
    }
    pub fn border() -> Color {
        palette().border
    }
    pub fn border_subtle() -> Color {
        palette().border_subtle
    }
    pub fn border_bright() -> Color {
        palette().border_bright
    }
    pub fn fg() -> Color {
        palette().fg
    }
    pub fn fg_dim() -> Color {
        palette().fg_dim
    }
    pub fn fg_faint() -> Color {
        palette().fg_faint
    }
    pub fn accent() -> Color {
        palette().accent
    }
    pub fn success() -> Color {
        palette().success
    }
    pub fn warning() -> Color {
        palette().warning
    }
    pub fn danger() -> Color {
        palette().danger
    }
    pub fn radius() -> f32 {
        palette().radius
    }
    pub fn shadow() -> Color {
        palette().shadow
    }
    pub fn shadow_offset() -> (f32, f32) {
        palette().shadow_offset
    }
}

// ─── Icons (lucide-style, 24px viewBox, stroke 1.75) ──────────────────────

mod icon {
    pub const COPY: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'><rect width='14' height='14' x='8' y='8' rx='2' ry='2'/><path d='M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2'/></svg>"#;
    pub const POWER: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'><path d='M18.36 6.64a9 9 0 1 1-12.73 0'/><line x1='12' x2='12' y1='2' y2='12'/></svg>"#;
    pub const POWER_OFF: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'><path d='M18.36 6.64a9 9 0 1 1-12.73 0'/><line x1='12' x2='12' y1='2' y2='12'/></svg>"#;
    pub const ACTIVITY: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'><path d='M22 12h-4l-3 9L9 3l-3 9H2'/></svg>"#;
    pub const TERMINAL: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'><polyline points='4 17 10 11 4 5'/><line x1='12' x2='20' y1='19' y2='19'/></svg>"#;
    pub const SWATCH: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'><circle cx='13.5' cy='6.5' r='.5' fill='currentColor'/><circle cx='17.5' cy='10.5' r='.5' fill='currentColor'/><circle cx='8.5' cy='7.5' r='.5' fill='currentColor'/><circle cx='6.5' cy='12.5' r='.5' fill='currentColor'/><path d='M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z'/></svg>"#;
    pub const ROTATE_CW: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'><path d='M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8'/><path d='M21 3v5h-5'/></svg>"#;

    // ─── Pixel campfire ──────────────────────────────────────────────────
    //
    // Four-frame flame flicker. The base (logs + stones) is the same on
    // every frame; only the flame pixels change so the loop reads as a
    // real fire breathing instead of a static icon. Picked by
    // `app.tick % 4` from `brand_icon`, which makes the animation sync
    // with the existing 1 Hz tick already driving the status pill.
    //
    // Pixel grid: 16×16, each cell is 1 SVG unit. `shape-rendering="crispEdges"`
    // disables anti-aliasing so the cells stay sharp when the SVG is
    // rasterised at small sizes — the whole point of the pixel look.

    const CAMPFIRE_BASE: &str = concat!(
        // ─── Logs ──────────────────────────────────────────────────────
        // Two slanted brown bars forming a lean-to.
        // row 11, cols 5..12
        "<rect x='5' y='11' width='7' height='1' fill='#6B3410'/>",
        "<rect x='6' y='12' width='7' height='1' fill='#6B3410'/>",
        // Embers between the logs — single bright pixel
        "<rect x='7' y='10' width='2' height='1' fill='#FFEC4A'/>",
        // ─── Stones ────────────────────────────────────────────────────
        // Three rows of grey stones framing the logs.
        // row 13
        "<rect x='4' y='13' width='1' height='1' fill='#3F4148'/>",
        "<rect x='5' y='13' width='6' height='1' fill='#6E7079'/>",
        "<rect x='11' y='13' width='1' height='1' fill='#3F4148'/>",
        // row 14
        "<rect x='3' y='14' width='1' height='1' fill='#6E7079'/>",
        "<rect x='4' y='14' width='8' height='1' fill='#6E7079'/>",
        "<rect x='12' y='14' width='1' height='1' fill='#3F4148'/>",
        // row 15 — bottom contact line
        "<rect x='3' y='15' width='9' height='1' fill='#3F4148'/>",
    );

    /// Frame 0 — neutral, slightly right-leaning. The "default" pose.
    /// Each rect is one non-overlapping pixel; rendering order is
    /// outer → inner so the bright yellow core always paints last.
    const CAMPFIRE_FLAME_0: &str = concat!(
        // Red outer.
        "<rect x='3' y='9' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='4' y='8' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='4' y='9' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='4' y='10' width='2' height='1' fill='#E63E2A'/>",
        // Orange body.
        "<rect x='5' y='6' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='5' y='7' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='5' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='6' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='7' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='8' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='6' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='7' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='8' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='6' y='10' width='2' height='1' fill='#FF8C2A'/>",
        // Yellow core (innermost).
        "<rect x='7' y='4' width='2' height='1' fill='#FFEC4A'/>",
        "<rect x='6' y='5' width='4' height='1' fill='#FFEC4A'/>",
        "<rect x='6' y='6' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='7' y='6' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='6' y='7' width='4' height='1' fill='#FFEC4A'/>",
        "<rect x='9' y='8' width='1' height='1' fill='#FFEC4A'/>",
    );

    /// Frame 1 — lean left, flame curling.
    const CAMPFIRE_FLAME_1: &str = concat!(
        // Red.
        "<rect x='2' y='9' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='3' y='8' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='3' y='9' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='3' y='10' width='2' height='1' fill='#E63E2A'/>",
        // Orange.
        "<rect x='4' y='6' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='4' y='7' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='4' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='5' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='6' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='7' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='5' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='6' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='7' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='5' y='10' width='2' height='1' fill='#FF8C2A'/>",
        // Yellow.
        "<rect x='5' y='4' width='2' height='1' fill='#FFEC4A'/>",
        "<rect x='4' y='5' width='4' height='1' fill='#FFEC4A'/>",
        "<rect x='5' y='6' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='6' y='6' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='5' y='7' width='4' height='1' fill='#FFEC4A'/>",
        "<rect x='8' y='8' width='1' height='1' fill='#FFEC4A'/>",
    );

    /// Frame 2 — tall reach. The biggest flame.
    const CAMPFIRE_FLAME_2: &str = concat!(
        // Red.
        "<rect x='4' y='9' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='5' y='10' width='2' height='1' fill='#E63E2A'/>",
        // Orange.
        "<rect x='4' y='7' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='4' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='5' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='6' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='7' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='5' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='6' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='7' y='9' width='1' height='1' fill='#FF8C2A'/>",
        // Yellow (tall core).
        "<rect x='7' y='3' width='2' height='1' fill='#FFEC4A'/>",
        "<rect x='6' y='4' width='4' height='1' fill='#FFEC4A'/>",
        "<rect x='6' y='5' width='4' height='1' fill='#FFEC4A'/>",
        "<rect x='5' y='6' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='6' y='6' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='7' y='6' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='5' y='7' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='6' y='7' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='7' y='7' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='8' y='7' width='2' height='1' fill='#FFEC4A'/>",
        "<rect x='8' y='8' width='2' height='1' fill='#FFEC4A'/>",
    );

    /// Frame 3 — lean right, flame curling the other way.
    const CAMPFIRE_FLAME_3: &str = concat!(
        // Red.
        "<rect x='11' y='9' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='10' y='8' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='10' y='9' width='1' height='1' fill='#E63E2A'/>",
        "<rect x='9' y='10' width='2' height='1' fill='#E63E2A'/>",
        // Orange.
        "<rect x='10' y='6' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='10' y='7' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='10' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='9' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='8' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='7' y='8' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='9' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='8' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='7' y='9' width='1' height='1' fill='#FF8C2A'/>",
        "<rect x='8' y='10' width='2' height='1' fill='#FF8C2A'/>",
        // Yellow.
        "<rect x='9' y='4' width='2' height='1' fill='#FFEC4A'/>",
        "<rect x='8' y='5' width='4' height='1' fill='#FFEC4A'/>",
        "<rect x='9' y='6' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='8' y='6' width='1' height='1' fill='#FFEC4A'/>",
        "<rect x='7' y='7' width='4' height='1' fill='#FFEC4A'/>",
        "<rect x='6' y='8' width='1' height='1' fill='#FFEC4A'/>",
    );

    /// Return the campfire SVG for the requested frame index (0..4). The
    /// `shape-rendering="crispEdges"` attribute disables anti-aliasing
    /// so each pixel stays sharp at small render sizes.
    pub fn campfire(frame: u32) -> String {
        let flame = match frame % 4 {
            1 => CAMPFIRE_FLAME_1,
            2 => CAMPFIRE_FLAME_2,
            3 => CAMPFIRE_FLAME_3,
            _ => CAMPFIRE_FLAME_0,
        };
        format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16' shape-rendering='crispEdges'>{base}{flame}</svg>",
            base = CAMPFIRE_BASE,
            flame = flame,
        )
    }

    /// Build the sparks layer for a given tick. Six sparks drift upward
    /// from above the flame and fade out before they reach the top edge.
    /// Each spark has a per-id phase offset (staggered) and a fixed
    /// column + colour, so the loop reads as a small irregular ember
    /// shower instead of a metronome.
    ///
    /// Cycle = 7 ticks: visible during the first 3 ticks (rows 2→0), then
    /// 4 ticks of "offscreen" (either rising up through the canvas, or
    /// waiting to be reborn). At any tick you see ~3 of the 6 sparks
    /// distributed across rows 0–2.
    fn sparks_for_tick(tick: u32) -> String {
        const XS: [u8; 6] = [4, 5, 7, 8, 9, 11];
        const PHASES: [u32; 6] = [0, 4, 2, 6, 1, 5];
        const COLORS: [&str; 6] = [
            "#FFEC4A", // yellow
            "#FF8C2A", // orange
            "#FFEC4A", "#E63E2A", // red ember
            "#FFEC4A", "#FF8C2A",
        ];
        const CYCLE: u32 = 7;
        const VISIBLE_LEN: u32 = 3;

        let mut out = String::with_capacity(192);
        for (i, &x) in XS.iter().enumerate() {
            let t = tick.wrapping_add(PHASES[i]) % CYCLE;
            if t < VISIBLE_LEN {
                // y descends: t=0 → row 2 (just above the flame),
                // t=2 → row 0 (about to vanish).
                let y = (VISIBLE_LEN - 1 - t) as u8;
                out.push_str(&format!(
                    "<rect x='{}' y='{}' width='1' height='1' fill='{}'/>",
                    x, y, COLORS[i]
                ));
            }
        }
        out
    }

    /// Idle-state hero: the same campfire PLUS a layer of drifting sparks
    /// above the flame. Used in the big 96×96 hero block on the idle
    /// screen; the small header icon stays spark-free to keep its footprint
    /// tight.
    pub fn idle_campfire(frame: u32) -> String {
        let flame = match frame % 4 {
            1 => CAMPFIRE_FLAME_1,
            2 => CAMPFIRE_FLAME_2,
            3 => CAMPFIRE_FLAME_3,
            _ => CAMPFIRE_FLAME_0,
        };
        format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16' shape-rendering='crispEdges'>{base}{flame}{sparks}</svg>",
            base = CAMPFIRE_BASE,
            flame = flame,
            sparks = sparks_for_tick(frame),
        )
    }
}

// ─── State ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum ServerStatus {
    Idle,
    Starting,
    Running { url: String, poll_url: String },
    Failed(String),
}

struct App {
    root_input: String,
    port_input: String,
    status: ServerStatus,
    qr_svg: Option<Arc<String>>,
    stats: StatsSnapshot,
    // RAII: drop to ask the server to stop.
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    // UI tick — drives blinking cursor + breathing dot.
    tick: u32,
    // Toast feedback ("copied", "started", etc.).
    toast: Option<(String, Instant)>,
    /// Live interface list backing the IP picker. Refreshed on demand.
    ip_options: Vec<IpOption>,
    /// Currently advertised interface (drives the displayed URL + QR). The
    /// server itself always binds to 0.0.0.0, so switching this is purely a
    /// display-side re-render — no restart needed.
    selected_ip: String,
    /// The native window's id, resolved once by a boot-time task. Needed to
    /// start a native window drag from the header brand area — macOS gives no
    /// drag region of its own once the title bar is hidden and the content
    /// view is full-size.
    window_id: Option<iced::window::Id>,
    /// Live log feed from the server — last 200 lines. Polled at 1 Hz.
    log_lines: Vec<LogLine>,
    /// Active WebSocket connections from `/api/connections`. Polled at 1 Hz.
    connections: Vec<ConnInfo>,
    /// Connection count captured at the moment the most recent log line
    /// was appended — used to render "+N peers" indicators in the log.
    log_prev_conn_count: usize,
    /// Modal visible: shows the full connection table.
    show_connections: bool,
    /// Active visual style (hacker / tech / retro / anime).
    style: GuiStyle,
}

#[derive(Debug, Clone)]
enum Message {
    RootChanged(String),
    PortChanged(String),
    /// Defensive no-op kept for source compatibility. The GUI no longer
    /// exposes a bind field — the server always binds to 0.0.0.0.
    #[allow(dead_code)]
    BindChanged(String),
    PickDirectory,
    DirectoryPicked(Option<PathBuf>),
    Start,
    ServerReady(Result<ServerReadyInfo, String>),
    Stop,
    CopyUrl,
    DismissError,
    Tick,
    PollStats,
    PollLog,
    PollConns,
    StatsUpdated(Result<StatsSnapshot, String>),
    LogUpdated(Result<Vec<LogLine>, String>),
    ConnectionsUpdated(Result<Vec<ConnInfo>, String>),
    /// Toggle the connection table modal.
    ToggleConnections,
    CloseConnections,
    /// Cycle to the next visual style (hacker → tech → retro → anime → …).
    CycleStyle,
    /// The floating toast has a click-to-dismiss affordance.
    DismissToast,
    /// User picked a different interface from the LAN IP dropdown.
    IpSelected(String),
    /// User pressed the refresh button on the LAN IP dropdown.
    RefreshIps,
    /// User pressed the header brand area — start a native window drag.
    DragWindow,
    /// The boot-time task resolved the native window id (macOS uses it to
    /// start window drags from the header once the title bar is hidden).
    WindowReady(Option<iced::window::Id>),
}

#[derive(Debug, Clone)]
struct ServerReadyInfo {
    url: String,
    poll_url: String,
    qr_svg: Arc<String>,
}

impl Drop for App {
    fn drop(&mut self) {
        // If a shutdown_tx is still held when the App is destroyed (e.g. by
        // iced tearing down the window), drop it here so the server thread
        // unblocks and exits. Without this, the tx would only drop when the
        // App goes out of scope anyway — but doing it explicitly surfaces
        // teardown in the tracing log and rules out one possible cause of
        // the "server didn't drain within 3s" warning firing unexpectedly.
        if let Some(tx) = self.shutdown_tx.take() {
            tracing::info!("App dropped — releasing server shutdown_tx");
            let _ = tx.send(());
        }
    }
}

impl App {
    fn new(args: Args) -> Self {
        let initial_root = args
            .root
            .or_else(|| std::env::current_dir().ok())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ip_options = list_ip_options();
        let selected_ip = default_ip(&ip_options);
        Self {
            root_input: initial_root,
            port_input: args.port.to_string(),
            status: ServerStatus::Idle,
            qr_svg: None,
            stats: StatsSnapshot::default(),
            shutdown_tx: None,
            tick: 0,
            toast: None,
            ip_options,
            selected_ip,
            window_id: None,
            log_lines: Vec::new(),
            connections: Vec::new(),
            log_prev_conn_count: 0,
            show_connections: false,
            style: GuiStyle::Hacker,
        }
    }

    fn is_running(&self) -> bool {
        matches!(
            self.status,
            ServerStatus::Running { .. } | ServerStatus::Starting
        )
    }

    /// True only when the server is actually accepting connections. Used
    /// by the polling subscription so we don't issue HTTP requests during
    /// the boot window (before `TcpListener::bind` completes) or after
    /// the user clicks STOP. Polling during those windows produces a wall
    /// of "Connection refused" warnings.
    fn is_actually_running(&self) -> bool {
        matches!(self.status, ServerStatus::Running { .. })
    }

    fn start(&mut self) -> Task<Message> {
        // Guard against double-click / focus quirk: if a server is already
        // booting or running, ignore the second START press. Otherwise the
        // new call would overwrite `self.shutdown_tx`, dropping the previous
        // sender — which silently signals the old server's `shutdown_rx` and
        // tears it down inside its 3-second graceful-shutdown window.
        if self.is_running() {
            return Task::none();
        }
        let root = PathBuf::from(self.root_input.trim());
        if root.as_os_str().is_empty() {
            return self.fail("> ERR: target directory not set");
        }
        if !root.is_dir() {
            return self.fail(&format!("> ERR: not a directory: {}", root.display()));
        }
        let port: u16 = match self.port_input.trim().parse() {
            Ok(p) => p,
            Err(_) => return self.fail("> ERR: port must be 1-65535"),
        };
        // Always bind to 0.0.0.0 (all interfaces). The displayed URL uses
        // the auto-detected LAN IP for human convenience; loopback always
        // works for the GUI's own polling. Binding to a specific IP was a
        // footgun — a stale/detached LAN IP left the server reachable from
        // nowhere (including 127.0.0.1) and produced a wall of
        // "Connection refused" warnings. Power users who need to constrain
        // binding should use the CLI (`qshare --host 192.168.1.5`).
        let cfg = ServerConfig {
            root,
            host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port,
            ..Default::default()
        };

        // The QR code / URL shown to the user must use the LAN IP so phones
        // on the same WiFi can reach it. But the GUI's own stats polling
        // should always use 127.0.0.1 — the GUI and server are on the same
        // machine, so localhost is always reachable regardless of bind addr
        // or routing quirks.
        // Pick up any interfaces that appeared since launch (e.g. an ethernet
        // cable plugged in after startup). Keeps the current selection when it
        // still exists.
        self.refresh_ip_options();
        let display_host = self.selected_ip.clone();
        let poll_host = "127.0.0.1".to_string();

        let (ready_tx, ready_rx) =
            tokio::sync::oneshot::channel::<Result<ServerReadyInfo, String>>();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        self.status = ServerStatus::Starting;
        self.shutdown_tx = Some(shutdown_tx);
        self.qr_svg = None;

        std::thread::Builder::new()
            .name("qshare-server".into())
            .spawn(move || {
                let t0 = std::time::Instant::now();
                // current_thread runtime is ~10x cheaper to build than multi_thread
                // (no extra OS threads, no signal-handler setup). The q-share server
                // is fully async and runs fine on a single-threaded executor.
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = ready_tx.send(Err(format!("runtime: {e}")));
                        return;
                    }
                };
                tracing::debug!("runtime built in {:?}", t0.elapsed());

                rt.block_on(async move {
                    let t1 = std::time::Instant::now();
                    let server = match Server::new(cfg.clone()) {
                        Ok(s) => s,
                        Err(e) => {
                            let _ = ready_tx.send(Err(format!("config: {e}")));
                            return;
                        }
                    };
                    let handle = match server.start().await {
                        Ok(h) => h,
                        Err(e) => {
                            let _ = ready_tx.send(Err(format!("server: {e}")));
                            return;
                        }
                    };
                    tracing::debug!("server started in {:?}", t1.elapsed());

                    let addr = handle.local_addr();
                    // Display URL uses LAN IP so phones can scan; poll URL is
                    // always 127.0.0.1 (same machine, no routing gotchas).
                    let url = format!("http://{}:{}", display_host, addr.port());
                    let poll_url = format!("http://{}:{}", poll_host, addr.port());
                    let t2 = std::time::Instant::now();
                    let svg = render_qr_svg(&url);
                    tracing::debug!("qr rendered in {:?}", t2.elapsed());
                    let _ = ready_tx.send(Ok(ServerReadyInfo {
                        url,
                        poll_url,
                        qr_svg: Arc::new(svg),
                    }));
                    let _ = shutdown_rx.await;
                    let _ = handle.shutdown().await;
                });
            })
            .expect("spawn server thread");

        Task::perform(
            async move {
                ready_rx
                    .await
                    .unwrap_or_else(|_| Err("server thread died".into()))
            },
            Message::ServerReady,
        )
    }

    fn fail(&mut self, msg: &str) -> Task<Message> {
        self.status = ServerStatus::Failed(msg.into());
        self.shutdown_tx = None;
        Task::none()
    }

    fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.qr_svg = None;
        self.status = ServerStatus::Idle;
        self.toast = None;
        // Reset live dashboard state so the next start shows a clean slate
        // and any in-flight poll failures (silently dropped by the result
        // handlers) can't leak into the post-restart view.
        self.stats = StatsSnapshot::default();
        self.log_lines.clear();
        self.connections.clear();
        self.log_prev_conn_count = 0;
    }

    /// Re-enumerate interfaces, preserving the current selection when it
    /// still exists, otherwise falling back to `default_ip`.
    fn refresh_ip_options(&mut self) {
        let new_options = list_ip_options();
        let prev = self.selected_ip.clone();
        let still_present = new_options.iter().any(|o| o.ip == prev);
        self.ip_options = new_options;
        if !still_present {
            self.selected_ip = default_ip(&self.ip_options);
        }
    }

    /// Rebuild the displayed URL + QR from `self.selected_ip`, keeping the
    /// bound port and the 127.0.0.1 poll_url. No-op unless actually running.
    /// The server never restarts: it already binds 0.0.0.0, so switching IPs
    /// is a pure display-side re-render.
    fn rebuild_running_url(&mut self) {
        // Extract (url, poll_url) first so the borrow of self.status ends
        // before we reassign self.status below (avoids aliasing the field).
        let current = match &self.status {
            ServerStatus::Running { url, poll_url } => Some((url.clone(), poll_url.clone())),
            _ => None,
        };
        if let Some((url, poll_url)) = current {
            // The displayed URL is always IPv4 "http://ip:port", so the last
            // ':' segment is the port. Never apply this to poll_url.
            let port = url.rsplit(':').next().unwrap_or("8888");
            let new_url = format!("http://{}:{}", self.selected_ip, port);
            self.status = ServerStatus::Running {
                url: new_url.clone(),
                poll_url,
            };
            self.qr_svg = Some(Arc::new(render_qr_svg(&new_url)));
            self.toast = Some(("url updated".to_string(), Instant::now()));
        }
    }
}

/// CSS hex (RRGGBB) for an iced colour. The QR renderer takes a plain
/// colour string; every palette colour used for the QR is opaque, so the
/// alpha channel is dropped and we never depend on 8-digit hex support.
fn color_hex(c: iced::Color) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8,
    )
}

fn render_qr_svg(url: &str) -> String {
    let code = match qrcode::QrCode::new(url.as_bytes()) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    // Modules follow the active palette: bright foreground on the app
    // background. `light_color` uses `t::bg()` so the QR's quiet zone melts
    // into the panel it sits on (the wrapper paints the same colour), while
    // `dark_color` uses `t::fg()` for maximum contrast. Every palette's bg
    // and fg are opaque and high-contrast, so the code stays scannable in
    // all five styles. Regenerated whenever the style switches (CycleStyle).
    code.render::<qrcode::render::svg::Color>()
        .quiet_zone(true)
        .min_dimensions(240, 240)
        .dark_color(qrcode::render::svg::Color(&color_hex(t::fg())))
        .light_color(qrcode::render::svg::Color(&color_hex(t::bg())))
        .build()
}

fn local_ip_hint() -> Option<String> {
    use std::net::UdpSocket;
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    Some(sock.local_addr().ok()?.ip().to_string())
}

/// One selectable network interface. The `Display` impl is what the IP
/// dropdown renders for each option (e.g. "en0 · 192.168.1.5").
#[derive(Debug, Clone, PartialEq, Eq)]
struct IpOption {
    name: String,
    ip: String,
}

impl std::fmt::Display for IpOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} · {}", self.name, self.ip)
    }
}

/// Enumerate every usable IPv4 interface. IPv6 is excluded — the displayed
/// URL is always IPv4. Tunnel/virtual/noise interfaces (utun/awdl/llw/anpi
/// etc.) are skipped. Loopback is kept but sorted last. Link-local
/// (169.254.x.x) is deliberately NOT filtered — picking the cable peer's
/// link-local IP is the whole point of this dropdown.
fn list_ip_options() -> Vec<IpOption> {
    let mut out = Vec::new();
    if let Ok(ifaces) = if_addrs::get_if_addrs() {
        for iface in ifaces {
            let ip = match iface.addr.ip() {
                std::net::IpAddr::V4(v4) => v4,
                std::net::IpAddr::V6(_) => continue,
            };
            let name = iface.name.as_str();
            let noise = name.starts_with("utun")
                || name.starts_with("awdl")
                || name.starts_with("llw")
                || name.starts_with("anpi")
                || name.starts_with("gif")
                || name.starts_with("stf")
                || name.starts_with("xnu")
                || name.starts_with("p2p");
            if noise {
                continue;
            }
            out.push(IpOption {
                name: iface.name,
                ip: ip.to_string(),
            });
        }
    }
    // Loopback last, then stable name order.
    out.sort_by(|a, b| {
        let a_lo = a.ip == "127.0.0.1";
        let b_lo = b.ip == "127.0.0.1";
        a_lo.cmp(&b_lo).then_with(|| a.name.cmp(&b.name))
    });
    // Guarantee loopback exists even if get_if_addrs failed entirely.
    if !out.iter().any(|o| o.ip == "127.0.0.1") {
        out.push(IpOption {
            name: "lo".into(),
            ip: "127.0.0.1".into(),
        });
    }
    out
}

/// Initial selection: prefer the default-route IP (`local_ip_hint`), else
/// the first non-loopback, else loopback.
fn default_ip(options: &[IpOption]) -> String {
    if let Some(hint) = local_ip_hint() {
        if options.iter().any(|o| o.ip == hint) {
            return hint;
        }
    }
    options
        .iter()
        .find(|o| o.ip != "127.0.0.1")
        .map(|o| o.ip.clone())
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

/// Minimal HTTP/1.1 GET for the local server's stats endpoint.
/// Uses std::net::TcpStream in a blocking task — avoids pulling in reqwest
/// just for one tiny GET. Times out after 2 s; failures are silent (UI just
/// shows the last good snapshot).
async fn poll_stats(base_url: String) -> Result<StatsSnapshot, String> {
    tokio::task::spawn_blocking(move || -> Result<StatsSnapshot, String> {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        // Strip scheme and path: keep just host:port.
        let without_scheme = base_url
            .strip_prefix("http://")
            .or_else(|| base_url.strip_prefix("https://"))
            .unwrap_or(&base_url);
        let host_port = without_scheme
            .split('/')
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "bad url".to_string())?
            .to_string();

        let mut stream = TcpStream::connect(&host_port)
            .map_err(|e| format!("connect {host_port}: {e}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;

        let req = format!(
            "GET /api/stats HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\nUser-Agent: qshare\r\n\r\n"
        );
        stream
            .write_all(req.as_bytes())
            .map_err(|e| format!("write: {e}"))?;
        stream.flush().ok();

        let mut buf = Vec::with_capacity(256);
        stream
            .read_to_end(&mut buf)
            .map_err(|e| format!("read: {e}"))?;

        let split = buf
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or_else(|| "no body separator".to_string())?;
        let body = &buf[split + 4..];

        // Be lenient about leading bytes (chunked transfer encoding etc) — if
        // there's a chunk-size header, just skip it. The streaming deserializer
        // also tolerates trailing chunked-encoding terminator bytes
        // (`\r\n0\r\n\r\n`) by stopping at the first JSON value.
        let body_start = body
            .iter()
            .position(|&b| b == b'{' || b == b'[')
            .unwrap_or(0);
        let mut de = serde_json::Deserializer::from_slice(&body[body_start..])
            .into_iter::<StatsSnapshot>();
        match de.next() {
            Some(Ok(v)) => Ok(v),
            Some(Err(e)) => Err(format!("parse: {e}")),
            None => Err("parse: empty body".into()),
        }
    })
    .await
    .map_err(|e| format!("join: {e}"))?
}

/// Generic JSON-GET helper for the log + connections endpoints.
/// Returns the parsed JSON body, or an error string on failure.
///
/// Uses a streaming `Deserializer` so it stops at the first JSON value and
/// silently discards any trailing bytes. HTTP/1.1 chunked encoding leaves
/// `\r\n0\r\n\r\n` after the body, which strict `serde_json::from_slice`
/// would reject as "trailing characters".
async fn poll_json<T: serde::de::DeserializeOwned + Send + 'static>(
    url: String,
) -> Result<T, String> {
    tokio::task::spawn_blocking(move || -> Result<T, String> {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let without_scheme = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(&url);
        let host_port = without_scheme
            .split('/')
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "bad url".to_string())?
            .to_string();
        let path = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .and_then(|s| s.find('/').map(|i| &s[i..]))
            .unwrap_or("/");

        let mut stream = TcpStream::connect(&host_port)
            .map_err(|e| format!("connect {host_port}: {e}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\nUser-Agent: qshare\r\n\r\n"
        );
        stream.write_all(req.as_bytes()).map_err(|e| format!("write: {e}"))?;
        stream.flush().ok();
        let mut buf = Vec::with_capacity(2048);
        stream.read_to_end(&mut buf).map_err(|e| format!("read: {e}"))?;
        let split = buf
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or_else(|| "no body separator".to_string())?;
        let body = &buf[split + 4..];
        let body_start = body
            .iter()
            .position(|&b| b == b'{' || b == b'[')
            .unwrap_or(0);
        let mut de = serde_json::Deserializer::from_slice(&body[body_start..])
            .into_iter::<T>();
        match de.next() {
            Some(Ok(v)) => Ok(v),
            Some(Err(e)) => Err(format!("parse: {e}")),
            None => Err("parse: empty body".into()),
        }
    })
    .await
    .map_err(|e| format!("join: {e}"))?
}

/// Wrapper used by the message handler for /api/log.
async fn poll_log_lines(base_url: String) -> Result<Vec<LogLine>, String> {
    #[derive(Deserialize)]
    struct Wrap {
        lines: Vec<LogLine>,
    }
    let url = format!("{}/api/log?tail=120", base_url);
    let wrap: Wrap = poll_json(url).await?;
    Ok(wrap.lines)
}

async fn poll_conns(base_url: String) -> Result<Vec<ConnInfo>, String> {
    #[derive(Deserialize)]
    struct Wrap {
        connections: Vec<ConnInfo>,
    }
    let url = format!("{}/api/connections", base_url);
    let wrap: Wrap = poll_json(url).await?;
    Ok(wrap.connections)
}

// ─── View helpers ──────────────────────────────────────────────────────────

fn title(app: &App) -> String {
    match &app.status {
        ServerStatus::Idle => "q-share :: idle".into(),
        ServerStatus::Starting => "q-share :: starting".into(),
        ServerStatus::Running { .. } => "q-share :: live".into(),
        ServerStatus::Failed(_) => "q-share :: error".into(),
    }
}

/// One line in the rolling server log — matches the JSON shape of
/// `qshare_core::conn::LogLine` exactly. Kept local so we don't pull
/// the whole `conn` module into the GUI's transitive deps.
#[derive(Debug, Clone, Deserialize)]
pub struct LogLine {
    pub ts_ms: u64,
    pub level: String,
    pub msg: String,
}

/// One active connection — matches `qshare_core::conn::ConnInfo`.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnInfo {
    pub id: String,
    pub ip: String,
    pub user_agent: String,
    pub watching: String,
    pub bytes_sent: u64,
    pub uptime_secs: u64,
    #[serde(default)]
    pub last_seen_unix_ms: u64,
}

fn mono_size<'a>(s: impl Into<String>, size: u16) -> text::Text<'a> {
    let s: String = s.into();
    text(s).size(size).font(iced::Font::MONOSPACE)
}

fn dim_text<'a>(s: impl Into<String>) -> text::Text<'a> {
    let s: String = s.into();
    text(s)
        .font(iced::Font::MONOSPACE)
        .size(11)
        .style(|_| text::Style {
            color: Some(t::fg_dim()),
        })
}

fn ico(svg_str: &'static str, size: u16, color: Color) -> Element<'static, Message> {
    svg(svg::Handle::from_memory(svg_str.as_bytes().to_vec()))
        .width(Length::Fixed(size as f32))
        .height(Length::Fixed(size as f32))
        .style(move |_theme, _status| svg::Style { color: Some(color) })
        .into()
}

/// Animated brand mark: pixel campfire whose flame flickers in sync
/// with `app.tick`. We rebuild the SVG string every redraw rather than
/// caching — the data is ~600 bytes and iced's redraw rate on this view
/// is the same as the tick itself (~1 Hz), so there's nothing to gain
/// from memoising.
fn brand_icon(app: &App, size: u16) -> Element<'_, Message> {
    let svg_str = icon::campfire(app.tick);
    svg(svg::Handle::from_memory(svg_str.into_bytes()))
        .width(Length::Fixed(size as f32))
        .height(Length::Fixed(size as f32))
        .into()
}

/// Idle-state hero: bigger version of the brand mark with drifting
/// sparks above the flame. Reserved for the idle view's 96px hero
/// block where there's room to read the embers — the 18px header icon
/// uses the spark-free `brand_icon` so the campfire silhouette stays
/// sharp at small sizes.
fn idle_brand_icon(app: &App, size: u16) -> Element<'_, Message> {
    let svg_str = icon::idle_campfire(app.tick);
    svg(svg::Handle::from_memory(svg_str.into_bytes()))
        .width(Length::Fixed(size as f32))
        .height(Length::Fixed(size as f32))
        .into()
}

fn card<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(12)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(t::card().into()),
            border: iced::Border {
                color: t::border(),
                width: 1.0,
                radius: t::radius().into(), // 0 for terminal/retro, rounded for anime
            },
            shadow: iced::Shadow {
                color: t::shadow(),
                offset: iced::Vector::new(t::shadow_offset().0, t::shadow_offset().1),
                blur_radius: 14.0,
            },
            ..Default::default()
        })
        .into()
}

fn stat_card<'a>(label: &'static str, value: String) -> Element<'a, Message> {
    card(
        column![
            dim_text(format!("[{}]", label)),
            vertical_space().height(4),
            mono_size(value, 16).style(|_| text::Style {
                color: Some(t::fg())
            }),
        ]
        .width(Length::Fill),
    )
}

fn status_pill<'a>(status: &ServerStatus, tick: u32) -> Element<'a, Message> {
    let (label, color) = match status {
        ServerStatus::Idle => ("IDLE", t::fg_faint()),
        ServerStatus::Starting => ("BOOT", t::warning()),
        ServerStatus::Running { .. } => ("LIVE", t::success()),
        ServerStatus::Failed(_) => ("ERR", t::danger()),
    };
    let dot_char = if tick.is_multiple_of(2) { "●" } else { "○" };
    container(
        row![mono_size(format!("[{} {}]", dot_char, label), 11)
            .style(move |_| text::Style { color: Some(color) }),]
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding::from([4, 10]))
    .style(|_theme| container::Style {
        background: Some(t::card_hover().into()),
        border: iced::Border {
            color: t::border(),
            width: 1.0,
            radius: t::radius().into(),
        },
        ..Default::default()
    })
    .into()
}

fn stop_button<'a>() -> Element<'a, Message> {
    button(
        row![
            ico(icon::POWER_OFF, 12, t::danger()),
            horizontal_space().width(6),
            mono_size("[STOP]", 11).style(|_| text::Style {
                color: Some(t::danger())
            }),
        ]
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding::from([4, 10]))
    .style(|_t, _s| button::Style {
        background: Some(t::card_hover().into()),
        border: iced::Border {
            color: t::danger(),
            width: 1.0,
            radius: t::radius().into(),
        },
        text_color: t::danger(),
        ..Default::default()
    })
    .on_press(Message::Stop)
    .into()
}

/// Style switcher — cycles hacker → tech → retro → anime on click. Sits in
/// the header next to the status pill so the current style is always
/// visible and one click away.
fn style_button(app: &App) -> Element<'_, Message> {
    button(
        row![
            ico(icon::SWATCH, 12, t::accent()),
            horizontal_space().width(6),
            mono_size(format!("[{}]", app.style.name()), 11).style(|_| text::Style {
                color: Some(t::accent()),
            }),
        ]
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding::from([4, 10]))
    .style(|_t, _s| button::Style {
        background: Some(t::card_hover().into()),
        border: iced::Border {
            color: t::border_bright(),
            width: 1.0,
            radius: t::radius().into(),
        },
        text_color: t::accent(),
        ..Default::default()
    })
    .on_press(Message::CycleStyle)
    .into()
}

// ─── Main view ─────────────────────────────────────────────────────────────

/// macOS draws its traffic lights in the top-left of the window. With a
/// full-size content view the header card extends underneath them, so the
/// brand row needs enough left padding to clear the three buttons.
#[cfg(target_os = "macos")]
const HEADER_LEFT_PAD: f32 = 80.0;
/// Other platforms keep their native title bar, so the header just uses its
/// regular horizontal padding.
#[cfg(not(target_os = "macos"))]
const HEADER_LEFT_PAD: f32 = 18.0;

fn view(app: &App) -> Element<'_, Message> {
    // With the macOS title bar hidden + full-size content view the header
    // card extends behind the traffic lights; the brand row doubles as the
    // window's drag handle (there's no native drag region left).
    let brand = mouse_area(
        row![
            brand_icon(app, 18),
            horizontal_space().width(8),
            mono_size("q-share", 13).style(|_| text::Style {
                color: Some(t::fg())
            }),
            mono_size("@local", 11).style(|_| text::Style {
                color: Some(t::fg_faint())
            }),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(Message::DragWindow);

    let header = container(
        row![
            brand,
            horizontal_space(),
            if app.is_running() {
                stop_button()
            } else {
                Space::new(0, 0).into()
            },
            horizontal_space().width(8),
            if app.is_running() {
                conn_button(app)
            } else {
                Space::new(0, 0).into()
            },
            horizontal_space().width(10),
            style_button(app),
            horizontal_space().width(10),
            status_pill(&app.status, app.tick),
        ]
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 8.0,
            right: 18.0,
            bottom: 8.0,
            left: HEADER_LEFT_PAD,
        }),
    )
    .style(|_theme| container::Style {
        background: Some(t::card().into()),
        border: iced::Border {
            color: t::border_subtle(),
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    });

    let body_content: Element<'_, Message> = match &app.status {
        ServerStatus::Running { url, .. } => running_view(app, url.clone()),
        ServerStatus::Starting => starting_view(app),
        ServerStatus::Failed(msg) => error_view(app, msg.clone()),
        ServerStatus::Idle => idle_view(app),
    };

    let body = container(body_content)
        .padding(iced::Padding::from([12, 18]))
        .width(Length::Fill)
        .height(Length::Fill);

    // The toast no longer lives in this column — it would push the page
    // content up while visible. It renders as a floating overlay layer in
    // `view_with_overlay` (see `toast_layer`) so it never affects layout.
    container(column![header, body])
        .style(|_theme| container::Style {
            background: Some(t::bg().into()),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Floating toast, pinned to the bottom-left of the window. Rendered as its
/// own stack layer (above the main view AND the connections modal) so it
/// never pushes other content around. The layer is a full-window container;
/// it has no interactive widget of its own, so clicks anywhere outside the
/// toast bubble fall through to whatever is underneath. Clicking the bubble
/// itself dismisses it early (`Message::DismissToast`).
fn toast_layer(app: &App) -> Option<Element<'_, Message>> {
    let (msg, when) = app.toast.as_ref()?;
    if when.elapsed() >= Duration::from_secs(2) {
        return None;
    }
    let bubble = mouse_area(
        container(
            row![mono_size(format!("> {}", msg), 11).style(|_| text::Style {
                color: Some(t::fg())
            }),]
            .align_y(Alignment::Center),
        )
        .padding(iced::Padding::from([6, 12]))
        .style(|_theme| container::Style {
            background: Some(t::card_hover().into()),
            border: iced::Border {
                color: t::border_bright(),
                width: 1.0,
                radius: t::radius().into(),
            },
            ..Default::default()
        }),
    )
    .on_press(Message::DismissToast);

    Some(
        container(bubble)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Start)
            .align_y(Alignment::End)
            .padding(iced::Padding {
                top: 0.0,
                right: 0.0,
                bottom: 16.0,
                left: 16.0,
            })
            .into(),
    )
}

// `view` returns the main content. We wrap with the connections modal and
// the floating toast here so both are always on top when visible.
// Extracted to a helper to keep `view` focused on the layout.
fn view_with_overlay(app: &App) -> Element<'_, Message> {
    let mut layers: Vec<Element<'_, Message>> = vec![view(app)];
    if app.show_connections {
        // Stack the modal overlay on top — modal captures all input via
        // its own mouse_area.
        layers.push(connections_modal(app));
    }
    // Toast floats above everything (including the modal), bottom-left.
    if let Some(el) = toast_layer(app) {
        layers.push(el);
    }
    iced::widget::stack(layers)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn idle_view(app: &App) -> Element<'_, Message> {
    // Animated pixel campfire — the same art we use in the header but
    // blown up to a 96×96 hero block. Painted with solid `█` glyphs in
    // our terminal palette rather than letter-shaped ASCII so the icon
    // reads as "fire" instead of "E". Four-frame flicker driven by
    // `app.tick` (1 Hz), so it animates every redraw.
    let banner = idle_brand_icon(app, 96);

    let tagline = mono_size("q-share", 16).style(|_| text::Style {
        color: Some(t::fg()),
    });
    let subtag = mono_size("// LAN file broadcaster", 11).style(|_| text::Style {
        color: Some(t::fg_faint()),
    });

    let hero = card(
        column![
            row![
                banner,
                horizontal_space().width(18),
                column![
                    tagline,
                    vertical_space().height(2),
                    subtag,
                    vertical_space().height(10),
                    mono_size("$ q-share --target <dir>", 12).style(|_| text::Style {
                        color: Some(t::fg_dim()),
                    }),
                    vertical_space().height(2),
                    mono_size("> awaiting operator input…", 10).style(|_| text::Style {
                        color: Some(t::fg_faint()),
                    }),
                ]
                .align_x(Alignment::Start),
            ]
            .align_y(Alignment::Center),
            vertical_space().height(18),
            setup_card(app, false),
        ]
        .width(Length::Fill),
    );

    column![hero].spacing(16).into()
}

fn starting_view(app: &App) -> Element<'_, Message> {
    let cursor = if app.tick.is_multiple_of(2) {
        "█"
    } else {
        " "
    };
    let spinner = match app.tick % 4 {
        0 => "│",
        1 => "/",
        2 => "─",
        _ => "\\",
    };
    let center = container(
        column![
            mono_size("// BOOT SEQUENCE", 11).style(|_| text::Style {
                color: Some(t::fg_faint()),
            }),
            vertical_space().height(14),
            mono_size(format!("[ {} ]", spinner), 28).style(|_| text::Style {
                color: Some(t::fg()),
            }),
            vertical_space().height(14),
            mono_size("$ initializing server", 13).style(|_| text::Style {
                color: Some(t::fg()),
            }),
            vertical_space().height(4),
            mono_size(format!("> binding port {}{}", app.port_input, cursor), 12).style(|_| {
                text::Style {
                    color: Some(t::fg_dim()),
                }
            }),
            vertical_space().height(2),
            mono_size(format!("> registering _qshare._tcp.local.{}", cursor), 12).style(|_| {
                text::Style {
                    color: Some(t::fg_dim()),
                }
            }),
        ]
        .align_x(Alignment::Start),
    )
    .padding(28)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_theme| container::Style {
        background: Some(t::card().into()),
        border: iced::Border {
            color: t::border(),
            width: 1.0,
            radius: t::radius().into(),
        },
        ..Default::default()
    });

    column![center]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

fn running_view(app: &App, url: String) -> Element<'_, Message> {
    // QR — modules follow the active palette (see render_qr_svg), no white
    // container. The SVG already paints its own background, so the wrapper
    // just adds a border in the theme's bright colour for framing. Compact
    // size so the running panel fits comfortably in a ~540 px tall window.
    let qr_block: Element<'_, Message> = if let Some(s) = app.qr_svg.as_ref() {
        container(
            svg(svg::Handle::from_memory(s.as_bytes().to_vec()))
                .width(Length::Fixed(140.0))
                .height(Length::Fixed(140.0)),
        )
        .padding(6)
        .style(|_theme| container::Style {
            background: Some(t::bg().into()),
            border: iced::Border {
                color: t::border_bright(),
                width: 1.0,
                radius: t::radius().into(),
            },
            ..Default::default()
        })
        .into()
    } else {
        container(Space::new(152, 152)).into()
    };

    let hero = card(
        row![
            qr_block,
            vertical_space().width(14),
            column![
                mono_size("// STATUS: LIVE", 11).style(|_| text::Style {
                    color: Some(t::success()),
                }),
                vertical_space().height(2),
                mono_size("broadcasting to LAN", 12).style(|_| text::Style {
                    color: Some(t::fg()),
                }),
                vertical_space().height(8),
                mono_size("$ ./scan", 10).style(|_| text::Style {
                    color: Some(t::fg_faint()),
                }),
                vertical_space().height(1),
                url_row(url.clone()),
                vertical_space().height(10),
                stat_grid(app),
            ]
            .width(Length::Fill)
            .align_x(Alignment::Start),
        ]
        .align_y(Alignment::Start),
    );

    column![hero, setup_card(app, true), log_panel(app)]
        .spacing(10)
        .into()
}

fn url_row(url: String) -> Element<'static, Message> {
    container(
        row![
            mono_size(">", 13).style(|_| text::Style {
                color: Some(t::fg_dim())
            }),
            horizontal_space().width(8),
            mono_size(url, 13).style(|_| text::Style {
                color: Some(t::fg())
            }),
            horizontal_space(),
            mouse_area(
                button(
                    row![
                        ico(icon::COPY, 11, t::fg_dim()),
                        horizontal_space().width(4),
                        mono_size("cp", 11).style(|_| text::Style {
                            color: Some(t::fg_dim())
                        }),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding(iced::Padding::from([4, 8]))
                .style(|_t, _s| button::Style {
                    background: Some(t::input_bg().into()),
                    border: iced::Border {
                        color: t::border(),
                        width: 1.0,
                        radius: t::radius().into(),
                    },
                    ..Default::default()
                })
                .on_press(Message::CopyUrl),
            )
            .on_press(Message::CopyUrl),
        ]
        .align_y(Alignment::Center)
        .padding(iced::Padding::from([5, 10])),
    )
    .style(|_theme| container::Style {
        background: Some(t::input_bg().into()),
        border: iced::Border {
            color: t::border(),
            width: 1.0,
            radius: t::radius().into(),
        },
        ..Default::default()
    })
    .into()
}

fn stat_grid(app: &App) -> Element<'_, Message> {
    // Four stats in a row — terminal dashboard feel. Each card is identical
    // width so the values line up vertically when one or more updates.
    //
    // CONN reads from `app.connections` (the actual connected-client list
    // used by the topbar CONN button) rather than `app.stats.active` (the
    // middleware's in-flight-request counter). The two diverge for any
    // long-lived connection — WebSocket clients especially — because they
    // are connected but not currently in a request handler. Users expect
    // "how many people are using my share?" to read the connection count,
    // and matching the topbar keeps both badges in sync.
    row![
        stat_card("CONN", app.connections.len().to_string()),
        horizontal_space().width(8),
        stat_card("XFER", human_bytes(app.stats.bytes_served)),
        horizontal_space().width(8),
        stat_card("ERR", app.stats.errors.to_string()),
        horizontal_space().width(8),
        stat_card("LOG", app.log_lines.len().to_string()),
    ]
    .into()
}

// ─── Log panel ─────────────────────────────────────────────────────────────
//
// Hacker-style terminal feed. Each line is monospace, prefixed with a short
// timestamp and a level tag, color-coded by severity:
//   info   → phosphor green (FG)
//   warn   → amber (WARNING)
//   error  → red (DANGER)
//   debug  → faint (FG_FAINT)
//
// Height-capped so it never takes more than ~30% of the panel — keeps the
// QR + URL area visible.

fn level_color(level: &str) -> Color {
    match level {
        "warn" => t::warning(),
        "error" => t::danger(),
        "debug" | "trace" => t::fg_faint(),
        _ => t::fg(),
    }
}

fn level_tag(level: &str) -> &'static str {
    match level {
        "warn" => "WARN",
        "error" => "ERR ",
        "debug" => "DBG ",
        "trace" => "TRC ",
        _ => "INFO",
    }
}

fn log_panel(app: &App) -> Element<'_, Message> {
    let header = row![
        ico(icon::TERMINAL, 12, t::fg()),
        horizontal_space().width(6),
        mono_size("// LIVE LOG", 11).style(|_| text::Style {
            color: Some(t::fg_dim())
        }),
        horizontal_space(),
        mono_size(format!("{} lines", app.log_lines.len()), 10).style(|_| text::Style {
            color: Some(t::fg_faint())
        }),
    ]
    .align_y(Alignment::Center)
    .padding(iced::Padding::from([5, 12]));

    let body: Element<'_, Message> = if app.log_lines.is_empty() {
        container(
            column![
                mono_size("// no events yet", 11).style(|_| text::Style {
                    color: Some(t::fg_faint()),
                }),
                vertical_space().height(2),
                mono_size("waiting for server output…", 10).style(|_| text::Style {
                    color: Some(t::fg_faint()),
                }),
            ]
            .padding(iced::Padding::from([10, 14])),
        )
        .into()
    } else {
        // Build a column of log lines. Each line is its own row so we get
        // clean alignment. Color comes from the parsed level.
        let mut col = column![].spacing(6).padding(iced::Padding::from([8, 14]));
        for line in &app.log_lines {
            let color = level_color(&line.level);
            let ts = format_ts_ms(line.ts_ms);
            let tag = level_tag(&line.level);
            let row_el = row![
                mono_size(format!("{}  ", ts), 10).style(|_| text::Style {
                    color: Some(t::fg_faint()),
                }),
                mono_size(format!("[{}] ", tag), 10)
                    .style(move |_| text::Style { color: Some(color) }),
                mono_size(line.msg.clone(), 11).style(move |_| text::Style { color: Some(color) }),
            ]
            .align_y(Alignment::Center);
            col = col.push(row_el);
        }
        scrollable(col)
            .id(scrollable::Id::new("log-scroll"))
            .height(Length::Fixed(110.0))
            // The scrollable deliberately has NO background of its own: the
            // previous design painted an opaque `input_bg` slab flush against
            // the panel's 1px border, which read as a separate layer floating
            // over the panel (and visually swallowing the thin border). Letting
            // it stay transparent makes the log sit *inside* the panel's card
            // background, with the panel border framing it cleanly.
            .style(|_theme, _status| scrollable::Style {
                container: container::Style {
                    background: None,
                    border: iced::Border {
                        color: t::border_subtle(),
                        width: 0.0,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                vertical_rail: scrollable::Rail {
                    background: None,
                    border: iced::Border {
                        color: t::border_subtle(),
                        width: 0.0,
                        ..Default::default()
                    },
                    scroller: scrollable::Scroller {
                        color: t::border_bright(),
                        border: iced::Border {
                            color: t::border(),
                            width: 1.0,
                            ..Default::default()
                        },
                    },
                },
                horizontal_rail: scrollable::Rail {
                    background: None,
                    border: iced::Border::default(),
                    scroller: scrollable::Scroller {
                        color: t::border_bright(),
                        border: iced::Border::default(),
                    },
                },
                gap: None,
            })
            .into()
    };

    container(column![header, body])
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(t::card().into()),
            border: iced::Border {
                color: t::border(),
                width: 1.0,
                radius: t::radius().into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render millisecond timestamp as `HH:MM:SS.mmm` in 24-hour format. Local
/// time, since this is a UI label for the operator.
fn format_ts_ms(ts_ms: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(ts_ms);
    let _ = now; // not used: we treat ts_ms as authoritative
    let total_secs = (ts_ms / 1000) % 86_400;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    let ms = ts_ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
}

// ─── Connections modal ─────────────────────────────────────────────────────
//
// Click the [CONN] button in the header to pop this up. It's a full-screen
// overlay with a terminal-styled table listing every active connection:
// id, ip, user_agent (truncated), watching path, bytes sent, uptime.
// Click outside or [X] to dismiss.

fn connections_modal(app: &App) -> Element<'_, Message> {
    let header = row![
        mono_size("// CONNECTIONS", 12).style(|_| text::Style {
            color: Some(t::fg()),
        }),
        horizontal_space(),
        mono_size(format!("{} active", app.connections.len()), 10).style(|_| text::Style {
            color: Some(t::fg_dim()),
        }),
        horizontal_space().width(10),
        button(mono_size("[X]", 11).style(|_| text::Style {
            color: Some(t::fg_dim())
        }))
        .padding(iced::Padding::from([2, 8]))
        .style(|_t, _s| button::Style {
            background: Some(t::card_hover().into()),
            border: iced::Border {
                color: t::border(),
                width: 1.0,
                radius: t::radius().into(),
            },
            ..Default::default()
        })
        .on_press(Message::CloseConnections),
    ]
    .align_y(Alignment::Center)
    .padding(iced::Padding::from([10, 16]));

    let table_head = row![
        mono_size("ID", 10)
            .width(Length::Fixed(70.0))
            .style(|_| text::Style {
                color: Some(t::fg_faint()),
            }),
        mono_size("IP", 10)
            .width(Length::Fixed(110.0))
            .style(|_| text::Style {
                color: Some(t::fg_faint()),
            }),
        mono_size("USER-AGENT", 10)
            .width(Length::Fill)
            .style(|_| text::Style {
                color: Some(t::fg_faint()),
            }),
        mono_size("WATCH", 10)
            .width(Length::Fixed(140.0))
            .style(|_| text::Style {
                color: Some(t::fg_faint()),
            }),
        mono_size("XFER", 10)
            .width(Length::Fixed(70.0))
            .style(|_| text::Style {
                color: Some(t::fg_faint()),
            }),
        mono_size("UP", 10)
            .width(Length::Fixed(50.0))
            .style(|_| text::Style {
                color: Some(t::fg_faint()),
            }),
    ]
    .padding(iced::Padding::from([6, 16]))
    .align_y(Alignment::Center);

    let body: Element<'_, Message> = if app.connections.is_empty() {
        container(
            column![
                vertical_space().height(20),
                mono_size("// no active connections", 12).style(|_| text::Style {
                    color: Some(t::fg_faint()),
                }),
                vertical_space().height(2),
                mono_size("open the URL in a browser to see peers here", 10).style(|_| {
                    text::Style {
                        color: Some(t::fg_faint()),
                    }
                }),
            ]
            .align_x(Alignment::Center),
        )
        .height(Length::Fixed(140.0))
        .into()
    } else {
        let mut col = column![].spacing(2).padding(iced::Padding::from([4, 8]));
        for c in &app.connections {
            let id_short: String = c.id.chars().take(8).collect();
            let ua = truncate_str(&c.user_agent, 32);
            let watching = if c.watching.is_empty() {
                "—".to_string()
            } else {
                c.watching.clone()
            };
            let row_el = row![
                mono_size(id_short, 10)
                    .width(Length::Fixed(70.0))
                    .style(|_| text::Style {
                        color: Some(t::fg_faint()),
                    }),
                mono_size(c.ip.clone(), 11)
                    .width(Length::Fixed(110.0))
                    .style(|_| text::Style {
                        color: Some(t::fg()),
                    }),
                mono_size(ua, 10)
                    .width(Length::Fill)
                    .style(|_| text::Style {
                        color: Some(t::fg_dim()),
                    }),
                mono_size(watching, 10)
                    .width(Length::Fixed(140.0))
                    .style(|_| text::Style {
                        color: Some(t::accent()),
                    }),
                mono_size(human_bytes(c.bytes_sent), 10)
                    .width(Length::Fixed(70.0))
                    .style(|_| text::Style {
                        color: Some(t::fg()),
                    }),
                mono_size(format_uptime(c.uptime_secs), 10)
                    .width(Length::Fixed(50.0))
                    .style(|_| text::Style {
                        color: Some(t::fg_dim()),
                    }),
            ]
            .padding(iced::Padding::from([4, 8]))
            .align_y(Alignment::Center);
            col = col.push(row_el);
        }
        scrollable(col)
            .height(Length::Fixed(180.0))
            .style(|_theme, _status| scrollable::Style {
                container: container::Style {
                    background: Some(t::input_bg().into()),
                    border: iced::Border {
                        color: t::border_subtle(),
                        width: 0.0,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                vertical_rail: scrollable::Rail {
                    background: Some(t::input_bg().into()),
                    border: iced::Border {
                        color: t::border_subtle(),
                        width: 0.0,
                        ..Default::default()
                    },
                    scroller: scrollable::Scroller {
                        color: t::border_bright(),
                        border: iced::Border {
                            color: t::border(),
                            width: 1.0,
                            ..Default::default()
                        },
                    },
                },
                horizontal_rail: scrollable::Rail {
                    background: Some(t::input_bg().into()),
                    border: iced::Border::default(),
                    scroller: scrollable::Scroller {
                        color: t::border_bright(),
                        border: iced::Border::default(),
                    },
                },
                gap: None,
            })
            .into()
    };

    let panel = container(column![header, table_head, body])
        .width(Length::Fixed(640.0))
        .style(|_theme| container::Style {
            background: Some(t::card().into()),
            border: iced::Border {
                color: t::border_bright(),
                width: 1.0,
                radius: t::radius().into(),
            },
            shadow: iced::Shadow {
                color: t::shadow(),
                offset: iced::Vector::new(t::shadow_offset().0, t::shadow_offset().1),
                blur_radius: 24.0,
            },
            ..Default::default()
        });

    // Centered overlay — captures the rest of the screen with a click-anywhere
    // scrim that dismisses the modal.
    let overlay = mouse_area(
        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(|_theme| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.7).into()),
                ..Default::default()
            }),
    )
    .on_press(Message::CloseConnections);

    overlay.into()
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn format_uptime(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Small clickable button that opens the connections modal. Shows a live
/// badge with the current connection count.
fn conn_button(app: &App) -> Element<'_, Message> {
    let n = app.connections.len();
    let label = if n == 0 {
        "[CONN 0]".to_string()
    } else {
        format!("[CONN {}]", n)
    };
    let color = if n == 0 { t::fg_dim() } else { t::accent() };
    button(
        row![
            ico(icon::ACTIVITY, 11, color),
            horizontal_space().width(4),
            mono_size(label, 11).style(move |_| text::Style { color: Some(color) }),
        ]
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding::from([5, 10]))
    .style(move |_t, _s| button::Style {
        background: Some(t::card_hover().into()),
        border: iced::Border {
            color: if n == 0 {
                t::border()
            } else {
                t::border_bright()
            },
            width: 1.0,
            radius: t::radius().into(),
        },
        ..Default::default()
    })
    .on_press(Message::ToggleConnections)
    .into()
}

fn error_view(app: &App, msg: String) -> Element<'_, Message> {
    let alert = card(
        column![row![
            mono_size("[!]", 14).style(|_| text::Style {
                color: Some(t::danger())
            }),
            horizontal_space().width(10),
            mono_size(msg, 12).style(|_| text::Style {
                color: Some(t::fg())
            }),
            horizontal_space(),
            mouse_area(
                button(mono_size("[X]", 11).style(|_| text::Style {
                    color: Some(t::fg_dim())
                }))
                .padding(iced::Padding::from([2, 6]))
                .style(|_t, _s| button::Style::default())
                .on_press(Message::DismissError),
            )
            .on_press(Message::DismissError),
        ]
        .align_y(Alignment::Center),]
        .width(Length::Fill),
    );

    column![alert, setup_card(app, false)].spacing(14).into()
}

fn setup_card(app: &App, disabled: bool) -> Element<'_, Message> {
    // When disabled (server is running), drop `.on_input`/`.on_press` so the
    // controls are no-op, and dim the styling to signal locked state.
    let label_color = if disabled { t::fg_faint() } else { t::fg_dim() };
    let value_color = if disabled { t::fg_dim() } else { t::fg() };
    let border_color = if disabled {
        t::border_subtle()
    } else {
        t::border()
    };
    let input_bg = if disabled {
        Color::from_rgba(t::input_bg().r, t::input_bg().g, t::input_bg().b, 0.5)
    } else {
        t::input_bg()
    };

    let root_field = row![
        mono_size("ROOT", 11)
            .width(Length::Fixed(46.0))
            .style(move |_| text::Style {
                color: Some(label_color)
            }),
        {
            let ti = text_input("path/to/share", &app.root_input)
                .padding(6)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .style(move |_theme, _status| text_input::Style {
                    background: input_bg.into(),
                    border: iced::Border {
                        color: border_color,
                        width: 1.0,
                        radius: t::radius().into(),
                    },
                    icon: t::fg_faint(),
                    placeholder: t::fg_faint(),
                    value: value_color,
                    selection: t::accent(),
                })
                .width(Length::Fill);
            if disabled {
                ti
            } else {
                ti.on_input(Message::RootChanged)
            }
        },
        horizontal_space().width(6),
        {
            let b = button(mono_size("[...]", 11).style(move |_| text::Style {
                color: Some(if disabled { t::fg_faint() } else { t::fg() }),
            }))
            .padding(iced::Padding::from([5, 10]))
            .style(move |_t, _s| button::Style {
                background: Some(input_bg.into()),
                border: iced::Border {
                    color: border_color,
                    width: 1.0,
                    radius: t::radius().into(),
                },
                ..Default::default()
            });
            if disabled {
                b
            } else {
                b.on_press(Message::PickDirectory)
            }
        },
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let port_field = row![
        mono_size("PORT", 11)
            .width(Length::Fixed(46.0))
            .style(move |_| text::Style {
                color: Some(label_color)
            }),
        {
            let ti = text_input("8888", &app.port_input)
                .padding(6)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .width(Length::Fixed(110.0))
                .style(move |_theme, _status| text_input::Style {
                    background: input_bg.into(),
                    border: iced::Border {
                        color: border_color,
                        width: 1.0,
                        radius: t::radius().into(),
                    },
                    icon: t::fg_faint(),
                    placeholder: t::fg_faint(),
                    value: value_color,
                    selection: t::accent(),
                });
            if disabled {
                ti
            } else {
                ti.on_input(Message::PortChanged)
            }
        },
        horizontal_space().width(14),
        mono_size("LAN", 11)
            .width(Length::Fixed(38.0))
            .style(move |_| text::Style {
                color: Some(label_color)
            }),
        // The IP picker stays interactive while running — switching the
        // advertised IP is a pure display-side re-render (the server binds
        // 0.0.0.0). `selected` must be an exact element of `options`
        // (PartialEq compares name AND ip), so it's looked up by ip rather
        // than constructed ad hoc.
        {
            let selected = app
                .ip_options
                .iter()
                .find(|o| o.ip == app.selected_ip)
                .cloned();
            // pick_list::Style and menu::Style have NO Default impl — every
            // field below is required.
            pick_list(app.ip_options.clone(), selected, |opt| {
                Message::IpSelected(opt.ip)
            })
            .placeholder("no interfaces")
            .width(Length::Fixed(240.0))
            .padding(iced::Padding::from([4, 8]))
            .font(iced::Font::MONOSPACE)
            .text_size(11)
            .style(|_t, _s| pick_list::Style {
                text_color: t::fg(),
                placeholder_color: t::fg_faint(),
                handle_color: t::fg_dim(),
                background: t::input_bg().into(),
                border: iced::Border {
                    color: t::border(),
                    width: 1.0,
                    radius: t::radius().into(),
                },
            })
            .menu_style(|_t| menu::Style {
                background: t::card().into(),
                border: iced::Border {
                    color: t::border_bright(),
                    width: 1.0,
                    radius: t::radius().into(),
                },
                text_color: t::fg(),
                selected_text_color: t::bg(),
                selected_background: t::accent().into(),
            })
        },
        horizontal_space().width(6),
        button(ico(icon::ROTATE_CW, 12, t::fg_dim()))
            .padding(iced::Padding::from([6, 8]))
            .style(|_t, _s| button::Style {
                background: Some(t::input_bg().into()),
                border: iced::Border {
                    color: t::border(),
                    width: 1.0,
                    radius: t::radius().into(),
                },
                ..Default::default()
            })
            .on_press(Message::RefreshIps),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let action_row = if disabled {
        // Running — the [STOP] button lives in the header, hint where it is.
        row![
            mono_size("// LOCKED  -  use [STOP] in header to edit config", 10).style(|_| {
                text::Style {
                    color: Some(t::fg_faint()),
                }
            })
        ]
        .align_y(Alignment::Center)
    } else {
        let label = if matches!(app.status, ServerStatus::Failed(_)) {
            "[RETRY]"
        } else {
            "[ START ]"
        };
        row![
            horizontal_space(),
            button(
                row![
                    ico(icon::POWER, 12, t::fg()),
                    horizontal_space().width(6),
                    mono_size(label, 12).style(|_| text::Style {
                        color: Some(t::fg())
                    }),
                ]
                .align_y(Alignment::Center),
            )
            .padding(iced::Padding::from([5, 14]))
            .style(|_t, _s| button::Style {
                background: Some(Color::from_rgba(t::fg().r, t::fg().g, t::fg().b, 0.06).into()),
                border: iced::Border {
                    color: t::fg(),
                    width: 1.0,
                    radius: t::radius().into(),
                },
                ..Default::default()
            })
            .on_press(Message::Start),
        ]
        .align_y(Alignment::Center)
    };

    card(
        column![
            mono_size("// CONFIG", 11).style(|_| text::Style {
                color: Some(t::fg_faint()),
            }),
            vertical_space().height(6),
            root_field,
            vertical_space().height(5),
            port_field,
            vertical_space().height(8),
            action_row,
        ]
        .width(Length::Fill),
    )
}

fn human_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", n, UNITS[0])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::RootChanged(v) => {
            app.root_input = v;
            Task::none()
        }
        Message::PortChanged(v) => {
            app.port_input = v;
            Task::none()
        }
        Message::IpSelected(ip) => {
            // The server already listens on 0.0.0.0 — switching the advertised
            // IP never restarts anything. Idle: just remember the choice (used
            // on the next Start). Running: re-render URL + QR in place.
            if app.selected_ip != ip {
                app.selected_ip = ip;
                app.rebuild_running_url();
            }
            Task::none()
        }
        Message::RefreshIps => {
            let prev = app.selected_ip.clone();
            app.refresh_ip_options();
            if app.selected_ip != prev {
                app.rebuild_running_url();
            }
            Task::none()
        }
        Message::WindowReady(id) => {
            // `or` keeps an already-known id: the boot-time task can only
            // resolve once, but if it ever re-fires we must not clobber the
            // real id with `None`.
            app.window_id = id.or(app.window_id);
            Task::none()
        }
        Message::DragWindow => match app.window_id {
            Some(id) => iced::window::drag(id),
            None => Task::none(),
        },
        Message::BindChanged(_v) => {
            // The GUI no longer exposes a bind field — the server always
            // binds to 0.0.0.0. This arm is kept only as a defensive no-op
            // in case any in-flight message somehow still references it.
            Task::none()
        }
        Message::PickDirectory => Task::perform(
            async {
                AsyncFileDialog::new()
                    .set_title("Pick a directory to share")
                    .pick_folder()
                    .await
                    .map(|h| h.path().to_path_buf())
            },
            Message::DirectoryPicked,
        ),
        Message::DirectoryPicked(Some(path)) => {
            app.root_input = path.to_string_lossy().into_owned();
            Task::none()
        }
        Message::DirectoryPicked(None) => Task::none(),
        Message::Start => app.start(),
        Message::ServerReady(Ok(info)) => {
            app.status = ServerStatus::Running {
                url: info.url,
                poll_url: info.poll_url,
            };
            app.qr_svg = Some(info.qr_svg);
            app.stats = StatsSnapshot::default();
            app.toast = Some(("server up".to_string(), Instant::now()));
            Task::none()
        }
        Message::ServerReady(Err(e)) => {
            app.status = ServerStatus::Failed(e);
            app.shutdown_tx = None;
            Task::none()
        }
        Message::Stop => {
            app.stop();
            app.toast = Some(("server stopped".to_string(), Instant::now()));
            Task::none()
        }
        Message::CopyUrl => {
            if let ServerStatus::Running { url, .. } = &app.status {
                let _ = copy_to_clipboard(url);
                app.toast = Some(("url copied".to_string(), Instant::now()));
            }
            Task::none()
        }
        Message::DismissError => {
            app.status = ServerStatus::Idle;
            Task::none()
        }
        Message::Tick => {
            app.tick = app.tick.wrapping_add(1);
            Task::none()
        }
        Message::PollStats => {
            // Only poll while the server is running and we have its poll URL.
            let url = match &app.status {
                ServerStatus::Running { poll_url, .. } => poll_url.clone(),
                _ => return Task::none(),
            };
            Task::perform(poll_stats(url), Message::StatsUpdated)
        }
        Message::PollLog => {
            let url = match &app.status {
                ServerStatus::Running { poll_url, .. } => poll_url.clone(),
                _ => return Task::none(),
            };
            Task::perform(poll_log_lines(url), Message::LogUpdated)
        }
        Message::PollConns => {
            let url = match &app.status {
                ServerStatus::Running { poll_url, .. } => poll_url.clone(),
                _ => return Task::none(),
            };
            Task::perform(poll_conns(url), Message::ConnectionsUpdated)
        }
        Message::StatsUpdated(Ok(snapshot)) => {
            if app.is_actually_running() {
                app.stats = snapshot;
            }
            Task::none()
        }
        Message::StatsUpdated(Err(e)) => {
            // If the user clicked Stop while this poll was in flight, the
            // failure is expected — silently drop. Otherwise surface it
            // (silent drops made the user think stats weren't incrementing
            // when actually polling was broken).
            if app.is_actually_running() {
                tracing::warn!("stats poll failed: {e}");
            }
            Task::none()
        }
        Message::LogUpdated(Ok(lines)) => {
            if app.is_actually_running() {
                app.log_lines = lines;
            }
            Task::none()
        }
        Message::LogUpdated(Err(_)) => Task::none(),
        Message::ConnectionsUpdated(Ok(conns)) => {
            if app.is_actually_running() {
                app.connections = conns;
            }
            Task::none()
        }
        Message::ConnectionsUpdated(Err(e)) => {
            if app.is_actually_running() {
                tracing::debug!("connections poll failed: {e}");
            }
            Task::none()
        }
        Message::ToggleConnections => {
            app.show_connections = !app.show_connections;
            Task::none()
        }
        Message::CloseConnections => {
            app.show_connections = false;
            Task::none()
        }
        Message::CycleStyle => {
            app.style = app.style.next();
            // The view reads `CURRENT_STYLE` on every redraw via the `t::*()`
            // accessors, so keep the global in sync with the enum.
            CURRENT_STYLE.store(app.style as u8, Ordering::Relaxed);
            // The QR is a cached SVG rendered at server-start with the old
            // palette — re-render it now so it follows the new style too.
            if let ServerStatus::Running { url, .. } = &app.status {
                app.qr_svg = Some(Arc::new(render_qr_svg(url)));
            }
            Task::none()
        }
        Message::DismissToast => {
            app.toast = None;
            Task::none()
        }
    }
}

fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    let prog = if cfg!(target_os = "macos") {
        Some("pbcopy")
    } else if cfg!(target_os = "windows") {
        Some("clip")
    } else {
        Some("xclip")
    };
    if let Some(p) = prog {
        use std::io::Write;
        use std::process::{Command, Stdio};
        if let Ok(mut child) = Command::new(p).stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
        }
    }
    Ok(())
}

fn subscription(app: &App) -> Subscription<Message> {
    // Tick is always active — drives the breathing status dot, the
    // boot-screen blinking cursor, AND the idle-screen campfire flicker
    // + drifting sparks. Polling (stats/log/conns) is gated on
    // `is_actually_running` so we don't fire HTTP requests during the
    // boot window before `TcpListener::bind` finishes, and we stop the
    // instant the user clicks STOP — preventing a flood of
    // "Connection refused" warnings once the server is dead.
    let tick = iced::time::every(Duration::from_millis(500)).map(|_| Message::Tick);

    if app.is_actually_running() {
        Subscription::batch(vec![
            tick,
            iced::time::every(Duration::from_secs(2)).map(|_| Message::PollStats),
            iced::time::every(Duration::from_secs(1)).map(|_| Message::PollLog),
            iced::time::every(Duration::from_secs(1)).map(|_| Message::PollConns),
        ])
    } else {
        tick
    }
}

fn theme(app: &App) -> Theme {
    // Retro Win95 and mac glass want the light OS theme (iced's Light tints
    // native widgets like scrollbars); our own palette still drives every panel.
    match app.style {
        GuiStyle::Retro | GuiStyle::Mac => Theme::Light,
        _ => Theme::Dark,
    }
}

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();

    let args = Args::parse();
    let boot = App::new(args);

    // Window chrome. iced's builder `.window()` REPLACES the whole window
    // Settings, so size/position must be baked in here — any chained
    // `.window_size()`/`.centered()` after it would be silently dropped.
    // `mut` is only consumed by the macOS block below; on other platforms
    // (e.g. ubuntu CI) it's unused, so allow it there rather than lint-fail.
    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut window_settings = iced::window::Settings {
        size: Size::new(820.0, 540.0),
        position: iced::window::Position::Centered,
        ..Default::default()
    };
    // macOS: hide the native title text, make the title bar transparent and
    // let the window content extend behind it, so the active style's `t::bg()`
    // paints the whole window (title bar included) and the system chrome color
    // follows the theme. The header brand row doubles as the drag handle.
    #[cfg(target_os = "macos")]
    {
        window_settings.platform_specific = iced::window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        };
    }

    iced::application(title, update, view_with_overlay)
        .theme(theme)
        .subscription(subscription)
        .window(window_settings)
        // Ask the runtime for the window id once it exists, so the header
        // brand can start a native window drag.
        .run_with(move || (boot, iced::window::get_oldest().map(Message::WindowReady)))
}
