//! Two-position model selector: a recessed well with a sliding thumb and two legends.

#![allow(clippy::too_many_arguments)]

use nih_plug_vizia::vizia::vg::{Canvas, Color, Paint, Path, Renderer};

use super::palette::{accent_magenta, is_light};
use super::text::draw_text_centered;
use crate::gui::theme::ThemeFonts;

pub fn draw_model_select<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    opt_a: &str,
    opt_b: &str,
    active_b: bool,
    width: f32,
    fonts: ThemeFonts,
) {
    draw_model_select_lit(canvas, cx, cy, opt_a, opt_b, active_b, width, fonts, true);
}

pub fn draw_model_select_lit<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    opt_a: &str,
    opt_b: &str,
    active_b: bool,
    width: f32,
    fonts: ThemeFonts,
    lit: bool,
) {
    let w = width;
    let h = 28.0;
    let x = cx - w * 0.5;
    let y = cy - h * 0.5;
    let btn_w = w * 0.5 - 2.0;

    let light = is_light();
    let mut well = Path::new();
    well.rounded_rect(x, y, w, h, 8.0);
    let (well_fill, well_stroke) = if light {
        (Color::rgb(0xB8, 0xB5, 0xAE), Color::rgb(0x96, 0x93, 0x8C))
    } else {
        (Color::rgb(0x0B, 0x0B, 0x0D), Color::rgb(0x26, 0x26, 0x2C))
    };
    canvas.fill_path(&well, &Paint::color(well_fill));
    let mut well_s = Paint::color(well_stroke);
    well_s.set_line_width(1.2);
    canvas.stroke_path(&well, &well_s);

    let thumb_x = if active_b { x + w * 0.5 } else { x + 2.0 };
    let mut thumb = Path::new();
    thumb.rounded_rect(thumb_x, y + 2.0, btn_w, h - 4.0, 6.0);
    let thumb_grad = if light {
        Paint::linear_gradient(
            thumb_x,
            y + 2.0,
            thumb_x,
            y + h - 2.0,
            Color::rgb(0xFD, 0xFD, 0xFD),
            Color::rgb(0xD8, 0xD8, 0xDE),
        )
    } else {
        Paint::linear_gradient(
            thumb_x,
            y + 2.0,
            thumb_x,
            y + h - 2.0,
            Color::rgb(0x3A, 0x3A, 0x40),
            Color::rgb(0x20, 0x20, 0x24),
        )
    };
    canvas.fill_path(&thumb, &thumb_grad);
    let mut thumb_s = Paint::color(if light {
        Color::rgb(0xB4, 0xB4, 0xBC)
    } else {
        Color::rgb(0x38, 0x38, 0x40)
    });
    thumb_s.set_line_width(0.8);
    canvas.stroke_path(&thumb, &thumb_s);

    let unlit_txt = if light {
        Color::rgb(0x60, 0x62, 0x6C)
    } else {
        Color::rgba(0x8A, 0x8A, 0x92, 140)
    };
    let col_a = if !active_b && lit {
        accent_magenta()
    } else {
        unlit_txt
    };
    let col_b = if active_b && lit {
        accent_magenta()
    } else {
        unlit_txt
    };
    draw_text_centered(
        canvas,
        x + w * 0.25,
        cy,
        opt_a,
        fonts.bold_paint(col_a, 10.5),
    );
    draw_text_centered(
        canvas,
        x + w * 0.75,
        cy,
        opt_b,
        fonts.bold_paint(col_b, 10.5),
    );
}
