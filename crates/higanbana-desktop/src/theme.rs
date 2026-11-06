//! Cores do higanbana (as mesmas de `src/styles/app.css` do frontend web).

use gpui::{Rgba, rgb, rgba};

pub fn bg() -> Rgba {
    rgb(0x07070a)
}

pub fn canvas() -> Rgba {
    rgb(0x0b0b10)
}

pub fn surface() -> Rgba {
    rgb(0x111118)
}

pub fn elevated() -> Rgba {
    rgb(0x18181f)
}

pub fn line() -> Rgba {
    white_a(0.08)
}

pub fn line_strong() -> Rgba {
    white_a(0.14)
}

pub fn fg() -> Rgba {
    rgb(0xededef)
}

pub fn muted() -> Rgba {
    rgb(0x9a9ea8)
}

pub fn faint() -> Rgba {
    rgb(0x767b85)
}

pub fn brand() -> Rgba {
    rgb(0xe8283c)
}

pub fn brand_ink() -> Rgba {
    rgb(0xff7a85)
}

/// Destaque das telas (no web, vinha da cor da capa; aqui, a da marca).
pub fn accent() -> Rgba {
    brand()
}

pub fn accent_ink() -> Rgba {
    brand_ink()
}

pub fn accent_soft() -> Rgba {
    rgba(0xe8283c26)
}

pub fn danger() -> Rgba {
    rgb(0xf87171)
}

pub fn ok() -> Rgba {
    rgb(0x4ade80)
}

pub fn white() -> Rgba {
    rgb(0xffffff)
}

pub fn white_a(a: f32) -> Rgba {
    Rgba { r: 1.0, g: 1.0, b: 1.0, a }
}

pub fn black_a(a: f32) -> Rgba {
    Rgba { r: 0.0, g: 0.0, b: 0.0, a }
}
