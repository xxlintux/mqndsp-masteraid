//! Coloration cassette: the amplifier/transformer selector pair plus the harmonic-drive
//! AUTO/PUSH buttons.

#![allow(clippy::too_many_arguments)]

use nih_plug_vizia::vizia::vg::{Canvas, Color, Paint, Path, Renderer};

use super::buttons::{
    draw_segmented_selector, draw_vintage_push_button, draw_vintage_push_button_fade,
};
use super::palette::{is_light, silk_muted};
use super::text::draw_text_centered;
use crate::gui::theme::ThemeFonts;

/// What the AUTO legend button on a coloration cassette is showing right now.
#[derive(Clone, Copy, Debug)]
pub enum AutoLamp {
    /// AUTO is off.
    Off,
    /// AUTO is engaged but nothing has been measured for this model yet. Steady, and dimmer than
    /// the learned breath, so "on" is distinct from "locked in".
    On,
    /// A measurement is running. The lamp blinks hard, like a blinker, so "working" cannot be
    /// mistaken for "on". `on` is the current phase of the blink.
    Learning { on: bool },
    /// Measured, frozen, and in use. The backlight fades slowly in and out: the lamp never goes
    /// dark, so this reads as "locked in", not "still working".
    Engaged { fade: f32 },
}

pub fn draw_coloration_cassette<T: Renderer>(
    canvas: &mut Canvas<T>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    amp_alias: &str,
    trans_alias: &str,
    match_active: bool,
    push_state: i32,
    auto_lamp: AutoLamp,
    fonts: ThemeFonts,
) {
    let light = is_light();
    let mut sub = Path::new();
    sub.rounded_rect(x, y, w, h, 12.0);
    let grad = if light {
        Paint::linear_gradient(
            x,
            y,
            x,
            y + h,
            Color::rgb(0xE0, 0xDC, 0xD5),
            Color::rgb(0xCC, 0xC8, 0xC0),
        )
    } else {
        Paint::linear_gradient(
            x,
            y,
            x,
            y + h,
            Color::rgb(0x11, 0x11, 0x14),
            Color::rgb(0x09, 0x09, 0x0B),
        )
    };
    canvas.fill_path(&sub, &grad);
    let mut s1 = Paint::color(if light {
        Color::rgb(0xB0, 0xAC, 0xA4)
    } else {
        Color::rgb(0x26, 0x26, 0x2C)
    });
    s1.set_line_width(1.2);
    canvas.stroke_path(&sub, &s1);

    let mut inner = Path::new();
    inner.rounded_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 11.0);
    let mut s2 = Paint::color(if light {
        Color::rgba(0xFF, 0xFF, 0xFF, 160)
    } else {
        Color::rgba(0x38, 0x38, 0x40, 90)
    });
    s2.set_line_width(0.7);
    canvas.stroke_path(&inner, &s2);

    // 4 Corner slotted screws
    for (sx, sy) in [
        (x + 10.0, y + 10.0),
        (x + w - 10.0, y + 10.0),
        (x + 10.0, y + h - 10.0),
        (x + w - 10.0, y + h - 10.0),
    ] {
        let mut sc = Path::new();
        sc.circle(sx, sy, 3.2);
        canvas.fill_path(&sc, &Paint::color(if light {
            Color::rgb(0xBA, 0xB6, 0xAE)
        } else {
            Color::rgb(0x34, 0x34, 0x3C)
        }));
        let mut sl = Path::new();
        sl.move_to(sx - 2.2, sy);
        sl.line_to(sx + 2.2, sy);
        let mut sp = Paint::color(if light {
            Color::rgb(0x8C, 0x88, 0x80)
        } else {
            Color::rgb(0x11, 0x11, 0x14)
        });
        sp.set_line_width(0.8);
        canvas.stroke_path(&sl, &sp);
    }

    draw_text_centered(
        canvas,
        x + w * 0.5,
        y + 16.0,
        "COLORATION",
        fonts.bold_paint(silk_muted(), 9.5),
    );
    let mut div = Path::new();
    div.move_to(x + 14.0, y + 26.0);
    div.line_to(x + w - 14.0, y + 26.0);
    let mut dp = Paint::color(if light {
        Color::rgb(0xBA, 0xB6, 0xAE)
    } else {
        Color::rgb(0x26, 0x26, 0x2C)
    });
    dp.set_line_width(1.0);
    canvas.stroke_path(&div, &dp);

    // 1. Amplifier Selector (concise, no redundant label)
    draw_segmented_selector(canvas, x + w * 0.5, y + 60.0, "", amp_alias, fonts);

    // 2. Transformer Selector (concise, no redundant label)
    draw_segmented_selector(canvas, x + w * 0.5, y + 116.0, "", trans_alias, fonts);

    // 3. Vintage push button trio: MATCH, AUTO, PUSH (classic square buttons)
    let cy_btns = y + 192.0;
    let btn_size = 40.0;
    let match_cx = x + 36.0;
    let auto_cx = x + w * 0.5;
    let push_cx = x + w - 36.0;

    draw_vintage_push_button(
        canvas,
        match_cx,
        cy_btns,
        "MATCH",
        if match_active { 1 } else { 0 },
        btn_size,
        btn_size,
        fonts,
    );

    let auto_lit = match auto_lamp {
        AutoLamp::Off => 0.0,
        AutoLamp::On => 0.62,
        AutoLamp::Learning { on } => {
            if on {
                1.0
            } else {
                0.0
            }
        }
        AutoLamp::Engaged { fade } => fade,
    };
    draw_vintage_push_button_fade(
        canvas, auto_cx, cy_btns, "AUTO", auto_lit, btn_size, btn_size, fonts,
    );

    draw_vintage_push_button(
        canvas,
        push_cx,
        cy_btns,
        "PUSH",
        push_state,
        btn_size,
        btn_size,
        fonts,
    );
}
