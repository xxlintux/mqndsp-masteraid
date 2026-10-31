//! Pro-Audio 2-Position Flat Paddle Toggle Switch Primitive
//! Models authentic vintage/mastering hardware (Carling 2M1 / C&K 7000 flatted paddle toggle).
//!
//! Two latching positions (90° total travel, ±45° throw):
//! - UP (+45° tilted throw): SHELF (top label)
//! - DOWN (-45° tilted throw): BELL (bottom label)

#![allow(clippy::too_many_arguments)]

use std::f32::consts::PI;
use nih_plug_vizia::vizia::vg::{Canvas, Color, Paint, Path, Renderer};

use super::palette::{accent_magenta, is_light, silk_muted};
use super::text::draw_text_centered;
use crate::gui::theme::ThemeFonts;

/// Draw a pro-audio 2-position flat paddle switch with 3D ball joint and top SHELF / bottom BELL labels.
pub fn draw_analog_toggle_switch<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    is_shelf: bool,
    fonts: ThemeFonts,
    is_lit: bool,
) {
    let light = is_light();

    // 1. Chassis Mounting Nut Collar (recessed base)
    let mut base_collar = Path::new();
    base_collar.circle(cx, cy, 13.0);
    let (base_fill, base_stroke) = if light {
        (Color::rgb(0xBA, 0xB6, 0xAE), Color::rgb(0x9A, 0x96, 0x8E))
    } else {
        (Color::rgb(0x13, 0x15, 0x1B), Color::rgb(0x25, 0x2A, 0x36))
    };
    canvas.fill_path(&base_collar, &Paint::color(base_fill));
    let mut base_s = Paint::color(base_stroke);
    base_s.set_line_width(1.0);
    canvas.stroke_path(&base_collar, &base_s);

    // 2. Hexagon Mounting Nut (Brushed Nickel)
    let mut hex = Path::new();
    for i in 0..6 {
        let angle = (i as f32 * 60.0 - 90.0) * PI / 180.0;
        let x = cx + 10.5 * angle.cos();
        let y = cy + 10.5 * angle.sin();
        if i == 0 {
            hex.move_to(x, y);
        } else {
            hex.line_to(x, y);
        }
    }
    hex.close();
    let hex_grad = if light {
        Paint::linear_gradient(
            cx - 10.0,
            cy - 10.0,
            cx + 10.0,
            cy + 10.0,
            Color::rgb(0xF2, 0xF4, 0xF8),
            Color::rgb(0xAA, 0xB2, 0xC0),
        )
    } else {
        Paint::linear_gradient(
            cx - 10.0,
            cy - 10.0,
            cx + 10.0,
            cy + 10.0,
            Color::rgb(0x6E, 0x76, 0x88),
            Color::rgb(0x16, 0x19, 0x20),
        )
    };
    canvas.fill_path(&hex, &hex_grad);
    let mut hex_s = Paint::color(if light {
        Color::rgb(0x72, 0x7A, 0x88)
    } else {
        Color::rgb(0x0E, 0x10, 0x15)
    });
    hex_s.set_line_width(1.0);
    canvas.stroke_path(&hex, &hex_s);

    // Inner bevel rim highlight on hex nut
    let mut hex_inner = Path::new();
    for i in 0..6 {
        let angle = (i as f32 * 60.0 - 90.0) * PI / 180.0;
        let x = cx + 9.5 * angle.cos();
        let y = cy + 9.5 * angle.sin();
        if i == 0 {
            hex_inner.move_to(x, y);
        } else {
            hex_inner.line_to(x, y);
        }
    }
    hex_inner.close();
    let mut hex_in_s = Paint::color(if light {
        Color::rgba(0xFF, 0xFF, 0xFF, 180)
    } else {
        Color::rgba(0xFF, 0xFF, 0xFF, 46)
    });
    hex_in_s.set_line_width(0.6);
    canvas.stroke_path(&hex_inner, &hex_in_s);

    // 3. Threaded Bushing Collar
    let mut bushing = Path::new();
    bushing.circle(cx, cy, 6.2);
    let bushing_grad = if light {
        Paint::linear_gradient(
            cx - 6.2,
            cy,
            cx + 6.2,
            cy,
            Color::rgb(0x9E, 0xA6, 0xB8),
            Color::rgb(0xDC, 0xE2, 0xEE),
        )
    } else {
        Paint::linear_gradient(
            cx - 6.2,
            cy,
            cx + 6.2,
            cy,
            Color::rgb(0x24, 0x29, 0x34),
            Color::rgb(0x72, 0x7D, 0x94),
        )
    };
    canvas.fill_path(&bushing, &bushing_grad);
    let mut bushing_s = Paint::color(if light {
        Color::rgb(0x70, 0x78, 0x88)
    } else {
        Color::rgb(0x11, 0x13, 0x18)
    });
    bushing_s.set_line_width(0.8);
    canvas.stroke_path(&bushing, &bushing_s);

    // 4. Socket Aperture
    let mut socket = Path::new();
    socket.circle(cx, cy, 4.8);
    canvas.fill_path(&socket, &Paint::color(if light {
        Color::rgb(0x32, 0x36, 0x42)
    } else {
        Color::rgb(0x07, 0x08, 0x0B)
    }));

    // 5. 3D Polished Ball Joint
    let mut ball = Path::new();
    ball.circle(cx, cy, 4.2);
    let ball_grad = if light {
        Paint::radial_gradient(
            cx - 0.7,
            cy - 0.9,
            0.5,
            4.2,
            Color::rgb(0xFF, 0xFF, 0xFF),
            Color::rgb(0x6E, 0x76, 0x86),
        )
    } else {
        Paint::radial_gradient(
            cx - 0.7,
            cy - 0.9,
            0.5,
            4.2,
            Color::rgb(0xFF, 0xFF, 0xFF),
            Color::rgb(0x16, 0x18, 0x22),
        )
    };
    canvas.fill_path(&ball, &ball_grad);
    let mut ball_s = Paint::color(if light {
        Color::rgb(0x54, 0x5C, 0x6A)
    } else {
        Color::rgb(0x15, 0x17, 0x20)
    });
    ball_s.set_line_width(0.5);
    canvas.stroke_path(&ball, &ball_s);

    // 6. Paddle (Skeuomorphic Pro-Audio Flatted Paddle)
    if is_shelf {
        // --- UP (+45° Tilted Throw) -> SHELF ---
        // Neck
        let mut neck = Path::new();
        neck.rect(cx - 1.4, cy - 3.0, 2.8, 2.0);
        canvas.fill_path(&neck, &Paint::color(if light {
            Color::rgb(0x96, 0xA0, 0xB2)
        } else {
            Color::rgb(0x58, 0x62, 0x76)
        }));

        // Trapezoidal Blade (foreshortened 45° projection)
        let mut blade = Path::new();
        blade.move_to(cx - 1.4, cy - 3.0);
        blade.line_to(cx - 4.6, cy - 17.0);
        blade.line_to(cx + 4.6, cy - 17.0);
        blade.line_to(cx + 1.4, cy - 3.0);
        blade.close();
        let blade_grad = if light {
            Paint::linear_gradient(
                cx - 4.6,
                cy - 17.0,
                cx + 4.6,
                cy - 17.0,
                Color::rgb(0x92, 0x9E, 0xB2),
                Color::rgb(0xE8, 0xEE, 0xF8),
            )
        } else {
            Paint::linear_gradient(
                cx - 4.6,
                cy - 17.0,
                cx + 4.6,
                cy - 17.0,
                Color::rgb(0x36, 0x3C, 0x4A),
                Color::rgb(0x9E, 0xAB, 0xC4),
            )
        };
        canvas.fill_path(&blade, &blade_grad);
        let mut blade_s = Paint::color(if light {
            Color::rgb(0x54, 0x5C, 0x6C)
        } else {
            Color::rgb(0x16, 0x18, 0x22)
        });
        blade_s.set_line_width(0.75);
        canvas.stroke_path(&blade, &blade_s);

        // 45° Angle Top Cut Facet Cap
        let mut facet = Path::new();
        facet.move_to(cx - 4.6, cy - 17.0);
        facet.line_to(cx - 4.0, cy - 18.8);
        facet.line_to(cx + 4.0, cy - 18.8);
        facet.line_to(cx + 4.6, cy - 17.0);
        facet.close();
        let facet_grad = if light {
            Paint::linear_gradient(
                cx,
                cy - 18.8,
                cx,
                cy - 17.0,
                Color::rgb(0xFF, 0xFF, 0xFF),
                Color::rgb(0xB8, 0xC4, 0xD6),
            )
        } else {
            Paint::linear_gradient(
                cx,
                cy - 18.8,
                cx,
                cy - 17.0,
                Color::rgb(0xE2, 0xED, 0xFC),
                Color::rgb(0x8E, 0x9A, 0xB4),
            )
        };
        canvas.fill_path(&facet, &facet_grad);
        let mut facet_s = Paint::color(if light {
            Color::rgb(0x54, 0x5C, 0x6C)
        } else {
            Color::rgb(0x15, 0x17, 0x20)
        });
        facet_s.set_line_width(0.6);
        canvas.stroke_path(&facet, &facet_s);

        // Specular top line highlight
        let mut top_line = Path::new();
        top_line.move_to(cx - 3.8, cy - 18.3);
        top_line.line_to(cx + 3.8, cy - 18.3);
        let mut top_lp = Paint::color(Color::rgba(0xFF, 0xFF, 0xFF, if light { 240 } else { 190 }));
        top_lp.set_line_width(0.7);
        canvas.stroke_path(&top_line, &top_lp);

        // Side chamfer accents
        let mut l_chamfer = Path::new();
        l_chamfer.move_to(cx - 4.6, cy - 17.0);
        l_chamfer.line_to(cx - 1.4, cy - 3.0);
        let mut l_cp = Paint::color(Color::rgba(0xFF, 0xFF, 0xFF, if light { 120 } else { 56 }));
        l_cp.set_line_width(0.6);
        canvas.stroke_path(&l_chamfer, &l_cp);

        let mut r_chamfer = Path::new();
        r_chamfer.move_to(cx + 4.6, cy - 17.0);
        r_chamfer.line_to(cx + 1.4, cy - 3.0);
        let mut r_cp = Paint::color(Color::rgba(0x00, 0x00, 0x00, if light { 90 } else { 128 }));
        r_cp.set_line_width(0.7);
        canvas.stroke_path(&r_chamfer, &r_cp);
    } else {
        // --- DOWN (-45° Tilted Throw) -> BELL ---
        // Neck
        let mut neck = Path::new();
        neck.rect(cx - 1.4, cy + 1.0, 2.8, 2.0);
        canvas.fill_path(&neck, &Paint::color(if light {
            Color::rgb(0x88, 0x92, 0xA4)
        } else {
            Color::rgb(0x44, 0x4C, 0x5C)
        }));

        // Trapezoidal Blade (mirrored across cy)
        let mut blade = Path::new();
        blade.move_to(cx - 1.4, cy + 3.0);
        blade.line_to(cx - 4.6, cy + 17.0);
        blade.line_to(cx + 4.6, cy + 17.0);
        blade.line_to(cx + 1.4, cy + 3.0);
        blade.close();
        let blade_grad = if light {
            Paint::linear_gradient(
                cx - 4.6,
                cy + 17.0,
                cx + 4.6,
                cy + 17.0,
                Color::rgb(0x80, 0x8C, 0xA0),
                Color::rgb(0xDC, 0xE4, 0xF0),
            )
        } else {
            Paint::linear_gradient(
                cx - 4.6,
                cy + 17.0,
                cx + 4.6,
                cy + 17.0,
                Color::rgb(0x24, 0x28, 0x34),
                Color::rgb(0x72, 0x7E, 0x96),
            )
        };
        canvas.fill_path(&blade, &blade_grad);
        let mut blade_s = Paint::color(if light {
            Color::rgb(0x54, 0x5C, 0x6C)
        } else {
            Color::rgb(0x16, 0x18, 0x22)
        });
        blade_s.set_line_width(0.75);
        canvas.stroke_path(&blade, &blade_s);

        // 45° Angle Bottom Cut Facet Cap (mirrored)
        let mut facet = Path::new();
        facet.move_to(cx - 4.6, cy + 17.0);
        facet.line_to(cx - 4.0, cy + 18.8);
        facet.line_to(cx + 4.0, cy + 18.8);
        facet.line_to(cx + 4.6, cy + 17.0);
        facet.close();
        let facet_grad = if light {
            Paint::linear_gradient(
                cx,
                cy + 17.0,
                cx,
                cy + 18.8,
                Color::rgb(0x80, 0x88, 0x98),
                Color::rgb(0x50, 0x58, 0x66),
            )
        } else {
            Paint::linear_gradient(
                cx,
                cy + 17.0,
                cx,
                cy + 18.8,
                Color::rgb(0x2A, 0x2F, 0x3C),
                Color::rgb(0x10, 0x12, 0x18),
            )
        };
        canvas.fill_path(&facet, &facet_grad);
        let mut facet_s = Paint::color(if light {
            Color::rgb(0x40, 0x46, 0x52)
        } else {
            Color::rgb(0x15, 0x17, 0x20)
        });
        facet_s.set_line_width(0.6);
        canvas.stroke_path(&facet, &facet_s);

        // Ambient highlight on top neck transition
        let mut neck_hi = Path::new();
        neck_hi.move_to(cx - 1.3, cy + 3.5);
        neck_hi.line_to(cx + 1.3, cy + 3.5);
        let mut neck_hp = Paint::color(Color::rgba(0xFF, 0xFF, 0xFF, if light { 160 } else { 102 }));
        neck_hp.set_line_width(0.7);
        canvas.stroke_path(&neck_hi, &neck_hp);

        // Side chamfers
        let mut l_chamfer = Path::new();
        l_chamfer.move_to(cx - 1.4, cy + 3.0);
        l_chamfer.line_to(cx - 4.6, cy + 17.0);
        let mut l_cp = Paint::color(Color::rgba(0xFF, 0xFF, 0xFF, if light { 100 } else { 46 }));
        l_cp.set_line_width(0.6);
        canvas.stroke_path(&l_chamfer, &l_cp);

        let mut r_chamfer = Path::new();
        r_chamfer.move_to(cx + 1.4, cy + 3.0);
        r_chamfer.line_to(cx + 4.6, cy + 17.0);
        let mut r_cp = Paint::color(Color::rgba(0x00, 0x00, 0x00, if light { 100 } else { 140 }));
        r_cp.set_line_width(0.7);
        canvas.stroke_path(&r_chamfer, &r_cp);
    }

    // 7. Snug Top/Bottom Labels: SHELF on top, BELL on bottom
    let active_col = accent_magenta();
    let muted_col = silk_muted();

    let shelf_col = if is_shelf && is_lit { active_col } else { muted_col };
    let bell_col = if !is_shelf && is_lit { active_col } else { muted_col };

    // SHELF (Top, centered)
    draw_text_centered(
        canvas,
        cx,
        cy - 29.5,
        "SHELF",
        fonts.bold_paint(shelf_col, 8.0),
    );

    // BELL (Bottom, centered)
    draw_text_centered(
        canvas,
        cx,
        cy + 36.0,
        "BELL",
        fonts.bold_paint(bell_col, 8.0),
    );
}
