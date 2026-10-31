//! Console palette. Shared colour tokens so every primitive draws from the same inks.

use nih_plug_vizia::vizia::vg::Color;
use std::sync::atomic::{AtomicBool, Ordering};

static IS_LIGHT: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn set_light_mode(light: bool) {
    IS_LIGHT.store(light, Ordering::Relaxed);
}

#[inline]
pub fn is_light() -> bool {
    IS_LIGHT.load(Ordering::Relaxed)
}

#[inline]
pub fn accent_magenta() -> Color {
    Color::rgb(0xFF, 0x00, 0x7F)
}

#[inline]
pub fn dseg_active_pink() -> Color {
    Color::rgb(0xFF, 0x24, 0x91)
}

#[inline]
pub fn silk_muted() -> Color {
    if is_light() {
        Color::rgb(0x60, 0x62, 0x6C)
    } else {
        Color::rgb(0x8A, 0x8A, 0x92)
    }
}

#[inline]
pub fn silk_white() -> Color {
    if is_light() {
        Color::rgb(0x1E, 0x1F, 0x24)
    } else {
        Color::rgb(0xEC, 0xE9, 0xE1)
    }
}

