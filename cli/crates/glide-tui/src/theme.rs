//! Palette from `docs/a2ui-cli-design.md`: dark terminal chrome, one warm
//! accent. Saffron is reserved for focus and the caret so the eye always knows
//! where it is.

use ratatui::style::Color;

pub const SHELL: Color = Color::Rgb(0x0e, 0x0f, 0x12);
pub const BAR: Color = Color::Rgb(0x19, 0x1c, 0x22);
pub const BODY: Color = Color::Rgb(0xc9, 0xcd, 0xd6);
pub const DIM: Color = Color::Rgb(0x6b, 0x72, 0x80);
pub const PROMPT: Color = Color::Rgb(0x7f, 0xae, 0x6b);
pub const SAFFRON: Color = Color::Rgb(0xd9, 0x77, 0x2f);
pub const ERROR: Color = Color::Rgb(0xd0, 0x5a, 0x4e);
pub const INK: Color = Color::Rgb(0xf7, 0xf8, 0xfa);
