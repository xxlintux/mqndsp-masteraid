//! Master power strip: pilot jewel, toggle switch and legend.

use nih_plug_vizia::vizia::vg::{Canvas, Color, LineCap, Paint, Path, Renderer};

use super::palette::{silk_muted, silk_white};
use super::text::draw_text_centered;
use crate::gui::theme::ThemeFonts;

pub fn draw_master_power<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    is_on: bool,
    fonts: ThemeFonts,
) {
    let mut box_p = Path::new();
    box_p.rounded_rect(cx - 70.0, cy - 23.0, 140.0, 46.0, 8.0);
    canvas.fill_path(&box_p, &Paint::color(Color::rgb(0x0B, 0x0B, 0x0D)));
    let mut bs = Paint::color(Color::rgb(0x26, 0x26, 0x2C));
    bs.set_line_width(1.2);
    canvas.stroke_path(&box_p, &bs);

    // Pilot jewel
    let mut jewel = Path::new();
    jewel.circle(cx - 36.0, cy, 10.0);
    if is_on {
        let j_grad = Paint::radial_gradient(
            cx - 38.0,
            cy - 2.0,
            1.5,
            10.5,
            Color::rgb(0xFF, 0x99, 0xCC),
            Color::rgb(0x4A, 0x00, 0x25),
        );
        canvas.fill_path(&jewel, &j_grad);
    } else {
        canvas.fill_path(&jewel, &Paint::color(Color::rgb(0x22, 0x10, 0x18)));
    }
    let mut j_rim = Path::new();
    j_rim.circle(cx - 36.0, cy, 12.0);
    let mut jrs = Paint::color(Color::rgb(0x38, 0x38, 0x40));
    jrs.set_line_width(0.8);
    canvas.stroke_path(&j_rim, &jrs);

    // Toggle switch housing
    let mut sw = Path::new();
    sw.rounded_rect(cx + 4.0, cy - 11.0, 52.0, 22.0, 4.0);
    canvas.fill_path(&sw, &Paint::color(Color::rgb(0x26, 0x26, 0x2C)));
    let mut sw_c = Path::new();
    sw_c.circle(cx + 20.0, cy, 7.0);
    canvas.fill_path(&sw_c, &Paint::color(Color::rgb(0x18, 0x18, 0x1C)));

    let mut bat = Path::new();
    bat.move_to(cx + 20.0, cy);
    if is_on {
        bat.line_to(cx + 38.0, cy - 6.0);
    } else {
        bat.line_to(cx + 10.0, cy + 6.0);
    }
    let mut bp = Paint::color(silk_white());
    bp.set_line_width(3.2);
    bp.set_line_cap(LineCap::Round);
    canvas.stroke_path(&bat, &bp);

    draw_text_centered(
        canvas,
        cx,
        cy + 38.0,
        "MASTER POWER",
        fonts.bold_paint(silk_muted(), 9.5),
    );
}
