//! Push-button and selector primitives. The button lens has three states (off, continuous
//! backlight, smashed), all drawn on the same glass base so the geometry never shifts.

#![allow(clippy::too_many_arguments)]

use nih_plug_vizia::vizia::vg::{Canvas, Color, LineCap, LineJoin, Paint, Path, Renderer};

use super::dseg::draw_dseg_display;
use super::palette::{accent_magenta, is_light, silk_muted, silk_white};
use super::text::draw_text_centered;
use crate::gui::theme::ThemeFonts;

pub fn draw_vintage_bypass_button<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    is_on: bool,
    fonts: ThemeFonts,
) {
    let light = is_light();
    let mut outer = Path::new();
    outer.rounded_rect(cx - 32.0, cy - 13.0, 64.0, 26.0, 6.0);
    let (outer_fill, outer_stroke) = if light {
        (Color::rgb(0xB8, 0xB5, 0xAE), Color::rgb(0x96, 0x93, 0x8C))
    } else {
        (Color::rgb(0x0B, 0x0B, 0x0D), Color::rgb(0x26, 0x26, 0x2C))
    };
    canvas.fill_path(&outer, &Paint::color(outer_fill));
    let mut outer_s = Paint::color(outer_stroke);
    outer_s.set_line_width(1.2);
    canvas.stroke_path(&outer, &outer_s);

    let mut inner = Path::new();
    inner.rounded_rect(cx - 29.0, cy - 10.0, 58.0, 20.0, 4.0);
    let inner_grad = if light {
        Paint::linear_gradient(
            cx,
            cy - 10.0,
            cx,
            cy + 10.0,
            Color::rgb(0xFD, 0xFD, 0xFD),
            Color::rgb(0xD8, 0xD8, 0xDE),
        )
    } else {
        Paint::linear_gradient(
            cx,
            cy - 10.0,
            cx,
            cy + 10.0,
            Color::rgb(0x3A, 0x3A, 0x40),
            Color::rgb(0x20, 0x20, 0x24),
        )
    };
    canvas.fill_path(&inner, &inner_grad);
    let mut inner_s = Paint::color(if light {
        Color::rgb(0xB4, 0xB4, 0xBC)
    } else {
        Color::rgb(0x18, 0x18, 0x1C)
    });
    inner_s.set_line_width(0.8);
    canvas.stroke_path(&inner, &inner_s);

    let mut dot = Path::new();
    dot.circle(cx - 14.0, cy, 3.5);
    if is_on {
        let mut glow = Path::new();
        glow.circle(cx - 14.0, cy, 5.5);
        canvas.fill_path(&glow, &Paint::color(Color::rgba(0xFF, 0x00, 0x7F, 95)));
        canvas.fill_path(&dot, &Paint::color(accent_magenta()));
    } else {
        canvas.fill_path(&dot, &Paint::color(if light {
            Color::rgba(0x88, 0x88, 0x90, 160)
        } else {
            Color::rgba(0x3A, 0x3A, 0x42, 155)
        }));
    }

    let txt_col = if is_on {
        silk_white()
    } else {
        if light {
            Color::rgba(0x55, 0x57, 0x62, 160)
        } else {
            Color::rgba(0x8A, 0x8A, 0x92, 140)
        }
    };
    draw_text_centered(canvas, cx + 7.0, cy, label, fonts.bold_paint(txt_col, 10.0));
}

/// Backlight level of a push-button lens.
#[derive(Clone, Copy, Debug)]
enum Lens {
    /// Backlight off.
    Off,
    /// Backlight at a continuous 0..1 level, so the same button can be faded.
    Lit(f32),
    /// "Smashed" overdriven core.
    Smashed,
}

#[inline]
fn blend_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

#[inline]
fn blend_rgba(a: (u8, u8, u8, u8), b: (u8, u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgba(
        blend_u8(a.0, b.0, t),
        blend_u8(a.1, b.1, t),
        blend_u8(a.2, b.2, t),
        blend_u8(a.3, b.3, t),
    )
}

pub fn draw_vintage_push_button<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    state: i32,
    width: f32,
    height: f32,
    fonts: ThemeFonts,
) {
    let lens = match state {
        2 => Lens::Smashed,
        1 => Lens::Lit(1.0),
        _ => Lens::Off,
    };
    draw_push_button(canvas, cx, cy, label, lens, width, height, fonts);
}

/// The same button with a continuously variable backlight, for controls whose lamp is itself the
/// status readout: it blinks while a measurement runs and breathes once the result is frozen and
/// in use. Fading the real button (rather than compositing an overlay) keeps the lens, its rim,
/// the engraved legend and the glass glare in exactly the same place at every level.
pub fn draw_vintage_push_button_fade<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    lit: f32,
    width: f32,
    height: f32,
    fonts: ThemeFonts,
) {
    draw_push_button(
        canvas,
        cx,
        cy,
        label,
        Lens::Lit(lit.clamp(0.0, 1.0)),
        width,
        height,
        fonts,
    );
}

fn draw_push_button<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    lens: Lens,
    width: f32,
    height: f32,
    fonts: ThemeFonts,
) {
    let w = width;
    let h = height;
    let x = cx - w * 0.5;
    let y = cy - h * 0.5;
    let lx = x + 3.0;
    let ly = y + 3.0;
    let lw = w - 6.0;
    let lh = h - 6.0;
    let rx_edge = lx + lw;
    let by_edge = ly + lh;

    let is_square = (w - h).abs() < 2.0 && w <= 44.0;
    let (outer_rx, trench_rx, lens_rx, bevel_rx) = if is_square {
        (4.0, 3.2, 2.5, 2.0)
    } else {
        (8.0, 6.5, 5.0, 4.0)
    };

    let light = is_light();

    // 1. Outer Machined Frame
    let mut outer = Path::new();
    outer.rounded_rect(x, y, w, h, outer_rx);
    let (outer_fill, outer_stroke) = if light {
        (Color::rgb(0xD0, 0xCD, 0xC6), Color::rgb(0xB5, 0xB2, 0xAA))
    } else {
        (Color::rgb(0x16, 0x16, 0x1B), Color::rgb(0x2C, 0x2C, 0x34))
    };
    canvas.fill_path(&outer, &Paint::color(outer_fill));
    let mut outer_s = Paint::color(outer_stroke);
    outer_s.set_line_width(1.0);
    canvas.stroke_path(&outer, &outer_s);

    // 2. Deep Recessed Bezel Trench
    let mut trench = Path::new();
    trench.rounded_rect(x + 1.5, y + 1.5, w - 3.0, h - 3.0, trench_rx);
    let (tr_fill, tr_stroke) = if light {
        (Color::rgb(0xB8, 0xB5, 0xAE), Color::rgb(0x96, 0x93, 0x8C))
    } else {
        (Color::rgb(0x07, 0x07, 0x09), Color::rgb(0x00, 0x00, 0x00))
    };
    canvas.fill_path(&trench, &Paint::color(tr_fill));
    let mut trench_s = Paint::color(tr_stroke);
    trench_s.set_line_width(1.2);
    canvas.stroke_path(&trench, &trench_s);

    // 3. Smoked Tinted Glass Cap / Lens. Every state sits on the same glass base, so dimming the
    // backlight never changes the shape or position of the lens.
    let mut lens_path = Path::new();
    lens_path.rounded_rect(lx, ly, lw, lh, lens_rx);
    let base = if light {
        Paint::linear_gradient(
            lx,
            ly,
            lx,
            ly + lh,
            Color::rgb(0xEB, 0xD8, 0xE6),
            Color::rgb(0xDC, 0xC6, 0xDA),
        )
    } else {
        Paint::linear_gradient(
            lx,
            ly,
            lx,
            ly + lh,
            Color::rgb(0x21, 0x07, 0x15),
            Color::rgb(0x0B, 0x02, 0x07),
        )
    };
    canvas.fill_path(&lens_path, &base);
    match lens {
        Lens::Smashed => {
            // Smashed Overdriven Core
            let grad = Paint::radial_gradient(
                cx + 2.0,
                cy - 2.0,
                2.0,
                lw * 0.72,
                Color::rgb(0xFF, 0x99, 0xCC),
                Color::rgb(0x38, 0x03, 0x1E),
            );
            canvas.fill_path(&lens_path, &grad);
            let mut ls = Paint::color(Color::rgb(0xFF, 0xB3, 0xD9));
            ls.set_line_width(1.35);
            canvas.stroke_path(&lens_path, &ls);
        }
        Lens::Lit(level) if level > 0.004 => {
            // Backlit magenta lens, faded in over the smoked glass.
            let a = (level * 255.0).round() as u8;
            let grad = if light {
                Paint::radial_gradient(
                    cx,
                    cy - 2.0,
                    2.0,
                    lw * 0.75,
                    Color::rgba(0xFF, 0x33, 0x99, a),
                    Color::rgba(0xD8, 0x0A, 0x6E, a),
                )
            } else {
                Paint::radial_gradient(
                    cx,
                    cy - 2.0,
                    2.0,
                    lw * 0.68,
                    Color::rgba(0x7A, 0x0C, 0x43, a),
                    Color::rgba(0x1E, 0x03, 0x10, a),
                )
            };
            canvas.fill_path(&lens_path, &grad);
            let mut ls = Paint::color(if light {
                Color::rgba(
                    0xFF,
                    0x80,
                    0xBF,
                    (245.0 * level).round().clamp(0.0, 255.0) as u8,
                )
            } else {
                Color::rgba(
                    0xFF,
                    0x4D,
                    0xA6,
                    (245.0 * level).round().clamp(0.0, 255.0) as u8,
                )
            });
            ls.set_line_width(1.2);
            canvas.stroke_path(&lens_path, &ls);
        }
        _ => {
            // Unlit Smoked Glass Lens
            let mut ls = Paint::color(if light {
                Color::rgba(0xFF, 0x00, 0x7F, 80)
            } else {
                Color::rgba(0xFF, 0x24, 0x91, 158)
            });
            ls.set_line_width(1.05);
            canvas.stroke_path(&lens_path, &ls);
        }
    }

    // 3b. Illuminated 3D Glass Bevel Rim (4.1px inset catches edge light in all states)
    let mut bevel = Path::new();
    bevel.rounded_rect(x + 4.1, y + 4.1, w - 8.2, h - 8.2, bevel_rx);
    let (bev_col, bev_w) = match lens {
        Lens::Smashed => (Color::rgba(0xFF, 0xE6, 0xF2, 255), 1.5),
        Lens::Lit(level) => (
            if light {
                blend_rgba((0xFF, 0x80, 0xBF, 180), (0xFF, 0xCC, 0xE6, 255), level)
            } else {
                blend_rgba((0xFF, 0x4D, 0xA6, 155), (0xFF, 0x80, 0xBF, 240), level)
            },
            1.15 + 0.2 * level,
        ),
        Lens::Off => (
            if light {
                Color::rgba(0xFF, 0x4D, 0xA6, 120)
            } else {
                Color::rgba(0xFF, 0x4D, 0xA6, 155)
            },
            1.15,
        ),
    };
    let mut bev_s = Paint::color(bev_col);
    bev_s.set_line_width(bev_w);
    canvas.stroke_path(&bevel, &bev_s);

    // 4. Engraved Backlit Legend (Text, Book Glyph, Sun Glyph, Moon Glyph)
    let unlit_txt = if light {
        (0x5A, 0x40, 0x50, 220)
    } else {
        (0xD4, 0x78, 0xA8, 214)
    };
    let txt_col = match lens {
        Lens::Smashed => Color::rgb(0xFF, 0xF0, 0xF7),
        Lens::Lit(level) => if light {
            blend_rgba(unlit_txt, (0xFF, 0xFF, 0xFF, 255), level)
        } else {
            blend_rgba(unlit_txt, (0xFF, 0x66, 0xB2, 255), level)
        },
        Lens::Off => Color::rgba(unlit_txt.0, unlit_txt.1, unlit_txt.2, unlit_txt.3),
    };
    if label == "BOOK" || label == "MANUAL_ICON" {
        // Backlit vector open-book glyph
        let mut book = Path::new();
        // Central spine fold
        book.move_to(cx, cy - 4.5);
        book.line_to(cx, cy + 5.5);
        // Left page (curved top, outer edge, curved bottom)
        book.move_to(cx, cy - 4.5);
        book.bezier_to(cx - 3.0, cy - 6.2, cx - 6.0, cy - 5.5, cx - 8.0, cy - 4.2);
        book.line_to(cx - 8.0, cy + 4.2);
        book.bezier_to(cx - 6.0, cy + 3.0, cx - 3.0, cy + 3.8, cx, cy + 5.5);
        // Right page (curved top, outer edge, curved bottom)
        book.move_to(cx, cy - 4.5);
        book.bezier_to(cx + 3.0, cy - 6.2, cx + 6.0, cy - 5.5, cx + 8.0, cy - 4.2);
        book.line_to(cx + 8.0, cy + 4.2);
        book.bezier_to(cx + 6.0, cy + 3.0, cx + 3.0, cy + 3.8, cx, cy + 5.5);
        // Page text lines
        book.move_to(cx - 6.2, cy - 1.2);
        book.line_to(cx - 2.0, cy - 0.7);
        book.move_to(cx - 6.2, cy + 1.3);
        book.line_to(cx - 2.0, cy + 1.8);
        book.move_to(cx + 2.0, cy - 0.7);
        book.line_to(cx + 6.2, cy - 1.2);
        book.move_to(cx + 2.0, cy + 1.8);
        book.line_to(cx + 6.2, cy + 1.3);

        let mut bp = Paint::color(txt_col);
        bp.set_line_width(1.2);
        bp.set_line_cap(LineCap::Round);
        canvas.stroke_path(&book, &bp);
    } else if label == "SUN" {
        // Backlit vector sun glyph: central disc + 8 radial ray spokes
        let mut sun = Path::new();
        sun.circle(cx, cy, 3.8);
        canvas.fill_path(&sun, &Paint::color(txt_col));

        let mut rays = Path::new();
        for i in 0..8 {
            let angle = (i as f32) * (std::f32::consts::PI / 4.0);
            let x1 = cx + 5.5 * angle.cos();
            let y1 = cy + 5.5 * angle.sin();
            let x2 = cx + 7.8 * angle.cos();
            let y2 = cy + 7.8 * angle.sin();
            rays.move_to(x1, y1);
            rays.line_to(x2, y2);
        }
        let mut ray_p = Paint::color(txt_col);
        ray_p.set_line_width(1.3);
        ray_p.set_line_cap(LineCap::Round);
        canvas.stroke_path(&rays, &ray_p);
    } else if label == "MOON" {
        // Backlit vector crescent moon glyph
        let mut moon = Path::new();
        moon.move_to(cx + 1.2, cy - 6.5);
        moon.bezier_to(cx - 6.5, cy - 5.5, cx - 6.5, cy + 5.5, cx + 1.2, cy + 6.5);
        moon.bezier_to(cx - 2.5, cy + 4.0, cx - 2.5, cy - 4.0, cx + 1.2, cy - 6.5);
        moon.close();
        canvas.fill_path(&moon, &Paint::color(txt_col));
    } else {
        draw_text_centered(canvas, cx, cy + 0.5, label, fonts.bold_paint(txt_col, 9.5));
    }

    // 5. Optical Glass Top Glare Reflection
    let glare_alpha = match lens {
        Lens::Smashed => 8,
        Lens::Lit(level) => blend_u8(18, 23, level),
        Lens::Off => 18,
    };
    let mut glare = Path::new();
    glare.rounded_rect(x + 4.5, y + 4.5, w - 9.0, 7.0, 3.2);
    canvas.fill_path(
        &glare,
        &Paint::color(Color::rgba(255, 255, 255, glare_alpha)),
    );

    // 6. Shattered Tempered-Glass Overlay (State 2: "smashed" / Extreme Drive)
    if matches!(lens, Lens::Smashed) {
        let ix = cx + 3.5;
        let iy = cy - 2.5;

        canvas.save();
        canvas.scissor(lx, ly, lw, lh);

        // Tilted glass shard refraction facets
        let mut f1 = Path::new();
        f1.move_to(ix, iy);
        f1.line_to(lx + 6.0, ly);
        f1.line_to(lx + 22.0, ly);
        f1.close();
        canvas.fill_path(&f1, &Paint::color(Color::rgba(255, 255, 255, 36)));

        let mut f2 = Path::new();
        f2.move_to(ix, iy);
        f2.line_to(rx_edge, ly + 6.0);
        f2.line_to(rx_edge, ly + 24.0);
        f2.close();
        canvas.fill_path(&f2, &Paint::color(Color::rgba(0, 0, 0, 62)));

        // Concentric spiderweb stress rings around impact point
        let mut ring1 = Path::new();
        ring1.move_to(ix - 4.2, iy - 2.0);
        ring1.line_to(ix - 0.8, iy - 4.8);
        ring1.line_to(ix + 4.0, iy - 2.6);
        ring1.line_to(ix + 4.8, iy + 2.2);
        ring1.line_to(ix + 0.5, iy + 5.0);
        ring1.line_to(ix - 4.5, iy + 2.4);
        ring1.close();
        let mut r1s = Paint::color(Color::rgba(0xFF, 0xE6, 0xF2, 215));
        r1s.set_line_width(0.7);
        canvas.stroke_path(&ring1, &r1s);

        let mut ring2 = Path::new();
        ring2.move_to(ix - 9.5, iy - 4.5);
        ring2.line_to(ix - 2.0, iy - 10.2);
        ring2.line_to(ix + 8.5, iy - 6.0);
        ring2.line_to(ix + 10.2, iy + 4.8);
        ring2.line_to(ix + 2.2, iy + 11.2);
        ring2.line_to(ix - 8.8, iy + 6.5);
        ring2.close();
        let mut r2s = Paint::color(Color::rgba(0xFF, 0x99, 0xCC, 185));
        r2s.set_line_width(0.6);
        canvas.stroke_path(&ring2, &r2s);

        // Radial fracture lines
        let rays: [([f32; 2], [f32; 2]); 8] = [
            ([ix - 7.0, iy - 8.0], [lx + 5.0, ly]),
            ([ix + 4.0, iy - 9.0], [rx_edge - 7.0, ly]),
            ([ix + 9.0, iy - 2.0], [rx_edge, ly + 9.0]),
            ([ix + 8.0, iy + 7.0], [rx_edge, by_edge - 6.0]),
            ([ix + 1.0, iy + 10.0], [ix + 4.0, by_edge]),
            ([ix - 7.0, iy + 8.0], [lx + 6.0, by_edge]),
            ([ix - 11.0, iy + 2.0], [lx, cy + 4.0]),
            ([ix - 10.0, iy - 3.0], [lx, ly + 10.0]),
        ];

        let mut cracks_sh = Path::new();
        let mut cracks_hi = Path::new();
        for (mid, end) in rays {
            cracks_sh.move_to(ix + 0.4, iy + 0.5);
            cracks_sh.line_to(mid[0] + 0.4, mid[1] + 0.5);
            cracks_sh.line_to(end[0] + 0.4, end[1] + 0.5);

            cracks_hi.move_to(ix, iy);
            cracks_hi.line_to(mid[0], mid[1]);
            cracks_hi.line_to(end[0], end[1]);
        }

        let mut c_sh = Paint::color(Color::rgba(0x14, 0x00, 0x0A, 210));
        c_sh.set_line_width(1.35);
        c_sh.set_line_cap(LineCap::Round);
        c_sh.set_line_join(LineJoin::Round);
        canvas.stroke_path(&cracks_sh, &c_sh);

        let mut c_hi = Paint::color(Color::rgba(0xFF, 0xF0, 0xF8, 242));
        c_hi.set_line_width(0.8);
        c_hi.set_line_cap(LineCap::Round);
        c_hi.set_line_join(LineJoin::Round);
        canvas.stroke_path(&cracks_hi, &c_hi);

        let mut core = Path::new();
        core.circle(ix, iy, 1.6);
        canvas.fill_path(&core, &Paint::color(Color::rgba(255, 255, 255, 242)));

        canvas.restore();
    }
}

pub fn draw_segmented_selector<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    alias_text: &str,
    fonts: ThemeFonts,
) {
    let disp_w = 122.0;
    let disp_h = 32.0;
    let btn_w = 24.0;
    let btn_h = 14.5;
    let gap = 5.0;
    let total_w = disp_w + gap + btn_w;

    let left_x = cx - total_w * 0.5;
    let disp_cx = left_x + disp_w * 0.5;
    let disp_y = cy - disp_h * 0.5;
    let btn_x = left_x + disp_w + gap;

    if !label.is_empty() {
        draw_text_centered(
            canvas,
            cx,
            cy - 23.0,
            label,
            fonts.bold_paint(silk_muted(), 10.0),
        );
    }
    draw_dseg_display(
        canvas, disp_cx, cy, disp_w, disp_h, 9, alias_text, 8.0, fonts,
    );

    // Up button [▲]
    let light = is_light();
    let mut b_up = Path::new();
    b_up.rounded_rect(btn_x, disp_y, btn_w, btn_h, 3.5);
    let up_grad = if light {
        Paint::linear_gradient(
            btn_x,
            disp_y,
            btn_x,
            disp_y + btn_h,
            Color::rgb(0xFD, 0xFD, 0xFD),
            Color::rgb(0xD6, 0xD2, 0xCA),
        )
    } else {
        Paint::linear_gradient(
            btn_x,
            disp_y,
            btn_x,
            disp_y + btn_h,
            Color::rgb(0x3A, 0x3A, 0x40),
            Color::rgb(0x20, 0x20, 0x24),
        )
    };
    canvas.fill_path(&b_up, &up_grad);
    let mut b_s = Paint::color(if light {
        Color::rgb(0x9E, 0x9A, 0x92)
    } else {
        Color::rgb(0x2C, 0x2C, 0x34)
    });
    b_s.set_line_width(0.9);
    canvas.stroke_path(&b_up, &b_s);

    let mut t_up = Path::new();
    t_up.move_to(btn_x + btn_w * 0.5 - 3.5, disp_y + btn_h * 0.5 + 2.0);
    t_up.line_to(btn_x + btn_w * 0.5, disp_y + btn_h * 0.5 - 2.5);
    t_up.line_to(btn_x + btn_w * 0.5 + 3.5, disp_y + btn_h * 0.5 + 2.0);
    t_up.close();
    let arrow_col = if light {
        Color::rgb(0x2E, 0x28, 0x2C)
    } else {
        silk_white()
    };
    canvas.fill_path(&t_up, &Paint::color(arrow_col));

    // Down button [▼]
    let dy2 = disp_y + btn_h + 3.0;
    let mut b_dn = Path::new();
    b_dn.rounded_rect(btn_x, dy2, btn_w, btn_h, 3.5);
    let dn_grad = if light {
        Paint::linear_gradient(
            btn_x,
            dy2,
            btn_x,
            dy2 + btn_h,
            Color::rgb(0xFD, 0xFD, 0xFD),
            Color::rgb(0xD6, 0xD2, 0xCA),
        )
    } else {
        Paint::linear_gradient(
            btn_x,
            dy2,
            btn_x,
            dy2 + btn_h,
            Color::rgb(0x3A, 0x3A, 0x40),
            Color::rgb(0x20, 0x20, 0x24),
        )
    };
    canvas.fill_path(&b_dn, &dn_grad);
    canvas.stroke_path(&b_dn, &b_s);

    let mut t_dn = Path::new();
    t_dn.move_to(btn_x + btn_w * 0.5 - 3.5, dy2 + btn_h * 0.5 - 2.0);
    t_dn.line_to(btn_x + btn_w * 0.5, dy2 + btn_h * 0.5 + 2.5);
    t_dn.line_to(btn_x + btn_w * 0.5 + 3.5, dy2 + btn_h * 0.5 - 2.0);
    t_dn.close();
    canvas.fill_path(&t_dn, &Paint::color(arrow_col));
}
